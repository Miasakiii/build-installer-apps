//! 文件系统扫描模块
//!
//! 递归扫描安装目录，收集文件元数据。

use std::path::Path;

use camino::Utf8PathBuf;
use packforge_core::{FileEntry, FileType, SoftwareInfo};
use rayon::prelude::*;

use crate::Result;

/// 扫描报告
#[derive(Debug)]
pub struct ScanReport {
    /// 软件信息
    pub software: SoftwareInfo,
    /// 安装目录
    pub install_dir: Utf8PathBuf,
    /// 扫描到的文件
    pub files: Vec<FileEntry>,
    /// 扫描的文件总数
    pub total_files: usize,
    /// 总大小（字节）
    pub total_size: u64,
    /// 扫描耗时
    pub elapsed: std::time::Duration,
}

/// 扫描选项
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// 是否计算 SHA-256 哈希（大文件会慢）
    pub compute_hashes: bool,
    /// 是否解析 PE 依赖（EXE/DLL 的导入表）
    pub parse_pe_dependencies: bool,
    /// 最大扫描深度（0 = 无限）
    pub max_depth: usize,
    /// 排除的目录名模式
    pub exclude_dirs: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            compute_hashes: false,
            parse_pe_dependencies: true,
            max_depth: 0,
            exclude_dirs: vec![
                ".git".into(),
                "node_modules".into(),
                "__pycache__".into(),
                ".svn".into(),
            ],
        }
    }
}

/// 根据软件名称查找并扫描其安装目录
pub fn scan_software_by_name(name: &str, options: &ScanOptions) -> Result<ScanReport> {
    let start = std::time::Instant::now();

    // 1. 查找软件
    let software = crate::discovery::search_software(name)?
        .into_iter()
        .find(|s| !s.is_system_component)
        .ok_or_else(|| {
            crate::ScannerError::SoftwareNotFound(format!("未找到软件: {}", name))
        })?;

    // 2. 获取安装目录
    let install_dir = software
        .install_dir
        .as_ref()
        .ok_or_else(|| {
            crate::ScannerError::SoftwareNotFound(format!(
                "软件 {} 没有安装路径信息",
                software.name
            ))
        })?
        .clone();

    let install_path = Path::new(install_dir.as_str());

    // 3. 扫描目录
    let files = scan_directory_with_options(install_path, options)?;

    let total_files = files.len();
    let total_size = files.iter().map(|f| f.size).sum();
    let elapsed = start.elapsed();

    Ok(ScanReport {
        software,
        install_dir,
        files,
        total_files,
        total_size,
        elapsed,
    })
}

/// 递归扫描目录下所有文件（带选项）
pub fn scan_directory_with_options(
    path: &Path,
    options: &ScanOptions,
) -> Result<Vec<FileEntry>> {
    if !path.exists() {
        return Err(crate::ScannerError::io(
            std::io::Error::new(std::io::ErrorKind::NotFound, "目录不存在"),
            path,
        ));
    }

    // 1. 收集所有文件路径
    let walker = walkdir::WalkDir::new(path)
        .follow_links(false)
        .max_depth(if options.max_depth == 0 {
            usize::MAX
        } else {
            options.max_depth
        });

    let file_paths: Vec<std::path::PathBuf> = walker
        .into_iter()
        .filter_entry(|e| {
            // 排除指定目录
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy();
                !options.exclude_dirs.iter().any(|ex| ex == dir_name.as_ref())
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    // 2. 并行分析文件
    let entries: Vec<FileEntry> = file_paths
        .par_iter()
        .filter_map(|path| match analyze_file_with_options(path, options) {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::debug!("分析文件失败 {}: {}", path.display(), e);
                None
            }
        })
        .collect();

    Ok(entries)
}

/// 递归扫描目录下所有文件（简单版本）
pub fn scan_directory(path: &Path) -> Result<Vec<FileEntry>> {
    scan_directory_with_options(path, &ScanOptions::default())
}

/// 分析单个文件的元数据（带选项）
pub fn analyze_file_with_options(path: &Path, options: &ScanOptions) -> Result<FileEntry> {
    let metadata = std::fs::metadata(path).map_err(|e| crate::ScannerError::io(e, path))?;

    let size = metadata.len();
    let file_type = classify_by_extension(path);
    let is_executable = matches!(file_type, FileType::Executable | FileType::DynamicLibrary);

    // 可选：计算 SHA-256 哈希
    let hash = if options.compute_hashes {
        compute_hash(path).ok()
    } else {
        None
    };

    // 可选：解析 PE 依赖
    let dependencies = if options.parse_pe_dependencies && is_executable {
        parse_pe_imports_safe(path)
    } else {
        Vec::new()
    };

    Ok(FileEntry {
        path: Utf8PathBuf::try_from(path.to_path_buf())
            .unwrap_or_else(|_| Utf8PathBuf::from(path.to_string_lossy().to_string())),
        size,
        hash,
        file_type,
        is_executable,
        dependencies,
    })
}

/// 分析单个文件的元数据（简单版本）
pub fn analyze_file(path: &Path) -> Result<FileEntry> {
    analyze_file_with_options(path, &ScanOptions::default())
}

/// 基于扩展名的快速文件分类
fn classify_by_extension(path: &Path) -> FileType {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("exe" | "com" | "bat" | "cmd" | "ps1") => FileType::Executable,
        Some("dll" | "ocx" | "ax") => FileType::DynamicLibrary,
        Some("lib" | "a" | "obj") => FileType::StaticLibrary,
        Some("sys" | "drv") => FileType::Driver,
        Some("config" | "ini" | "json" | "xml" | "yaml" | "yml" | "toml") => FileType::ConfigFile,
        Some("log" | "tmp" | "bak" | "cache") => FileType::DataFile,
        Some("txt" | "md" | "html" | "htm" | "pdf" | "chm") => FileType::Documentation,
        Some("plugin" | "addon" | "xpi") => FileType::Plugin,
        _ => FileType::Unknown,
    }
}

/// 计算文件的 SHA-256 哈希
pub fn compute_hash(path: &Path) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};

    let data = std::fs::read(path).map_err(|e| crate::ScannerError::io(e, path))?;
    let hash = Sha256::digest(&data);
    Ok(hash.into())
}

/// 安全解析 PE 导入表（失败返回空列表）
fn parse_pe_imports_safe(path: &Path) -> Vec<String> {
    match parse_pe_imports(path) {
        Ok(deps) => deps,
        Err(e) => {
            tracing::debug!("解析 PE 导入表失败 {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// 解析 PE 文件的导入表
fn parse_pe_imports(path: &Path) -> Result<Vec<String>> {
    let data = std::fs::read(path).map_err(|e| crate::ScannerError::io(e, path))?;

    let pe = goblin::pe::PE::parse(&data)
        .map_err(|e| crate::ScannerError::PeParse(e.to_string()))?;

    let deps: std::collections::HashSet<String> =
        pe.imports.iter().map(|i| i.dll.to_string()).collect();

    Ok(deps.into_iter().collect())
}

/// 格式化文件大小为人类可读格式
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return format!("{:.1} {}", size, unit);
        }
        size /= 1024.0;
    }
    format!("{:.1} PB", size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classify_executable_extensions() {
        assert_eq!(
            classify_by_extension(&PathBuf::from("app.exe")),
            FileType::Executable
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("lib.dll")),
            FileType::DynamicLibrary
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("config.json")),
            FileType::ConfigFile
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("readme.md")),
            FileType::Documentation
        );
        assert_eq!(
            classify_by_extension(&PathBuf::from("data.bin")),
            FileType::Unknown
        );
    }

    #[test]
    fn scan_nonexistent_dir_returns_error() {
        let result = scan_directory(Path::new("Z:\\nonexistent\\path\\12345"));
        assert!(result.is_err());
    }

    #[test]
    fn format_size_works() {
        assert_eq!(format_size(0), "0.0 B");
        assert_eq!(format_size(512), "512.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn scan_temp_dir_returns_entries() {
        let dir = tempfile::tempdir().unwrap();
        // 创建测试文件
        std::fs::write(dir.path().join("test.exe"), b"MZ").unwrap();
        std::fs::write(dir.path().join("config.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn scan_with_hash_computation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.txt"), b"hello world").unwrap();

        let options = ScanOptions {
            compute_hashes: true,
            ..Default::default()
        };
        let files = scan_directory_with_options(dir.path(), &options).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].hash.is_some(), "应该计算了哈希");
    }

    #[test]
    fn scan_respects_exclude_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git/config"), b"").unwrap();
        std::fs::write(dir.path().join("app.exe"), b"MZ").unwrap();

        let options = ScanOptions {
            exclude_dirs: vec![".git".into()],
            ..Default::default()
        };
        let files = scan_directory_with_options(dir.path(), &options).unwrap();
        assert_eq!(files.len(), 1, "应该排除 .git 目录");
    }

    #[cfg(windows)]
    #[test]
    fn scan_real_executable_detects_pe() {
        // 扫描一个真实的 exe 文件（如 notepad.exe）
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let notepad = std::path::PathBuf::from(&system_root).join("notepad.exe");
        if notepad.exists() {
            let result = analyze_file(&notepad);
            assert!(result.is_ok());
            let entry = result.unwrap();
            assert_eq!(entry.file_type, FileType::Executable);
            assert!(entry.size > 0);
            // notepad.exe 应该有一些 DLL 依赖
            assert!(
                !entry.dependencies.is_empty(),
                "notepad.exe 应该有 DLL 依赖"
            );
        }
    }
}
