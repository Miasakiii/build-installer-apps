//! 共享枚举与配置类型

use serde::{Deserialize, Serialize};

/// 安装框架类型（打包工具）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallFramework {
    Nsis,
    InnoSetup,
    WiX,
    InstallShield,
    AdvancedInstaller,
    Electron,
    Msix,
    Custom(String),
    Unknown,
}

/// 运行时环境类型（非安装框架，但影响便携化可行性）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeType {
    DotNet {
        version: String,
        self_contained: bool,
    },
    Java {
        has_embedded_jre: bool,
    },
    Python {
        has_embedded_interpreter: bool,
    },
    Go,
    Rust,
    Node {
        is_electron: bool,
    },
    Native,
}

/// Windows 服务启动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStartType {
    Auto,
    Manual,
    Disabled,
    Boot,
    System,
}

/// 环境变量作用域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvScope {
    Machine,
    User,
    Process,
}

/// 启动脚本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherType {
    Batch,
    Shell,
    Both,
}

/// 提取选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    /// 是否包含可选组件
    pub include_optional: bool,
    /// 是否包含文档
    pub include_docs: bool,
    /// 是否打包为 ZIP
    pub create_zip: bool,
    /// 启动脚本类型
    pub launcher_type: LauncherType,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            include_optional: false,
            include_docs: true,
            create_zip: false,
            launcher_type: LauncherType::Batch,
        }
    }
}

/// 打包选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageOptions {
    /// 输出格式
    pub format: PackageFormat,
    /// 输出目录
    pub output_dir: Option<std::path::PathBuf>,
    /// 是否仅分析（dry-run）
    pub dry_run: bool,
    /// 是否包含用户数据
    pub include_user_data: bool,
}

impl Default for PackageOptions {
    fn default() -> Self {
        Self {
            format: PackageFormat::Portable,
            output_dir: None,
            dry_run: false,
            include_user_data: false,
        }
    }
}

/// 打包输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageFormat {
    Portable,
    InnoSetup,
    Nsis,
    Wix,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_options_default() {
        let opts = ExtractOptions::default();
        assert!(!opts.include_optional);
        assert!(opts.include_docs);
        assert!(!opts.create_zip);
        assert_eq!(opts.launcher_type, LauncherType::Batch);
    }

    #[test]
    fn package_options_default() {
        let opts = PackageOptions::default();
        assert_eq!(opts.format, PackageFormat::Portable);
        assert!(opts.output_dir.is_none());
        assert!(!opts.dry_run);
        assert!(!opts.include_user_data);
    }

    #[test]
    fn install_framework_serialization_roundtrip() {
        let frameworks = vec![
            InstallFramework::Nsis,
            InstallFramework::InnoSetup,
            InstallFramework::WiX,
            InstallFramework::InstallShield,
            InstallFramework::Electron,
            InstallFramework::Msix,
            InstallFramework::Custom("MyInstaller".to_string()),
            InstallFramework::Unknown,
        ];

        for fw in &frameworks {
            let json = serde_json::to_string(fw).unwrap();
            let deserialized: InstallFramework = serde_json::from_str(&json).unwrap();
            assert_eq!(*fw, deserialized);
        }
    }

    #[test]
    fn runtime_type_serialization_roundtrip() {
        let runtimes = vec![
            RuntimeType::DotNet { version: "4.8".to_string(), self_contained: false },
            RuntimeType::Java { has_embedded_jre: true },
            RuntimeType::Python { has_embedded_interpreter: false },
            RuntimeType::Go,
            RuntimeType::Rust,
            RuntimeType::Node { is_electron: true },
            RuntimeType::Native,
        ];

        for rt in &runtimes {
            let json = serde_json::to_string(rt).unwrap();
            let deserialized: RuntimeType = serde_json::from_str(&json).unwrap();
            assert_eq!(*rt, deserialized);
        }
    }

    #[test]
    fn service_start_type_serialization() {
        let types = vec![
            ServiceStartType::Auto,
            ServiceStartType::Manual,
            ServiceStartType::Disabled,
            ServiceStartType::Boot,
            ServiceStartType::System,
        ];

        for st in &types {
            let json = serde_json::to_string(st).unwrap();
            let deserialized: ServiceStartType = serde_json::from_str(&json).unwrap();
            assert_eq!(*st, deserialized);
        }
    }

    #[test]
    fn extract_options_serialization_roundtrip() {
        let opts = ExtractOptions {
            include_optional: true,
            include_docs: false,
            create_zip: true,
            launcher_type: LauncherType::Both,
        };

        let json = serde_json::to_string(&opts).unwrap();
        let deserialized: ExtractOptions = serde_json::from_str(&json).unwrap();
        assert!(deserialized.include_optional);
        assert!(!deserialized.include_docs);
        assert!(deserialized.create_zip);
        assert_eq!(deserialized.launcher_type, LauncherType::Both);
    }
}
