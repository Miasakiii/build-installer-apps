//! PackForge Core — 核心数据模型与抽象
//!
//! 定义 PackForge 全局共享的数据结构、错误类型和 trait。

pub mod error;
pub mod manifest;
pub mod profile;
pub mod types;

// 便捷 re-export
pub use error::{CoreError, Result};
pub use manifest::{InstallAction, PackageEntry, PackageManifest, UninstallAction};
pub use profile::{
    AnalysisResult, ComEntry, Dependency, EnvVarEntry, FileEntry, FileType, PortabilityIssue,
    RegistryEntry, RegistryHive, RegistryValueType, ServiceEntry, ShortcutEntry, SoftwareInfo,
    SoftwareProfile,
};
pub use types::{
    EnvScope, ExtractOptions, InstallFramework, LauncherType, PackageOptions, RuntimeType,
    ServiceStartType,
};
