//! 文件分类器
//!
//! 将扫描到的文件分为：核心、库、插件、配置、文档、用户数据、临时文件。

use packforge_core::{FileEntry, FileType};

/// 文件分类结果
#[derive(Debug, Clone)]
pub struct ClassifiedFiles {
    pub core: Vec<FileEntry>,
    pub libraries: Vec<FileEntry>,
    pub plugins: Vec<FileEntry>,
    pub configs: Vec<FileEntry>,
    pub docs: Vec<FileEntry>,
    pub user_data: Vec<FileEntry>,
    pub temp: Vec<FileEntry>,
    pub unknown: Vec<FileEntry>,
}

/// 对文件列表进行分类
///
/// 分类策略：
/// 1. 扩展名匹配（快速路径）
/// 2. 路径模式匹配（bin/lib/config/docs/temp 等目录）
/// 3. 文件大小启发式（<1KB 可能是配置）
pub fn classify_files(files: &[FileEntry]) -> ClassifiedFiles {
    let mut result = ClassifiedFiles {
        core: Vec::new(),
        libraries: Vec::new(),
        plugins: Vec::new(),
        configs: Vec::new(),
        docs: Vec::new(),
        user_data: Vec::new(),
        temp: Vec::new(),
        unknown: Vec::new(),
    };

    for file in files {
        let category = classify_single(file);
        match category {
            Category::Core => result.core.push(file.clone()),
            Category::Library => result.libraries.push(file.clone()),
            Category::Plugin => result.plugins.push(file.clone()),
            Category::Config => result.configs.push(file.clone()),
            Category::Doc => result.docs.push(file.clone()),
            Category::UserData => result.user_data.push(file.clone()),
            Category::Temp => result.temp.push(file.clone()),
            Category::Unknown => result.unknown.push(file.clone()),
        }
    }

    result
}

enum Category {
    Core,
    Library,
    Plugin,
    Config,
    Doc,
    #[allow(dead_code)] // P2 阶段用于用户数据剥离
    UserData,
    Temp,
    Unknown,
}

fn classify_single(file: &FileEntry) -> Category {
    // 1. 按 FileType 快速分类
    match file.file_type {
        FileType::Executable => return Category::Core,
        FileType::DynamicLibrary => return Category::Library,
        FileType::StaticLibrary => return Category::Library,
        FileType::Driver => return Category::Core,
        FileType::Plugin => return Category::Plugin,
        FileType::ConfigFile => return Category::Config,
        FileType::Documentation => return Category::Doc,
        FileType::DataFile => return Category::Temp,
        FileType::Unknown => {} // 继续用路径启发式
    }

    // 2. 路径模式匹配
    let path_lower = file.path.as_str().to_lowercase();

    if path_lower.contains("/temp/")
        || path_lower.contains("\\temp\\")
        || path_lower.contains("/cache/")
        || path_lower.contains("\\cache\\")
        || path_lower.contains("/tmp/")
        || path_lower.contains("\\tmp\\")
    {
        return Category::Temp;
    }

    if path_lower.contains("/plugins/")
        || path_lower.contains("\\plugins\\")
        || path_lower.contains("/addons/")
        || path_lower.contains("\\addons\\")
    {
        return Category::Plugin;
    }

    if path_lower.contains("/docs/")
        || path_lower.contains("\\docs\\")
        || path_lower.contains("/doc/")
        || path_lower.contains("\\doc\\")
        || path_lower.contains("/help/")
        || path_lower.contains("\\help\\")
    {
        return Category::Doc;
    }

    // 3. 文件大小启发式
    if file.size < 1024 {
        return Category::Config;
    }

    Category::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn make_file(path: &str, size: u64, ft: FileType) -> FileEntry {
        FileEntry {
            path: Utf8PathBuf::from(path),
            size,
            hash: None,
            file_type: ft,
            is_executable: false,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn exe_classified_as_core() {
        let f = make_file("C:/App/bin/main.exe", 100_000, FileType::Executable);
        assert!(matches!(classify_single(&f), Category::Core));
    }

    #[test]
    fn dll_classified_as_library() {
        let f = make_file("C:/App/lib/core.dll", 50_000, FileType::DynamicLibrary);
        assert!(matches!(classify_single(&f), Category::Library));
    }

    #[test]
    fn config_json_classified_as_config() {
        let f = make_file("C:/App/config.json", 500, FileType::ConfigFile);
        assert!(matches!(classify_single(&f), Category::Config));
    }

    #[test]
    fn temp_path_classified_as_temp() {
        let f = make_file("C:/App/temp/cache.dat", 100, FileType::Unknown);
        assert!(matches!(classify_single(&f), Category::Temp));
    }
}
