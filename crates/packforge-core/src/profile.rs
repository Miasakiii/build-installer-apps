//! 软件画像与分析结果数据模型

use camino::Utf8PathBuf;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::types::{InstallFramework, RuntimeType, ServiceStartType};

// ───────────────────────────── SoftwareProfile ─────────────────────────────

/// 一个已安装软件的完整画像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareProfile {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_dir: Utf8PathBuf,
    pub uninstall_string: Option<String>,
    pub install_date: Option<NaiveDate>,

    // 扫描结果
    pub files: Vec<FileEntry>,
    pub registry_entries: Vec<RegistryEntry>,
    pub services: Vec<ServiceEntry>,
    pub com_components: Vec<ComEntry>,
    pub env_vars: Vec<EnvVarEntry>,
    pub shortcuts: Vec<ShortcutEntry>,
}

/// 软件发现时的简要信息（列表用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_dir: Option<Utf8PathBuf>,
    pub uninstall_string: Option<String>,
    pub install_date: Option<NaiveDate>,
    pub estimated_size_kb: Option<u32>,
    pub is_system_component: bool,
}

// ───────────────────────────── FileEntry ─────────────────────────────

/// 文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: Utf8PathBuf,
    pub size: u64,
    /// SHA-256 哈希（延迟计算时为 None）
    pub hash: Option<[u8; 32]>,
    pub file_type: FileType,
    pub is_executable: bool,
    /// PE 导入的 DLL 列表
    pub dependencies: Vec<String>,
}

/// 文件类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Executable,
    DynamicLibrary,
    StaticLibrary,
    ConfigFile,
    DataFile,
    Documentation,
    Plugin,
    Driver,
    Unknown,
}

// ───────────────────────────── RegistryEntry ─────────────────────────────

/// 注册表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub hive: RegistryHive,
    pub key_path: String,
    pub value_name: Option<String>,
    pub value_type: RegistryValueType,
    pub value_data: String,
    /// 是否是程序运行必需
    pub is_system_critical: bool,
}

/// 注册表根键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryHive {
    HKLM,
    HKCU,
    HKCR,
    HKU,
    HKCC,
}

/// 注册表值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryValueType {
    String,
    ExpandString,
    Binary,
    DWord,
    QWord,
    MultiString,
    Unknown,
}

// ───────────────────────────── Service / COM / Env / Shortcut ─────────────────────────────

/// Windows 服务条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub display_name: Option<String>,
    pub exe_path: Utf8PathBuf,
    pub start_type: ServiceStartType,
    pub description: Option<String>,
}

/// COM 组件注册条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComEntry {
    pub clsid: String,
    pub description: Option<String>,
    pub server_path: Option<Utf8PathBuf>,
    pub inproc: bool,
}

/// 环境变量条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarEntry {
    pub name: String,
    pub value: String,
    pub is_path_extended: bool,
}

/// 快捷方式条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEntry {
    pub path: Utf8PathBuf,
    pub target: Utf8PathBuf,
    pub arguments: Option<String>,
    pub icon_path: Option<Utf8PathBuf>,
    pub working_dir: Option<Utf8PathBuf>,
}

// ───────────────────────────── AnalysisResult ─────────────────────────────

/// 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub software: SoftwareProfile,

    // 分类结果
    pub core_files: Vec<FileEntry>,
    pub optional_files: Vec<FileEntry>,
    pub user_data_files: Vec<FileEntry>,
    pub temp_files: Vec<FileEntry>,

    pub core_registry: Vec<RegistryEntry>,
    pub user_registry: Vec<RegistryEntry>,

    // 依赖分析
    pub external_dependencies: Vec<Dependency>,
    pub install_framework: InstallFramework,
    pub runtime: Option<RuntimeType>,

    // 可移植性评估
    pub portability_score: f32,
    pub portability_issues: Vec<PortabilityIssue>,
}

/// 外部依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    /// 是否系统自带
    pub is_system: bool,
    /// 获取地址
    pub download_url: Option<String>,
}

/// 可移植性问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortabilityIssue {
    /// 硬编码绝对路径
    HardcodedPath(String),
    /// 依赖 Windows 服务
    RequiresService(String),
    /// 需要 COM 注册
    RequiresComRegistration(String),
    /// 必须写注册表
    WritesToRegistry(String),
    /// 需要管理员权限
    RequiresAdmin,
    /// 使用全局互斥量
    UsesGlobalMutex(String),
    /// 内核驱动
    KernelDriver(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_info_serialization_roundtrip() {
        let info = SoftwareInfo {
            name: "Test App".to_string(),
            version: Some("2.0.0".to_string()),
            publisher: Some("Test Publisher".to_string()),
            install_dir: Some(Utf8PathBuf::from("C:\\Program Files\\TestApp")),
            uninstall_string: Some("C:\\uninstall.exe".to_string()),
            install_date: None,
            estimated_size_kb: Some(1024),
            is_system_component: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SoftwareInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test App");
        assert_eq!(deserialized.version, Some("2.0.0".to_string()));
        assert_eq!(deserialized.estimated_size_kb, Some(1024));
        assert!(!deserialized.is_system_component);
    }

    #[test]
    fn file_entry_serialization_roundtrip() {
        let entry = FileEntry {
            path: Utf8PathBuf::from("bin/app.exe"),
            size: 1048576,
            hash: Some([0u8; 32]),
            file_type: FileType::Executable,
            is_executable: true,
            dependencies: vec!["kernel32.dll".to_string(), "user32.dll".to_string()],
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, Utf8PathBuf::from("bin/app.exe"));
        assert_eq!(deserialized.size, 1048576);
        assert_eq!(deserialized.file_type, FileType::Executable);
        assert!(deserialized.is_executable);
        assert_eq!(deserialized.dependencies.len(), 2);
    }

    #[test]
    fn file_type_serialization_roundtrip() {
        let types = vec![
            FileType::Executable,
            FileType::DynamicLibrary,
            FileType::StaticLibrary,
            FileType::ConfigFile,
            FileType::DataFile,
            FileType::Documentation,
            FileType::Plugin,
            FileType::Driver,
            FileType::Unknown,
        ];

        for ft in &types {
            let json = serde_json::to_string(ft).unwrap();
            let deserialized: FileType = serde_json::from_str(&json).unwrap();
            assert_eq!(*ft, deserialized);
        }
    }

    #[test]
    fn registry_entry_serialization() {
        let entry = RegistryEntry {
            hive: RegistryHive::HKLM,
            key_path: "SOFTWARE\\TestApp".to_string(),
            value_name: Some("InstallPath".to_string()),
            value_type: RegistryValueType::String,
            value_data: "C:\\TestApp".to_string(),
            is_system_critical: false,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hive, RegistryHive::HKLM);
        assert_eq!(deserialized.key_path, "SOFTWARE\\TestApp");
        assert_eq!(deserialized.value_type, RegistryValueType::String);
    }

    #[test]
    fn dependency_serialization() {
        let dep = Dependency {
            name: "msvcp140.dll".to_string(),
            version: Some("14.38".to_string()),
            is_system: false,
            download_url: Some("https://aka.ms/vs/17/release/vc_redist.x64.exe".to_string()),
        };

        let json = serde_json::to_string(&dep).unwrap();
        let deserialized: Dependency = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "msvcp140.dll");
        assert!(!deserialized.is_system);
        assert!(deserialized.download_url.is_some());
    }

    #[test]
    fn portability_issue_serialization_roundtrip() {
        let issues = vec![
            PortabilityIssue::HardcodedPath("C:\\Users".to_string()),
            PortabilityIssue::RequiresService("MyService".to_string()),
            PortabilityIssue::RequiresComRegistration("MyPlugin.dll".to_string()),
            PortabilityIssue::WritesToRegistry("HKLM\\SOFTWARE\\Test".to_string()),
            PortabilityIssue::RequiresAdmin,
            PortabilityIssue::UsesGlobalMutex("Global\\MyApp".to_string()),
            PortabilityIssue::KernelDriver("mydriver.sys".to_string()),
        ];

        for issue in &issues {
            let json = serde_json::to_string(issue).unwrap();
            let _: PortabilityIssue = serde_json::from_str(&json).unwrap();
        }
    }
}
