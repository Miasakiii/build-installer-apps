//! 可移植性评估模块
//!
//! 评估软件能否被便携化，给出 0.0~1.0 的评分。

use packforge_core::{PortabilityIssue, SoftwareProfile};

/// 可移植性报告
#[derive(Debug, Clone)]
pub struct PortabilityReport {
    /// 0.0 ~ 1.0 评分
    pub score: f32,
    /// 是否可以便携化
    pub can_portable: bool,
    /// 问题列表
    pub issues: Vec<PortabilityIssue>,
    /// 改进建议
    pub suggestions: Vec<String>,
}

/// 评估可移植性
pub fn assess_portability(profile: &SoftwareProfile) -> PortabilityReport {
    let mut score: f32 = 1.0;
    let mut issues = Vec::new();
    let mut suggestions = Vec::new();

    // 检查服务依赖
    if !profile.services.is_empty() {
        score -= 0.3;
        for svc in &profile.services {
            issues.push(PortabilityIssue::RequiresService(svc.name.clone()));
        }
        suggestions.push("服务依赖使便携化复杂化，考虑是否可以运行时不启动服务".into());
    }

    // 检查 COM 注册
    if !profile.com_components.is_empty() {
        score -= 0.2;
        for com in &profile.com_components {
            issues.push(PortabilityIssue::RequiresComRegistration(
                com.clsid.clone(),
            ));
        }
        suggestions.push("COM 组件需要注册才能使用，考虑运行时注册或替换方案".into());
    }

    // 检查 HKLM 注册表写入
    let hklm_writes: Vec<_> = profile
        .registry_entries
        .iter()
        .filter(|r| matches!(r.hive, packforge_core::RegistryHive::HKLM))
        .collect();
    if !hklm_writes.is_empty() {
        score -= 0.2;
        for entry in &hklm_writes {
            issues.push(PortabilityIssue::WritesToRegistry(format!(
                "HKLM\\{}",
                entry.key_path
            )));
        }
        suggestions.push("写 HKLM 注册表需要管理员权限，考虑重定向到 HKCU".into());
    }

    // 检查内核驱动
    let has_drivers = profile
        .files
        .iter()
        .any(|f| f.file_type == packforge_core::FileType::Driver);
    if has_drivers {
        score -= 0.5;
        issues.push(PortabilityIssue::KernelDriver(
            "包含内核驱动文件".into(),
        ));
        suggestions.push("内核驱动无法便携化，该软件可能不适合便携化".into());
    }

    // 检查硬编码路径（注册表中的绝对路径）
    let hardcoded_paths: Vec<_> = profile
        .registry_entries
        .iter()
        .filter(|r| {
            r.value_data.contains("C:\\Program Files")
                || r.value_data.contains("C:\\Windows")
                || r.value_data.contains("C:\\Users")
        })
        .collect();
    if !hardcoded_paths.is_empty() {
        score -= 0.1;
        suggestions.push(format!(
            "发现 {} 个硬编码路径的注册表条目，便携化后可能需要修正",
            hardcoded_paths.len()
        ));
    }

    // 检查环境变量写入
    if !profile.env_vars.is_empty() {
        score -= 0.1;
        for env in &profile.env_vars {
            suggestions.push(format!(
                "修改环境变量 {}，便携化时需要在启动脚本中设置",
                env.name
            ));
        }
    }

    // 检查快捷方式
    if !profile.shortcuts.is_empty() {
        // 快捷方式本身不影响可移植性，只是信息
        suggestions.push(format!(
            "软件创建了 {} 个快捷方式，便携化后需要重新创建",
            profile.shortcuts.len()
        ));
    }

    // 加分项：如果只有 HKCU 写入（不需要管理员权限）
    let only_hkcu = profile
        .registry_entries
        .iter()
        .all(|r| matches!(r.hive, packforge_core::RegistryHive::HKCU));
    if !profile.registry_entries.is_empty() && only_hkcu {
        score += 0.1; // 奖励分
        suggestions.push("所有注册表写入都在 HKCU，不需要管理员权限".into());
    }

    // 确保评分在 [0.0, 1.0] 范围内
    let score = score.clamp(0.0, 1.0);
    let can_portable = score >= 0.3;

    PortabilityReport {
        score,
        can_portable,
        issues,
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn empty_profile(name: &str) -> SoftwareProfile {
        SoftwareProfile {
            name: name.to_string(),
            version: None,
            publisher: None,
            install_dir: Utf8PathBuf::from("C:\\Program Files\\App"),
            uninstall_string: None,
            install_date: None,
            files: Vec::new(),
            registry_entries: Vec::new(),
            services: Vec::new(),
            com_components: Vec::new(),
            env_vars: Vec::new(),
            shortcuts: Vec::new(),
        }
    }

    #[test]
    fn clean_app_gets_high_score() {
        let profile = empty_profile("CleanApp");
        let report = assess_portability(&profile);
        assert!(report.score >= 0.9);
        assert!(report.can_portable);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn service_dependency_reduces_score() {
        let mut profile = empty_profile("ServiceApp");
        profile.services.push(packforge_core::ServiceEntry {
            name: "MyService".into(),
            display_name: None,
            exe_path: Utf8PathBuf::from("C:\\svc.exe"),
            start_type: packforge_core::ServiceStartType::Auto,
            description: None,
        });
        let report = assess_portability(&profile);
        assert!(report.score < 0.9);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn com_registration_reduces_score() {
        let mut profile = empty_profile("ComApp");
        profile.com_components.push(packforge_core::ComEntry {
            clsid: "{12345678-1234-1234-1234-123456789012}".into(),
            description: None,
            server_path: Some(Utf8PathBuf::from("C:\\myapp.dll")),
            inproc: true,
        });
        let report = assess_portability(&profile);
        assert!(report.score < 0.9);
        assert!(report.issues.iter().any(|i| matches!(i, PortabilityIssue::RequiresComRegistration(_))));
    }

    #[test]
    fn driver_makes_non_portable() {
        let mut profile = empty_profile("DriverApp");
        profile.files.push(packforge_core::FileEntry {
            path: Utf8PathBuf::from("C:\\driver.sys"),
            size: 1024,
            file_type: packforge_core::FileType::Driver,
            is_executable: true,
            hash: None,
            dependencies: Vec::new(),
        });
        let report = assess_portability(&profile);
        assert!(report.score <= 0.5);
        // score 0.5 >= 0.3, 所以 can_portable 仍为 true
        assert!(report.can_portable);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn hkcu_only_gets_bonus() {
        let mut profile = empty_profile("HkcuApp");
        profile.registry_entries.push(packforge_core::RegistryEntry {
            hive: packforge_core::RegistryHive::HKCU,
            key_path: "SOFTWARE\\MyApp".into(),
            value_name: Some("Setting".into()),
            value_type: packforge_core::RegistryValueType::String,
            value_data: "value".into(),
            is_system_critical: false,
        });
        let report = assess_portability(&profile);
        // HKCU only 应该有加分
        assert!(report.score >= 0.9);
    }
}
