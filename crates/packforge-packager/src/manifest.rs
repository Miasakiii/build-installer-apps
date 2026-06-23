//! PackageManifest 构建器
//!
//! 从分析结果构建打包清单，用于生成安装程序。

use crate::error::Result;
use camino::Utf8PathBuf;
use packforge_core::{
    AnalysisResult, InstallAction, PackageEntry, PackageManifest, SoftwareProfile,
    UninstallAction,
};
use packforge_analyzer::AnalysisReport;

/// 打包选项
#[derive(Debug, Clone)]
pub struct PackageBuildOptions {
    /// 是否包含文档文件
    pub include_docs: bool,
    /// 是否包含可选组件
    pub include_optional: bool,
    /// 安装后是否创建桌面快捷方式
    pub create_desktop_shortcut: bool,
    /// 安装后是否创建开始菜单快捷方式
    pub create_start_menu_shortcut: bool,
    /// 是否注册 COM 组件
    pub register_com: bool,
    /// 是否创建 Windows 服务
    pub create_services: bool,
}

impl Default for PackageBuildOptions {
    fn default() -> Self {
        Self {
            include_docs: false,
            include_optional: false,
            create_desktop_shortcut: true,
            create_start_menu_shortcut: true,
            register_com: true,
            create_services: false,
        }
    }
}

/// 从分析结果构建打包清单
pub fn build_manifest(
    report: &AnalysisReport,
    options: &PackageBuildOptions,
) -> Result<PackageManifest> {
    let analysis = &report.result;
    let profile = &analysis.software;

    // 构建文件条目
    let entries = build_file_entries(analysis, options);

    // 构建注册表条目（仅程序必需的）
    let registry = build_registry_entries(profile);

    // 构建安装动作
    let install_actions = build_install_actions(report, options);

    // 构建卸载动作
    let uninstall_actions = build_uninstall_actions(report, options);

    Ok(PackageManifest {
        name: profile.name.clone(),
        version: profile.version.clone().unwrap_or_else(|| "1.0".to_string()),
        publisher: profile.publisher.clone(),
        description: None,
        license: None,
        icon: find_icon(profile),
        entries,
        registry,
        install_actions,
        uninstall_actions,
    })
}

/// 构建文件条目
fn build_file_entries(
    analysis: &AnalysisResult,
    options: &PackageBuildOptions,
) -> Vec<PackageEntry> {
    let mut entries = Vec::new();

    // 核心文件
    for file in &analysis.core_files {
        entries.push(PackageEntry::File {
            source: file.path.clone(),
            dest: compute_dest_path(&analysis.software.install_dir, &file.path),
            overwrite: true,
        });
    }

    // 库文件（DLL）
    for file in &analysis.software.files {
        if file.file_type == packforge_core::FileType::DynamicLibrary {
            entries.push(PackageEntry::File {
                source: file.path.clone(),
                dest: compute_dest_path(&analysis.software.install_dir, &file.path),
                overwrite: true,
            });
        }
    }

    // 可选组件
    if options.include_optional {
        for file in &analysis.optional_files {
            entries.push(PackageEntry::File {
                source: file.path.clone(),
                dest: compute_dest_path(&analysis.software.install_dir, &file.path),
                overwrite: true,
            });
        }
    }

    // 文档
    if options.include_docs {
        for file in &analysis.software.files {
            if file.file_type == packforge_core::FileType::Documentation {
                entries.push(PackageEntry::File {
                    source: file.path.clone(),
                    dest: compute_dest_path(&analysis.software.install_dir, &file.path),
                    overwrite: true,
                });
            }
        }
    }

    entries
}

/// 构建注册表条目
fn build_registry_entries(profile: &SoftwareProfile) -> Vec<packforge_core::RegistryEntry> {
    // 只包含非系统关键的注册表条目
    profile
        .registry_entries
        .iter()
        .filter(|r| !r.is_system_critical)
        .cloned()
        .collect()
}

/// 构建安装动作
fn build_install_actions(
    report: &AnalysisReport,
    options: &PackageBuildOptions,
) -> Vec<InstallAction> {
    let mut actions = Vec::new();
    let profile = &report.result.software;

    // 创建安装目录
    actions.push(InstallAction::CreateDirectory(
        profile.install_dir.clone(),
    ));

    // 注册 COM 组件
    if options.register_com {
        for com in &profile.com_components {
            if let Some(ref path) = com.server_path {
                actions.push(InstallAction::RegisterCom {
                    dll_path: path.clone(),
                });
            }
        }
    }

    // 创建服务
    if options.create_services {
        for svc in &profile.services {
            actions.push(InstallAction::CreateService {
                name: svc.name.clone(),
                exe_path: svc.exe_path.clone(),
                start_type: svc.start_type,
            });
        }
    }

    // 设置环境变量
    for env in &profile.env_vars {
        actions.push(InstallAction::SetEnvironmentVariable {
            name: env.name.clone(),
            value: env.value.clone(),
        });
    }

    // 创建快捷方式
    let main_exe = find_main_exe(profile);
    if let Some(ref exe) = main_exe {
        if options.create_start_menu_shortcut {
            actions.push(InstallAction::CreateShortcut {
                target: exe.clone(),
                shortcut: Utf8PathBuf::from(format!(
                    "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\{}.lnk",
                    profile.name
                )),
                icon: None,
            });
        }

        if options.create_desktop_shortcut {
            actions.push(InstallAction::CreateShortcut {
                target: exe.clone(),
                shortcut: Utf8PathBuf::from(format!(
                    "%PUBLIC%\\Desktop\\{}.lnk",
                    profile.name
                )),
                icon: None,
            });
        }
    }

    actions
}

/// 构建卸载动作
fn build_uninstall_actions(
    report: &AnalysisReport,
    options: &PackageBuildOptions,
) -> Vec<UninstallAction> {
    let mut actions = Vec::new();
    let profile = &report.result.software;

    // 删除安装目录
    actions.push(UninstallAction::RemoveDirectory(profile.install_dir.clone()));

    // 取消注册 COM 组件
    if options.register_com {
        for com in &profile.com_components {
            if let Some(ref path) = com.server_path {
                actions.push(UninstallAction::UnregisterCom {
                    dll_path: path.clone(),
                });
            }
        }
    }

    // 删除服务
    if options.create_services {
        for svc in &profile.services {
            actions.push(UninstallAction::RemoveService {
                name: svc.name.clone(),
            });
        }
    }

    // 删除环境变量
    for env in &profile.env_vars {
        actions.push(UninstallAction::RemoveEnvironmentVariable {
            name: env.name.clone(),
        });
    }

    // 删除快捷方式
    if options.create_start_menu_shortcut {
        actions.push(UninstallAction::RemoveShortcut(Utf8PathBuf::from(format!(
            "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\{}.lnk",
            profile.name
        ))));
    }

    if options.create_desktop_shortcut {
        actions.push(UninstallAction::RemoveShortcut(Utf8PathBuf::from(format!(
            "%PUBLIC%\\Desktop\\{}.lnk",
            profile.name
        ))));
    }

    actions
}

/// 计算目标路径（相对于安装目录）
fn compute_dest_path(install_dir: &Utf8PathBuf, file_path: &Utf8PathBuf) -> Utf8PathBuf {
    let install_str = install_dir.as_str();
    let file_str = file_path.as_str();

    if let Some(relative) = file_str.strip_prefix(install_str) {
        // 保留相对于安装目录的路径
        let relative = relative.trim_start_matches('\\');
        Utf8PathBuf::from(format!("{{app}}\\{relative}"))
    } else {
        // 未知路径，放到 {app} 根目录
        let file_name = file_path.file_name().unwrap_or("unknown");
        Utf8PathBuf::from(format!("{{app}}\\{file_name}"))
    }
}

/// 查找主 EXE
fn find_main_exe(profile: &SoftwareProfile) -> Option<Utf8PathBuf> {
    profile
        .files
        .iter()
        .filter(|f| f.is_executable && f.file_type == packforge_core::FileType::Executable)
        .filter(|f| {
            let name = f.path.file_name().unwrap_or("").to_lowercase();
            !name.contains("uninstall") && !name.contains("setup") && !name.contains("install")
        })
        .min_by_key(|f| f.path.as_str().len())
        .map(|f| f.path.clone())
}

/// 查找图标文件
fn find_icon(profile: &SoftwareProfile) -> Option<Utf8PathBuf> {
    profile
        .files
        .iter()
        .find(|f| {
            f.path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("ico"))
                .unwrap_or(false)
        })
        .map(|f| f.path.clone())
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_dest_path_relative() {
        let install_dir = Utf8PathBuf::from("C:\\Program Files\\App");
        let file_path = Utf8PathBuf::from("C:\\Program Files\\App\\bin\\main.exe");
        let dest = compute_dest_path(&install_dir, &file_path);
        assert_eq!(dest.as_str(), "{app}\\bin\\main.exe");
    }

    #[test]
    fn compute_dest_path_unknown() {
        let install_dir = Utf8PathBuf::from("C:\\Program Files\\App");
        let file_path = Utf8PathBuf::from("D:\\Other\\file.dll");
        let dest = compute_dest_path(&install_dir, &file_path);
        assert_eq!(dest.as_str(), "{app}\\file.dll");
    }

    #[test]
    fn default_options() {
        let opts = PackageBuildOptions::default();
        assert!(opts.create_desktop_shortcut);
        assert!(opts.create_start_menu_shortcut);
        assert!(opts.register_com);
        assert!(!opts.include_docs);
        assert!(!opts.include_optional);
    }
}
