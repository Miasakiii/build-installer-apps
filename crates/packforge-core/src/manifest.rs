//! 打包清单数据模型

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::profile::RegistryEntry;
use crate::types::ServiceStartType;

/// 打包清单——描述一个完整的安装包内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub icon: Option<Utf8PathBuf>,

    pub entries: Vec<PackageEntry>,
    pub registry: Vec<RegistryEntry>,
    pub install_actions: Vec<InstallAction>,
    pub uninstall_actions: Vec<UninstallAction>,
}

/// 打包条目——文件或目录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageEntry {
    File {
        /// 源路径（本地文件系统）
        source: Utf8PathBuf,
        /// 安装后目标路径
        dest: Utf8PathBuf,
        overwrite: bool,
    },
    Directory {
        source: Utf8PathBuf,
        dest: Utf8PathBuf,
    },
}

/// 安装时执行的动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallAction {
    CreateDirectory(Utf8PathBuf),
    SetPermissions {
        path: Utf8PathBuf,
        permissions: String,
    },
    RegisterCom {
        dll_path: Utf8PathBuf,
    },
    CreateService {
        name: String,
        exe_path: Utf8PathBuf,
        start_type: ServiceStartType,
    },
    SetEnvironmentVariable {
        name: String,
        value: String,
    },
    CreateShortcut {
        target: Utf8PathBuf,
        shortcut: Utf8PathBuf,
        icon: Option<Utf8PathBuf>,
    },
    RunCommand {
        command: String,
        args: Vec<String>,
    },
}

/// 卸载时执行的清理动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UninstallAction {
    RemoveDirectory(Utf8PathBuf),
    RemoveFile(Utf8PathBuf),
    UnregisterCom {
        dll_path: Utf8PathBuf,
    },
    RemoveService {
        name: String,
    },
    RemoveEnvironmentVariable {
        name: String,
    },
    RemoveShortcut(Utf8PathBuf),
    RunCommand {
        command: String,
        args: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manifest_serialization_roundtrip() {
        let manifest = PackageManifest {
            name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            publisher: Some("Test Publisher".to_string()),
            description: Some("A test application".to_string()),
            license: Some("MIT".to_string()),
            icon: None,
            entries: vec![
                PackageEntry::File {
                    source: Utf8PathBuf::from("staging/bin/app.exe"),
                    dest: Utf8PathBuf::from("bin/app.exe"),
                    overwrite: true,
                },
            ],
            registry: vec![],
            install_actions: vec![],
            uninstall_actions: vec![],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: PackageManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "TestApp");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.publisher, Some("Test Publisher".to_string()));
        assert_eq!(deserialized.entries.len(), 1);
    }

    #[test]
    fn package_entry_file_serialization() {
        let entry = PackageEntry::File {
            source: Utf8PathBuf::from("src/lib.dll"),
            dest: Utf8PathBuf::from("lib/lib.dll"),
            overwrite: false,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("File"));
        assert!(json.contains("src/lib.dll"));

        let deserialized: PackageEntry = serde_json::from_str(&json).unwrap();
        match deserialized {
            PackageEntry::File { source, dest, overwrite } => {
                assert_eq!(source, Utf8PathBuf::from("src/lib.dll"));
                assert_eq!(dest, Utf8PathBuf::from("lib/lib.dll"));
                assert!(!overwrite);
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn install_action_serialization_roundtrip() {
        let actions = vec![
            InstallAction::CreateDirectory(Utf8PathBuf::from("data")),
            InstallAction::RegisterCom {
                dll_path: Utf8PathBuf::from("bin/plugin.dll"),
            },
            InstallAction::SetEnvironmentVariable {
                name: "APP_HOME".to_string(),
                value: "C:\\TestApp".to_string(),
            },
        ];

        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let _: InstallAction = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn uninstall_action_serialization_roundtrip() {
        let actions = vec![
            UninstallAction::RemoveDirectory(Utf8PathBuf::from("data")),
            UninstallAction::RemoveFile(Utf8PathBuf::from("config.ini")),
            UninstallAction::UnregisterCom {
                dll_path: Utf8PathBuf::from("bin/plugin.dll"),
            },
        ];

        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let _: UninstallAction = serde_json::from_str(&json).unwrap();
        }
    }
}
