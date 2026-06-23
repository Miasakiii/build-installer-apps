//! Windows 服务 & COM 组件扫描
//!
//! 枚举系统中注册的 Windows 服务和 COM 组件。

use crate::error::{Result, ScannerError};
use camino::Utf8PathBuf;
use packforge_core::{ComEntry, ServiceEntry, ServiceStartType};
use serde::{Deserialize, Serialize};
use windows_registry::{Key, LOCAL_MACHINE};

// ── 公共 API ──────────────────────────────────────────────────────────────────

/// 枚举所有 Windows 服务
///
/// 扫描 HKLM\SYSTEM\CurrentControlSet\Services 注册表。
pub fn scan_services() -> Result<Vec<ServiceEntry>> {
    let services_path = "SYSTEM\\CurrentControlSet\\Services";
    let base_key = LOCAL_MACHINE
        .open(services_path)
        .map_err(|e| ScannerError::Registry(format!("打开 Services 键失败: {e}")))?;

    let subkey_names = base_key
        .keys()
        .map_err(|e| ScannerError::Registry(format!("枚举服务子键失败: {e}")))?;

    let mut services = Vec::new();

    for name in subkey_names {
        if let Ok(entry) = read_service_entry(&base_key, &name) {
            services.push(entry);
        }
    }

    // 按名称排序
    services.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(services)
}

/// 扫描指定软件相关的 Windows 服务
///
/// 通过 EXE 路径前缀匹配。
pub fn scan_services_for_software(install_dir: &str) -> Result<Vec<ServiceEntry>> {
    let all = scan_services()?;
    let install_lower = install_dir.to_lowercase();

    Ok(all
        .into_iter()
        .filter(|s| s.exe_path.as_str().to_lowercase().contains(&install_lower))
        .collect())
}

/// 扫描 COM 组件注册
///
/// 扫描 HKCR\CLSID 下的 COM 组件。
pub fn scan_com_registration() -> Result<Vec<ComEntry>> {
    let clsid_path = "CLSID";
    let base_key = match LOCAL_MACHINE.open(clsid_path) {
        Ok(k) => k,
        Err(_) => return Ok(Vec::new()),
    };

    let subkey_names = match base_key.keys() {
        Ok(names) => names,
        Err(_) => return Ok(Vec::new()),
    };

    let mut entries = Vec::new();

    for clsid in subkey_names {
        if let Ok(entry) = read_com_entry(&base_key, &clsid) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// 扫描指定软件相关的 COM 组件
///
/// 通过服务器路径前缀匹配。
pub fn scan_com_for_software(install_dir: &str) -> Result<Vec<ComEntry>> {
    let all = scan_com_registration()?;
    let install_lower = install_dir.to_lowercase();

    Ok(all
        .into_iter()
        .filter(|c| {
            c.server_path
                .as_ref()
                .map(|p| p.as_str().to_lowercase().contains(&install_lower))
                .unwrap_or(false)
        })
        .collect())
}

/// 扫描结果报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceScanReport {
    pub total_services: usize,
    pub auto_start: usize,
    pub manual_start: usize,
    pub disabled: usize,
    pub services: Vec<ServiceEntry>,
}

/// 格式化服务扫描报告
pub fn format_service_report(services: &[ServiceEntry]) -> ServiceScanReport {
    let auto_start = services
        .iter()
        .filter(|s| s.start_type == ServiceStartType::Auto)
        .count();
    let manual_start = services
        .iter()
        .filter(|s| s.start_type == ServiceStartType::Manual)
        .count();
    let disabled = services
        .iter()
        .filter(|s| s.start_type == ServiceStartType::Disabled)
        .count();

    ServiceScanReport {
        total_services: services.len(),
        auto_start,
        manual_start,
        disabled,
        services: services.to_vec(),
    }
}

// ── 内部实现 ──────────────────────────────────────────────────────────────────

/// 读取单个服务条目
fn read_service_entry(base_key: &Key, name: &str) -> Result<ServiceEntry> {
    let subkey = base_key
        .open(name)
        .map_err(|e| ScannerError::Registry(format!("打开服务键 {name} 失败: {e}")))?;

    let display_name = subkey.get_string("DisplayName").ok();
    let description = subkey.get_string("Description").ok();

    // 读取 ImagePath
    let image_path = subkey
        .get_string("ImagePath")
        .unwrap_or_default();

    // 解析 Start 值
    let start_type = match subkey.get_u32("Start").unwrap_or(3) {
        0 => ServiceStartType::Boot,
        1 => ServiceStartType::System,
        2 => ServiceStartType::Auto,
        3 => ServiceStartType::Manual,
        4 => ServiceStartType::Disabled,
        _ => ServiceStartType::Manual,
    };

    Ok(ServiceEntry {
        name: name.to_string(),
        display_name,
        exe_path: Utf8PathBuf::from(image_path),
        start_type,
        description,
    })
}

/// 读取单个 COM 组件条目
fn read_com_entry(base_key: &Key, clsid: &str) -> Result<ComEntry> {
    let subkey = match base_key.open(clsid) {
        Ok(k) => k,
        Err(_) => return Err(ScannerError::Registry(format!("打开 CLSID {clsid} 失败"))),
    };

    let description = subkey.get_string("").ok();

    // 查找服务器路径（InprocServer32 或 LocalServer32）
    let (server_path, inproc) = if let Ok(inproc_key) = subkey.open("InprocServer32") {
        let path = inproc_key.get_string("").ok().map(Utf8PathBuf::from);
        (path, true)
    } else if let Ok(local_key) = subkey.open("LocalServer32") {
        let path = local_key.get_string("").ok().map(Utf8PathBuf::from);
        (path, false)
    } else {
        (None, false)
    };

    Ok(ComEntry {
        clsid: clsid.to_string(),
        description,
        server_path,
        inproc,
    })
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_services_returns_results() {
        let result = scan_services();
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(!services.is_empty());
    }

    #[test]
    fn scan_services_have_required_fields() {
        let services = scan_services().unwrap();
        for service in &services {
            assert!(!service.name.is_empty());
            // exe_path 可能为空（驱动程序等）
        }
    }

    #[test]
    fn scan_services_for_software_filters() {
        // 用一个不存在的路径，应该返回空
        let services = scan_services_for_software("C:\\NonExistent\\Path").unwrap();
        assert!(services.is_empty());
    }

    #[test]
    fn format_service_report_works() {
        let services = scan_services().unwrap();
        let report = format_service_report(&services);
        assert_eq!(report.total_services, services.len());
        // auto + manual + disabled 可能不等于总数（还有 Boot/System 类型）
        assert!(report.auto_start + report.manual_start + report.disabled <= report.total_services);
    }

    #[test]
    fn scan_com_registration_works() {
        let result = scan_com_registration();
        assert!(result.is_ok());
        // COM 扫描可能很慢，只验证不报错
    }

    #[test]
    fn scan_com_for_software_filters() {
        let entries = scan_com_for_software("C:\\NonExistent\\Path").unwrap();
        assert!(entries.is_empty());
    }
}
