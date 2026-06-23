//! 便携化打包模块
//!
//! 将分析结果打包为可分发的便携化包。

use std::io::Write;
use std::path::Path;

use camino::Utf8PathBuf;
use packforge_core::{AnalysisResult, ExtractOptions, FileType, PackageManifest};

use crate::Result;

/// 打包报告
#[derive(Debug, Clone)]
pub struct PackageReport {
    pub output_path: Utf8PathBuf,
    pub total_files: u64,
    pub total_size: u64,
    pub warnings: Vec<String>,
    pub main_exe: Option<String>,
}

/// 创建便携化包
///
/// 流程：
/// 1. 创建输出目录结构
/// 2. 复制 Core + Library + Plugin 文件（保留目录结构）
/// 3. 复制默认 Config（排除用户数据）
/// 4. 生成 launcher.bat
/// 5. 生成 manifest.json
/// 6. 可选：打包为 ZIP
pub fn create_portable_package(
    analysis: &AnalysisResult,
    output_dir: &Path,
    options: &ExtractOptions,
) -> Result<PackageReport> {
    let profile = &analysis.software;

    // 1. 创建目录结构
    let package_name = format!(
        "{}-{}-portable",
        profile.name.to_lowercase().replace(' ', "-"),
        profile.version.as_deref().unwrap_or("0.0.0")
    );
    let package_dir = output_dir.join(&package_name);
    let app_dir = package_dir.join("App");
    let data_dir = package_dir.join("Data");

    std::fs::create_dir_all(&app_dir)?;
    std::fs::create_dir_all(&data_dir)?;

    // 2. 找到主 EXE 文件
    let main_exe = find_main_exe(analysis);

    // 3. 复制文件（保留目录结构）
    let mut total_files: u64 = 0;
    let mut total_size: u64 = 0;
    let mut warnings = Vec::new();

    // 收集要复制的文件：核心 + 库 + 插件 + 配置
    let files_to_copy: Vec<_> = analysis
        .core_files
        .iter()
        .chain(analysis.optional_files.iter())
        .chain(analysis.software.files.iter().filter(|f| {
            f.file_type == FileType::ConfigFile || f.file_type == FileType::Documentation
        }))
        .collect();

    for file in &files_to_copy {
        let source = file.path.as_std_path();
        if !source.exists() {
            warnings.push(format!("源文件不存在: {}", file.path));
            continue;
        }

        // 计算相对于安装目录的路径
        let relative_path = compute_relative_path(&profile.install_dir, &file.path);

        // 创建目标路径
        let dest = app_dir.join(&relative_path);

        // 创建父目录
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match std::fs::copy(source, &dest) {
            Ok(bytes) => {
                total_files += 1;
                total_size += bytes;
            }
            Err(e) => {
                warnings.push(format!("复制失败 {}: {}", file.path, e));
            }
        }
    }

    // 4. 生成 launcher.bat
    let launcher = generate_launcher_batch(
        &profile.name,
        main_exe.as_deref().unwrap_or(&profile.name),
    );
    std::fs::write(package_dir.join("launcher.bat"), launcher)?;

    // 5. 生成 launcher.sh（Linux 兼容）
    let launcher_sh = generate_launcher_shell(
        &profile.name,
        main_exe.as_deref().unwrap_or(&profile.name),
    );
    std::fs::write(package_dir.join("launcher.sh"), launcher_sh)?;

    // 6. 生成 manifest.json
    let manifest = PackageManifest {
        name: profile.name.clone(),
        version: profile.version.clone().unwrap_or_else(|| "0.0.0".into()),
        publisher: profile.publisher.clone(),
        description: Some(format!("Portable package of {}", profile.name)),
        license: None,
        icon: None,
        entries: Vec::new(),
        registry: Vec::new(),
        install_actions: Vec::new(),
        uninstall_actions: Vec::new(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(package_dir.join("manifest.json"), manifest_json)?;

    // 7. 生成 README.md
    let readme = generate_readme(&profile.name, profile.version.as_deref(), &main_exe);
    std::fs::write(package_dir.join("README.md"), readme)?;

    // 8. 可选 ZIP 打包
    if options.create_zip {
        let zip_path = output_dir.join(format!("{}.zip", package_name));
        match create_zip_archive(&package_dir, &zip_path) {
            Ok(_) => {
                // ZIP 创建成功，可以选择删除临时目录
                // 这里保留临时目录，让用户决定
            }
            Err(e) => {
                warnings.push(format!("ZIP 打包失败: {}", e));
            }
        }
    }

    Ok(PackageReport {
        output_path: Utf8PathBuf::try_from(package_dir)
            .unwrap_or_else(|p| Utf8PathBuf::from(p.to_string())),
        total_files,
        total_size,
        warnings,
        main_exe,
    })
}

/// 计算相对于安装目录的路径
fn compute_relative_path(install_dir: &Utf8PathBuf, file_path: &Utf8PathBuf) -> String {
    let install_str = install_dir.as_str().trim_end_matches('\\').trim_end_matches('/');
    let file_str = file_path.as_str();

    if let Some(relative) = file_str.strip_prefix(install_str) {
        relative
            .trim_start_matches('\\')
            .trim_start_matches('/')
            .to_string()
    } else {
        // 如果不在安装目录下，只保留文件名
        file_path
            .file_name()
            .unwrap_or("unknown")
            .to_string()
    }
}

/// 找到主 EXE 文件
fn find_main_exe(analysis: &AnalysisResult) -> Option<String> {
    // 优先找根目录下的 EXE（非卸载程序）
    let _install_dir = &analysis.software.install_dir;

    analysis
        .core_files
        .iter()
        .filter(|f| f.file_type == FileType::Executable)
        .filter(|f| {
            let path_lower = f.path.as_str().to_lowercase();
            !path_lower.contains("unins")
                && !path_lower.contains("uninstall")
                && !path_lower.contains("setup")
        })
        .min_by_key(|f| {
            // 优先选择在根目录下的 EXE
            
            f.path.as_str().matches('\\').count() + f.path.as_str().matches('/').count()
        })
        .map(|f| {
            f.path
                .file_name()
                .unwrap_or("unknown.exe")
                .to_string()
        })
}

/// 生成 launcher.bat 启动脚本
fn generate_launcher_batch(app_name: &str, main_exe: &str) -> String {
    format!(
        r#"@echo off
setlocal
set "APP_DIR=%~dp0"
set "PATH=%APP_DIR%App;%PATH%"
set "APPDATA=%APP_DIR%Data"
set "LOCALAPPDATA=%APP_DIR%Data"
set "HOME=%APP_DIR%Data"

:: 启动 {app_name}
echo Starting {app_name}...
start "" "%APP_DIR%App\{main_exe}" %*
"#,
        app_name = app_name,
        main_exe = main_exe,
    )
}

/// 生成 launcher.sh 启动脚本（Linux 兼容）
fn generate_launcher_shell(app_name: &str, main_exe: &str) -> String {
    format!(
        r#"#!/bin/bash
# {app_name} 便携化启动脚本

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
export PATH="$SCRIPT_DIR/App:$PATH"
export APPDATA="$SCRIPT_DIR/Data"
export LOCALAPPDATA="$SCRIPT_DIR/Data"
export HOME="$SCRIPT_DIR/Data"

# 启动 {app_name}
echo "Starting {app_name}..."
"$SCRIPT_DIR/App/{main_exe}" "$@"
"#,
        app_name = app_name,
        main_exe = main_exe,
    )
}

/// 生成 README.md
fn generate_readme(app_name: &str, version: Option<&str>, main_exe: &Option<String>) -> String {
    let version_str = version.unwrap_or("unknown");
    let exe_name = main_exe.as_deref().unwrap_or("app.exe");

    format!(
        r#"# {app_name} {version_str} — 便携化包

## 使用方法

### Windows
双击 `launcher.bat` 启动 {app_name}。

### Linux/macOS
```bash
chmod +x launcher.sh
./launcher.sh
```

## 目录结构

```
{app_name}-{version_str}-portable/
├── App/              # 程序文件
│   ├── {exe_name}    # 主程序
│   └── ...
├── Data/             # 运行时数据（用户配置、缓存等）
├── launcher.bat      # Windows 启动脚本
├── launcher.sh       # Linux/macOS 启动脚本
├── manifest.json     # 包元数据
└── README.md         # 本文件
```

## 注意事项

- 此包由 PackForge 自动生成
- 用户数据存储在 `Data/` 目录下
- 删除 `Data/` 目录可恢复到初始状态
"#
    )
}

/// 创建 ZIP 压缩包
fn create_zip_archive(source_dir: &Path, zip_path: &Path) -> Result<()> {
    let file = std::fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));

    add_directory_to_zip(&mut zip, source_dir, source_dir, options)?;

    zip.finish()?;
    Ok(())
}

/// 递归添加目录到 ZIP
fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base_dir: &Path,
    current_dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            add_directory_to_zip(zip, base_dir, &path, options)?;
        } else {
            let relative_path = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            zip.start_file(relative_path, options)?;
            let data = std::fs::read(&path)?;
            zip.write_all(&data)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_batch_has_correct_structure() {
        let content = generate_launcher_batch("MyApp", "myapp.exe");
        assert!(content.contains("APP_DIR=%~dp0"));
        assert!(content.contains("myapp.exe"));
        assert!(content.contains("set \"APPDATA="));
    }

    #[test]
    fn launcher_shell_has_correct_structure() {
        let content = generate_launcher_shell("MyApp", "myapp");
        assert!(content.contains("#!/bin/bash"));
        assert!(content.contains("myapp"));
        assert!(content.contains("APPDATA"));
    }

    #[test]
    fn compute_relative_path_works() {
        let install = Utf8PathBuf::from("C:\\Program Files\\App");
        let file = Utf8PathBuf::from("C:\\Program Files\\App\\bin\\main.exe");
        assert_eq!(compute_relative_path(&install, &file), "bin\\main.exe");
    }

    #[test]
    fn compute_relative_path_handles_root_file() {
        let install = Utf8PathBuf::from("C:\\Program Files\\App");
        let file = Utf8PathBuf::from("C:\\Program Files\\App\\main.exe");
        assert_eq!(compute_relative_path(&install, &file), "main.exe");
    }

    #[test]
    fn readme_generation() {
        let readme = generate_readme("TestApp", Some("1.0.0"), &Some("test.exe".into()));
        assert!(readme.contains("TestApp"));
        assert!(readme.contains("1.0.0"));
        assert!(readme.contains("test.exe"));
    }

    #[test]
    fn zip_archive_creation() {
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().join("App");
        let sub_dir = app_dir.join("sub");
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(app_dir.join("test.txt"), "hello").unwrap();
        std::fs::write(sub_dir.join("nested.txt"), "world").unwrap();

        let zip_path = dir.path().join("test.zip");
        let result = create_zip_archive(dir.path(), &zip_path);
        assert!(result.is_ok());
        assert!(zip_path.exists());
    }
}
