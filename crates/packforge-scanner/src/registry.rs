//! Registry 深度扫描 — 扫描指定软件相关的所有注册表条目
//!
//! 使用多种策略定位软件相关的注册表项：
//! 1. 精确路径（从 Uninstall 键获取的路径前缀匹配）
//! 2. 名称搜索（在 HKLM/HKCU 按 DisplayName 搜索）
//! 3. GUID 搜索（MSI 安装的软件有 ProductCode GUID）
//! 4. 发布者搜索（按发布者名搜索相关键）
//! 5. 安装路径搜索（App Paths 关联）

use crate::error::{Result, ScannerError};
use packforge_core::{RegistryEntry, RegistryHive, RegistryValueType, SoftwareInfo};

use std::collections::HashSet;
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

// ── 公共 API ──────────────────────────────────────────────────────────────────

/// 扫描指定软件相关的所有注册表条目
///
/// 使用多种策略尽可能完整地收集注册表信息。
pub fn scan_registry_for_software(software: &SoftwareInfo) -> Result<Vec<RegistryEntry>> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();

    // 策略 1：精确路径 — 从 Uninstall 键的原始条目开始
    scan_uninstall_entry(software, &mut entries, &mut seen)?;

    // 策略 2：名称搜索 — 在常见位置按名称搜索
    search_by_name(software, &mut entries, &mut seen)?;

    // 策略 3：GUID 搜索 — MSI 产品代码
    if let Some(ref guid) = extract_guid_from_uninstall(software) {
        search_by_guid(guid, &mut entries, &mut seen)?;
    }

    // 策略 4：发布者搜索
    if let Some(ref publisher) = software.publisher {
        search_by_publisher(publisher, &software.name, &mut entries, &mut seen)?;
    }

    // 策略 5：安装路径搜索
    if let Some(ref install_dir) = software.install_dir {
        search_by_install_path(install_dir.as_str(), &mut entries, &mut seen)?;
    }

    Ok(entries)
}

/// 扫描指定路径下的所有注册表值（递归）
pub fn scan_registry_tree(hive: RegistryHive, key_path: &str, max_depth: u32) -> Result<Vec<RegistryEntry>> {
    let root = match hive {
        RegistryHive::HKLM => LOCAL_MACHINE,
        RegistryHive::HKCU => CURRENT_USER,
        _ => return Ok(Vec::new()),
    };

    let key = root
        .open(key_path)
        .map_err(|e| ScannerError::Registry(format!("打开注册表键失败: {key_path}: {e}")))?;

    let mut entries = Vec::new();
    collect_key_values(hive, key_path, &key, &mut entries);

    if max_depth > 0 {
        collect_subkeys(hive, key_path, &key, &mut entries, max_depth);
    }

    Ok(entries)
}

// ── 策略实现 ──────────────────────────────────────────────────────────────────

/// 策略 1：扫描 Uninstall 键的完整条目
fn scan_uninstall_entry(
    software: &SoftwareInfo,
    entries: &mut Vec<RegistryEntry>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    let uninstall_paths = [
        (RegistryHive::HKLM, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegistryHive::HKLM, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegistryHive::HKCU, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    ];

    for (hive, base_path) in &uninstall_paths {
        let root = match hive {
            RegistryHive::HKLM => LOCAL_MACHINE,
            RegistryHive::HKCU => CURRENT_USER,
            _ => continue,
        };

        let base_key = match root.open(base_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let subkey_names = match base_key.keys() {
            Ok(names) => names,
            Err(_) => continue,
        };

        for subkey_name in subkey_names {
            let subkey = match base_key.open(&subkey_name) {
                Ok(k) => k,
                Err(_) => continue,
            };

            let display_name = subkey.get_string("DisplayName").unwrap_or_default();
            if display_name != software.name {
                continue;
            }

            let full_path = format!("{base_path}\\{subkey_name}");
            if seen.contains(&full_path) {
                continue;
            }

            collect_key_values(*hive, &full_path, &subkey, entries);
            seen.insert(full_path);
        }
    }

    Ok(())
}

/// 策略 2：按名称搜索
fn search_by_name(
    software: &SoftwareInfo,
    entries: &mut Vec<RegistryEntry>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    let search_roots = [
        (RegistryHive::HKLM, "SOFTWARE"),
        (RegistryHive::HKCU, "SOFTWARE"),
    ];

    let name_lower = software.name.to_lowercase();

    for (hive, root_path) in &search_roots {
        let root = match hive {
            RegistryHive::HKLM => LOCAL_MACHINE,
            RegistryHive::HKCU => CURRENT_USER,
            _ => continue,
        };

        let key = match root.open(root_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let subkey_names = match key.keys() {
            Ok(names) => names,
            Err(_) => continue,
        };

        for subkey_name in subkey_names {
            if subkey_name.to_lowercase().contains(&name_lower) {
                let full_path = format!("{root_path}\\{subkey_name}");
                if seen.contains(&full_path) {
                    continue;
                }

                if let Ok(subkey) = key.open(&subkey_name) {
                    collect_key_values(*hive, &full_path, &subkey, entries);
                    seen.insert(full_path);
                }
            }
        }
    }

    Ok(())
}

/// 策略 3：按 GUID 搜索（MSI 产品）
fn search_by_guid(
    guid: &str,
    entries: &mut Vec<RegistryEntry>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    // 搜索 HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{GUID}
    let uninstall_path = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{guid}");
    if !seen.contains(&uninstall_path)
        && let Ok(key) = LOCAL_MACHINE.open(&uninstall_path) {
            collect_key_values(RegistryHive::HKLM, &uninstall_path, &key, entries);
            seen.insert(uninstall_path);
        }

    // 搜索 HKCR\CLSID 下的 GUID
    let clsid_path = format!("CLSID\\{guid}");
    if !seen.contains(&clsid_path)
        && let Ok(key) = LOCAL_MACHINE.open(&clsid_path) {
            collect_key_values(RegistryHive::HKLM, &clsid_path, &key, entries);
            seen.insert(clsid_path);
        }

    Ok(())
}

/// 策略 4：按发布者搜索
fn search_by_publisher(
    publisher: &str,
    software_name: &str,
    entries: &mut Vec<RegistryEntry>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    let publisher_lower = publisher.to_lowercase();
    let name_lower = software_name.to_lowercase();

    let search_roots = [
        (RegistryHive::HKLM, "SOFTWARE"),
        (RegistryHive::HKCU, "SOFTWARE"),
    ];

    for (hive, root_path) in &search_roots {
        let root = match hive {
            RegistryHive::HKLM => LOCAL_MACHINE,
            RegistryHive::HKCU => CURRENT_USER,
            _ => continue,
        };

        let key = match root.open(root_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let subkey_names = match key.keys() {
            Ok(names) => names,
            Err(_) => continue,
        };

        for subkey_name in subkey_names {
            if subkey_name.to_lowercase() == publisher_lower {
                let publisher_path = format!("{root_path}\\{subkey_name}");
                if let Ok(pub_key) = key.open(&subkey_name) {
                    let product_names = match pub_key.keys() {
                        Ok(names) => names,
                        Err(_) => continue,
                    };

                    for product_name in product_names {
                        if product_name.to_lowercase().contains(&name_lower) {
                            let full_path = format!("{publisher_path}\\{product_name}");
                            if seen.contains(&full_path) {
                                continue;
                            }
                            if let Ok(prod_key) = pub_key.open(&product_name) {
                                collect_key_values(*hive, &full_path, &prod_key, entries);
                                seen.insert(full_path);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// 策略 5：按安装路径搜索
fn search_by_install_path(
    install_dir: &str,
    entries: &mut Vec<RegistryEntry>,
    seen: &mut HashSet<String>,
) -> Result<()> {
    let install_lower = install_dir.to_lowercase();

    // 搜索 App Paths
    let app_paths_base = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths";
    let base_key = match LOCAL_MACHINE.open(app_paths_base) {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };

    let subkey_names = match base_key.keys() {
        Ok(names) => names,
        Err(_) => return Ok(()),
    };

    for subkey_name in subkey_names {
        let full_path = format!("{app_paths_base}\\{subkey_name}");
        if seen.contains(&full_path) {
            continue;
        }
        if let Ok(subkey) = base_key.open(&subkey_name) {
            // 检查默认值是否指向安装目录
            if let Ok(default_val) = subkey.get_string("")
                && default_val.to_lowercase().contains(&install_lower) {
                    collect_key_values(RegistryHive::HKLM, &full_path, &subkey, entries);
                    seen.insert(full_path);
                }
        }
    }

    Ok(())
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 从 Uninstall 条目中提取 GUID
fn extract_guid_from_uninstall(software: &SoftwareInfo) -> Option<String> {
    let uninstall_paths = [
        (RegistryHive::HKLM, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegistryHive::HKLM, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegistryHive::HKCU, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    ];

    for (hive, base_path) in &uninstall_paths {
        let root = match hive {
            RegistryHive::HKLM => LOCAL_MACHINE,
            RegistryHive::HKCU => CURRENT_USER,
            _ => continue,
        };

        let base_key = match root.open(base_path) {
            Ok(k) => k,
            Err(_) => continue,
        };

        let subkey_names = match base_key.keys() {
            Ok(names) => names,
            Err(_) => continue,
        };

        for subkey_name in subkey_names {
            // GUID 格式的子键名
            if subkey_name.starts_with('{') && subkey_name.ends_with('}')
                && let Ok(subkey) = base_key.open(&subkey_name) {
                    let display_name = subkey.get_string("DisplayName").unwrap_or_default();
                    if display_name == software.name {
                        return Some(subkey_name);
                    }
                }
        }
    }

    None
}

/// 收集注册表键的所有值
fn collect_key_values(
    hive: RegistryHive,
    key_path: &str,
    key: &Key,
    entries: &mut Vec<RegistryEntry>,
) {
    let value_iter = match key.values() {
        Ok(iter) => iter,
        Err(_) => return,
    };

    for (name, value) in value_iter {
        let (value_type, value_data) = value_to_string(&name, value, key);
        entries.push(RegistryEntry {
            hive,
            key_path: key_path.to_string(),
            value_name: Some(name),
            value_type,
            value_data,
            is_system_critical: false,
        });
    }
}

/// 将注册表值转换为字符串表示
fn value_to_string(name: &str, value: windows_registry::Value, key: &Key) -> (RegistryValueType, String) {
    use windows_registry::Type;

    match value.ty() {
        Type::String | Type::ExpandString => {
            if let Ok(val) = key.get_string(name) {
                (RegistryValueType::String, val)
            } else {
                (RegistryValueType::Unknown, String::new())
            }
        }
        Type::U32 => {
            if let Ok(val) = key.get_u32(name) {
                (RegistryValueType::DWord, val.to_string())
            } else {
                (RegistryValueType::Unknown, String::new())
            }
        }
        Type::U64 => {
            if let Ok(val) = key.get_u64(name) {
                (RegistryValueType::QWord, val.to_string())
            } else {
                (RegistryValueType::Unknown, String::new())
            }
        }
        Type::MultiString => {
            if let Ok(val) = key.get_multi_string(name) {
                (RegistryValueType::MultiString, val.join("; "))
            } else {
                (RegistryValueType::Unknown, String::new())
            }
        }
        Type::Bytes => {
            let raw = value.as_wide();
            // 转换为十六进制字符串
            let bytes: Vec<u8> = raw.iter().flat_map(|w| w.to_le_bytes()).collect();
            let hex: String = bytes.iter().take(32).map(|b| format!("{b:02x}")).collect();
            let suffix = if bytes.len() > 32 { "..." } else { "" };
            (RegistryValueType::Binary, format!("{hex}{suffix}"))
        }
        _ => (RegistryValueType::Unknown, String::new()),
    }
}

/// 递归收集子键的值
fn collect_subkeys(
    hive: RegistryHive,
    parent_path: &str,
    key: &Key,
    entries: &mut Vec<RegistryEntry>,
    remaining_depth: u32,
) {
    if remaining_depth == 0 {
        return;
    }

    let subkey_names = match key.keys() {
        Ok(names) => names,
        Err(_) => return,
    };

    for subkey_name in subkey_names {
        let full_path = format!("{parent_path}\\{subkey_name}");
        if let Ok(subkey) = key.open(&subkey_name) {
            collect_key_values(hive, &full_path, &subkey, entries);
            collect_subkeys(hive, &full_path, &subkey, entries, remaining_depth - 1);
        }
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    fn make_test_software(name: &str) -> SoftwareInfo {
        SoftwareInfo {
            name: name.to_string(),
            version: Some("1.0".to_string()),
            publisher: Some("Test Publisher".to_string()),
            install_dir: Some(Utf8PathBuf::from("C:\\Program Files\\Test")),
            uninstall_string: None,
            install_date: None,
            estimated_size_kb: None,
            is_system_component: false,
        }
    }

    #[test]
    fn scan_registry_returns_entries() {
        let software = make_test_software("7-Zip");
        let result = scan_registry_for_software(&software);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn scan_registry_deduplicates() {
        let software = make_test_software("7-Zip");
        let entries = scan_registry_for_software(&software).unwrap();

        let mut seen = HashSet::new();
        for entry in &entries {
            let key = format!(
                "{:?}\\{}\\{}",
                entry.hive,
                entry.key_path,
                entry.value_name.as_deref().unwrap_or("")
            );
            assert!(seen.insert(key.clone()), "发现重复条目: {key}");
        }
    }

    #[test]
    fn scan_registry_tree_works() {
        let result = scan_registry_tree(
            RegistryHive::HKLM,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion",
            0,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn extract_guid_returns_none_for_non_msi() {
        let software = make_test_software("NonExistent12345");
        let guid = extract_guid_from_uninstall(&software);
        assert!(guid.is_none());
    }
}
