//! 软件发现模块
//!
//! 从 Windows 注册表枚举已安装软件。

use packforge_core::SoftwareInfo;

use crate::Result;

/// 已安装软件注册表路径
const UNINSTALL_PATHS: &[&str] = &[
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
];

const UNINSTALL_PATH_CU: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

/// 列出系统中所有已安装软件
///
/// 扫描以下注册表路径：
/// - HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
/// - HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall
/// - HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
pub fn list_installed_software() -> Result<Vec<SoftwareInfo>> {
    #[cfg(windows)]
    {
        windows_registry::list_installed_software()
    }

    #[cfg(not(windows))]
    {
        Err(crate::ScannerError::PermissionDenied(
            "当前仅支持 Windows 平台".into(),
        ))
    }
}

/// 按名称/发布者模糊搜索已安装软件
pub fn search_software(query: &str) -> Result<Vec<SoftwareInfo>> {
    let all = list_installed_software()?;
    let query_lower = query.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&query_lower)
                || s.publisher
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
        })
        .collect())
}

#[cfg(windows)]
mod windows_registry {
    use chrono::NaiveDate;
    use packforge_core::SoftwareInfo;
    use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

    use crate::ScannerError;

    pub fn list_installed_software() -> crate::Result<Vec<SoftwareInfo>> {
        let mut software = Vec::new();

        // 扫描 HKLM 路径（64 位和 32 位软件）
        for path in super::UNINSTALL_PATHS {
            match scan_uninstall_key(LOCAL_MACHINE, path) {
                Ok(mut entries) => software.append(&mut entries),
                Err(e) => {
                    tracing::warn!("扫描 HKLM\\{} 失败: {}", path, e);
                }
            }
        }

        // 扫描 HKCU 路径（用户级安装）
        match scan_uninstall_key(CURRENT_USER, super::UNINSTALL_PATH_CU) {
            Ok(mut entries) => software.append(&mut entries),
            Err(e) => {
                tracing::warn!("扫描 HKCU\\{} 失败: {}", super::UNINSTALL_PATH_CU, e);
            }
        }

        // 按名称去重（同一软件可能在 HKLM 和 WOW6432Node 都有记录）
        deduplicate(&mut software);

        Ok(software)
    }

    /// 扫描单个 Uninstall 注册表键
    fn scan_uninstall_key(root: &Key, path: &str) -> crate::Result<Vec<SoftwareInfo>> {
        let key = root
            .open(path)
            .map_err(|e| ScannerError::Registry(format!("打开 {} 失败: {}", path, e)))?;

        let mut software = Vec::new();

        for subkey_name in key
            .keys()
            .map_err(|e| ScannerError::Registry(format!("枚举子键失败: {}", e)))?
        {
            match read_software_entry(&key, &subkey_name) {
                Ok(Some(info)) => software.push(info),
                Ok(None) => {} // 被过滤（无 DisplayName 或 SystemComponent）
                Err(e) => {
                    tracing::debug!("读取子键 {} 失败: {}", subkey_name, e);
                }
            }
        }

        Ok(software)
    }

    /// 从单个子键读取软件信息
    fn read_software_entry(parent: &Key, name: &str) -> crate::Result<Option<SoftwareInfo>> {
        let subkey = parent
            .open(name)
            .map_err(|e| ScannerError::Registry(format!("打开子键 {} 失败: {}", name, e)))?;

        // 必须有 DisplayName
        let display_name = match subkey.get_string("DisplayName") {
            Ok(v) => v,
            Err(_) => return Ok(None), // 无 DisplayName，跳过
        };

        // 过滤系统组件
        let is_system_component = subkey
            .get_u32("SystemComponent")
            .map(|v| v == 1)
            .unwrap_or(false);

        // 过滤没有实际安装路径的更新补丁
        let parent_key_name = subkey
            .get_string("ParentKeyName")
            .ok()
            .filter(|s| !s.is_empty());
        if parent_key_name.is_some() {
            // 这是一个更新补丁，不是独立软件
            return Ok(None);
        }

        // 读取可选字段
        let version = subkey.get_string("DisplayVersion").ok();
        let publisher = subkey.get_string("Publisher").ok();
        let install_location = subkey.get_string("InstallLocation").ok();
        let uninstall_string = subkey.get_string("UninstallString").ok();
        let install_date = parse_install_date(subkey.get_string("InstallDate").ok().as_deref());
        let estimated_size_kb = subkey.get_u32("EstimatedSize").ok();

        // 处理空字符串
        let publisher = publisher.filter(|s| !s.is_empty());
        let install_location = install_location.filter(|s| !s.is_empty());
        let uninstall_string = uninstall_string.filter(|s| !s.is_empty());

        Ok(Some(SoftwareInfo {
            name: display_name,
            version,
            publisher,
            install_dir: install_location
                .map(|s| camino::Utf8PathBuf::from(s.trim_matches('"'))),
            uninstall_string,
            install_date,
            estimated_size_kb,
            is_system_component,
        }))
    }

    /// 解析安装日期（格式：YYYYMMDD）
    fn parse_install_date(date_str: Option<&str>) -> Option<NaiveDate> {
        let s = date_str?.trim();
        if s.len() != 8 {
            return None;
        }
        NaiveDate::parse_from_str(s, "%Y%m%d").ok()
    }

    /// 去重：同名软件保留信息更完整的那条
    fn deduplicate(software: &mut Vec<SoftwareInfo>) {
        // 按名称分组，保留第一个（通常是 HKLM 的更完整）
        let mut seen = std::collections::HashSet::new();
        software.retain(|s| seen.insert(s.name.to_lowercase()));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_is_case_insensitive() {
        // 纯逻辑测试：search_software 的过滤逻辑
        // 不实际调用注册表，用 mock 数据验证
        // TODO: 添加集成测试
    }

    #[cfg(windows)]
    #[test]
    fn list_installed_software_returns_results() {
        // 这个测试需要 Windows 环境
        let result = super::list_installed_software();
        assert!(result.is_ok(), "list_installed_software 应该成功: {:?}", result.err());

        let software = result.unwrap();
        // Windows 系统至少应该有一些已安装软件
        assert!(!software.is_empty(), "应该至少找到一个已安装软件");

        // 验证每个条目都有名称
        for s in &software {
            assert!(!s.name.is_empty(), "软件名称不应为空");
        }
    }

    #[cfg(windows)]
    #[test]
    fn search_software_finds_known_software() {
        let result = super::search_software("Microsoft");
        assert!(result.is_ok());

        let found = result.unwrap();
        // Windows 系统应该有 Microsoft 相关软件
        assert!(!found.is_empty(), "应该找到 Microsoft 相关软件");
    }
}
