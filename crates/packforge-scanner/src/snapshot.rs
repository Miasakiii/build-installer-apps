//! 系统快照 & Diff 引擎
//!
//! 拍摄系统状态快照，用于安装前后对比。
//! 通过对比两次快照，可以精确识别软件安装时创建的所有资源。

use crate::error::{Result, ScannerError};
use camino::Utf8PathBuf;
use packforge_core::SoftwareInfo;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

// ── 快照数据结构 ──────────────────────────────────────────────────────────────

/// 系统快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// 快照时间戳（ISO 8601）
    pub timestamp: String,
    /// 已安装软件列表
    pub software: Vec<SoftwareInfo>,
    /// Program Files 目录下的条目（目录名 → 路径）
    pub program_files: HashMap<String, Utf8PathBuf>,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 快照耗时（毫秒）
    pub elapsed_ms: u64,
}

/// 快照差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDiff {
    /// 新增的软件
    pub added_software: Vec<SoftwareInfo>,
    /// 删除的软件
    pub removed_software: Vec<SoftwareInfo>,
    /// 新增的目录
    pub added_directories: Vec<Utf8PathBuf>,
    /// 删除的目录
    pub removed_directories: Vec<Utf8PathBuf>,
    /// 新增的环境变量
    pub added_environment: Vec<EnvVarChange>,
    /// 修改的环境变量
    pub modified_environment: Vec<EnvVarChange>,
    /// 差异统计
    pub summary: DiffSummary,
}

/// 环境变量变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarChange {
    pub name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// 差异统计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub software_added: usize,
    pub software_removed: usize,
    pub directories_added: usize,
    pub directories_removed: usize,
    pub env_vars_added: usize,
    pub env_vars_modified: usize,
}

// ── 公共 API ──────────────────────────────────────────────────────────────────

/// 拍摄系统快照
///
/// 记录当前系统状态：已安装软件、Program Files 目录、环境变量。
pub fn take_snapshot() -> Result<SystemSnapshot> {
    let start = Instant::now();

    // 1. 枚举已安装软件
    let software = crate::discovery::list_installed_software()?;

    // 2. 扫描 Program Files 目录
    let program_files = scan_program_files()?;

    // 3. 读取环境变量
    let environment = scan_environment();

    let elapsed = start.elapsed();

    Ok(SystemSnapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        software,
        program_files,
        environment,
        elapsed_ms: elapsed.as_millis() as u64,
    })
}

/// 对比两次快照差异
pub fn diff_snapshots(before: &SystemSnapshot, after: &SystemSnapshot) -> SystemDiff {
    // 软件差异
    let before_names: HashSet<&str> = before.software.iter().map(|s| s.name.as_str()).collect();
    let after_names: HashSet<&str> = after.software.iter().map(|s| s.name.as_str()).collect();

    let added_software: Vec<SoftwareInfo> = after
        .software
        .iter()
        .filter(|s| !before_names.contains(s.name.as_str()))
        .cloned()
        .collect();

    let removed_software: Vec<SoftwareInfo> = before
        .software
        .iter()
        .filter(|s| !after_names.contains(s.name.as_str()))
        .cloned()
        .collect();

    // 目录差异
    let before_dirs: HashSet<&str> = before.program_files.keys().map(|s| s.as_str()).collect();
    let after_dirs: HashSet<&str> = after.program_files.keys().map(|s| s.as_str()).collect();

    let added_directories: Vec<Utf8PathBuf> = after
        .program_files
        .iter()
        .filter(|(name, _)| !before_dirs.contains(name.as_str()))
        .map(|(_, path)| path.clone())
        .collect();

    let removed_directories: Vec<Utf8PathBuf> = before
        .program_files
        .iter()
        .filter(|(name, _)| !after_dirs.contains(name.as_str()))
        .map(|(_, path)| path.clone())
        .collect();

    // 环境变量差异
    let mut added_environment = Vec::new();
    let mut modified_environment = Vec::new();

    for (name, new_value) in &after.environment {
        match before.environment.get(name) {
            None => {
                added_environment.push(EnvVarChange {
                    name: name.clone(),
                    old_value: None,
                    new_value: Some(new_value.clone()),
                });
            }
            Some(old_value) => {
                if old_value != new_value {
                    modified_environment.push(EnvVarChange {
                        name: name.clone(),
                        old_value: Some(old_value.clone()),
                        new_value: Some(new_value.clone()),
                    });
                }
            }
        }
    }

    let summary = DiffSummary {
        software_added: added_software.len(),
        software_removed: removed_software.len(),
        directories_added: added_directories.len(),
        directories_removed: removed_directories.len(),
        env_vars_added: added_environment.len(),
        env_vars_modified: modified_environment.len(),
    };

    SystemDiff {
        added_software,
        removed_software,
        added_directories,
        removed_directories,
        added_environment,
        modified_environment,
        summary,
    }
}

/// 格式化差异报告为人类可读的文本
pub fn format_diff_report(diff: &SystemDiff) -> String {
    let mut out = String::new();

    out.push_str("🔄 快照差异报告\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');

    // 摘要
    out.push_str(&format!(
        "  新增软件: {}  |  删除软件: {}\n",
        diff.summary.software_added, diff.summary.software_removed
    ));
    out.push_str(&format!(
        "  新增目录: {}  |  删除目录: {}\n",
        diff.summary.directories_added, diff.summary.directories_removed
    ));
    out.push_str(&format!(
        "  新增环境变量: {}  |  修改环境变量: {}\n",
        diff.summary.env_vars_added, diff.summary.env_vars_modified
    ));

    // 新增软件
    if !diff.added_software.is_empty() {
        out.push_str("\n📦 新增软件:\n");
        for s in &diff.added_software {
            out.push_str(&format!(
                "  + {} ({})\n",
                s.name,
                s.version.as_deref().unwrap_or("?")
            ));
        }
    }

    // 删除软件
    if !diff.removed_software.is_empty() {
        out.push_str("\n🗑️  删除软件:\n");
        for s in &diff.removed_software {
            out.push_str(&format!(
                "  - {} ({})\n",
                s.name,
                s.version.as_deref().unwrap_or("?")
            ));
        }
    }

    // 新增目录
    if !diff.added_directories.is_empty() {
        out.push_str("\n📁 新增目录:\n");
        for d in &diff.added_directories {
            out.push_str(&format!("  + {d}\n"));
        }
    }

    // 环境变量变化
    if !diff.added_environment.is_empty() {
        out.push_str("\n🔧 新增环境变量:\n");
        for env in &diff.added_environment {
            out.push_str(&format!("  + {} = {}\n", env.name, env.new_value.as_deref().unwrap_or("")));
        }
    }

    if !diff.modified_environment.is_empty() {
        out.push_str("\n🔧 修改环境变量:\n");
        for env in &diff.modified_environment {
            out.push_str(&format!(
                "  ~ {} : {} → {}\n",
                env.name,
                env.old_value.as_deref().unwrap_or(""),
                env.new_value.as_deref().unwrap_or("")
            ));
        }
    }

    out
}

// ── 内部实现 ──────────────────────────────────────────────────────────────────

/// 扫描 Program Files 目录
fn scan_program_files() -> Result<HashMap<String, Utf8PathBuf>> {
    let mut entries = HashMap::new();

    let program_files_dirs = [
        std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string()),
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string()),
    ];

    for dir in &program_files_dirs {
        let path = std::path::Path::new(dir);
        if !path.exists() {
            continue;
        }

        let read_dir = std::fs::read_dir(path).map_err(|e| ScannerError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        for entry in read_dir.flatten() {
            let file_type = entry.file_type().unwrap_or_else(|_| {
                entry
                    .metadata()
                    .map(|m| m.file_type())
                    .unwrap_or_else(|_| entry.file_type().unwrap())
            });

            if file_type.is_dir()
                && let Some(name) = entry.file_name().to_str()
                && let Ok(utf8_path) = Utf8PathBuf::try_from(entry.path())
            {
                entries.insert(name.to_string(), utf8_path);
            }
        }
    }

    Ok(entries)
}

/// 扫描环境变量
///
/// 安全说明：过滤掉可能包含敏感信息的环境变量，
/// 如密码、密钥、令牌等。
fn scan_environment() -> HashMap<String, String> {
    // 敏感环境变量名称模式（不区分大小写）
    let sensitive_patterns = [
        "password", "passwd", "pwd", "secret", "token", "key",
        "auth", "credential", "api_key", "apikey", "access_key",
        "private", "signing", "encryption", "decrypt",
    ];

    let mut env = HashMap::new();
    for (key, value) in std::env::vars() {
        let key_lower = key.to_lowercase();

        // 检查是否是敏感变量
        let is_sensitive = sensitive_patterns.iter().any(|pattern| {
            key_lower.contains(pattern)
        });

        // 如果不是敏感变量，则包含在快照中
        if !is_sensitive {
            env.insert(key, value);
        }
    }
    env
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_snapshot_returns_data() {
        let result = take_snapshot();
        assert!(result.is_ok());
        let snapshot = result.unwrap();
        assert!(!snapshot.software.is_empty());
        assert!(!snapshot.program_files.is_empty());
        assert!(!snapshot.environment.is_empty());
    }

    #[test]
    fn diff_identical_snapshots_is_empty() {
        let snapshot = take_snapshot().unwrap();
        let diff = diff_snapshots(&snapshot, &snapshot);

        assert_eq!(diff.summary.software_added, 0);
        assert_eq!(diff.summary.software_removed, 0);
        assert_eq!(diff.summary.directories_added, 0);
        assert_eq!(diff.summary.directories_removed, 0);
        assert_eq!(diff.summary.env_vars_added, 0);
        assert_eq!(diff.summary.env_vars_modified, 0);
    }

    #[test]
    fn diff_detects_added_software() {
        let before = take_snapshot().unwrap();
        let mut after = before.clone();

        // 添加一个假软件
        after.software.push(SoftwareInfo {
            name: "TestDiffSoftware".to_string(),
            version: Some("1.0".to_string()),
            publisher: Some("Test".to_string()),
            install_dir: None,
            uninstall_string: None,
            install_date: None,
            estimated_size_kb: None,
            is_system_component: false,
        });

        let diff = diff_snapshots(&before, &after);
        assert_eq!(diff.summary.software_added, 1);
        assert_eq!(diff.added_software[0].name, "TestDiffSoftware");
    }

    #[test]
    fn diff_detects_removed_software() {
        let before = take_snapshot().unwrap();
        let mut after = before.clone();

        // 移除第一个软件
        if !before.software.is_empty() {
            let removed_name = before.software[0].name.clone();
            after.software.remove(0);

            let diff = diff_snapshots(&before, &after);
            assert_eq!(diff.summary.software_removed, 1);
            assert_eq!(diff.removed_software[0].name, removed_name);
        }
    }

    #[test]
    fn diff_detects_added_env_var() {
        let before = take_snapshot().unwrap();
        let mut after = before.clone();

        // 添加一个假环境变量
        after.environment.insert(
            "PACKFORGE_TEST_VAR".to_string(),
            "test_value".to_string(),
        );

        let diff = diff_snapshots(&before, &after);
        assert_eq!(diff.summary.env_vars_added, 1);
        assert_eq!(diff.added_environment[0].name, "PACKFORGE_TEST_VAR");
    }

    #[test]
    fn format_diff_report_works() {
        let snapshot = take_snapshot().unwrap();
        let diff = diff_snapshots(&snapshot, &snapshot);
        let report = format_diff_report(&diff);
        assert!(report.contains("快照差异报告"));
    }
}
