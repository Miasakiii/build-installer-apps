//! CLI 工具函数
//!
//! 进度条、彩色输出、时间格式化、路径验证等辅助函数。

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

// ── 常量 ──────────────────────────────────────────────────────────────────────

pub const STATUS_SUCCESS: &str = "success";
pub const STATUS_ERROR: &str = "error";

// ── 进度条辅助 ────────────────────────────────────────────────────────────────

/// 创建一个 spinner 进度条，用于不确定时长的操作
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// 完成 spinner 并显示成功消息
pub fn finish_spinner(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(format!("{} {}", "✓".green().bold(), message));
}

// ── 彩色输出辅助 ──────────────────────────────────────────────────────────────

/// 打印带图标的信息横幅
pub fn print_banner(icon: &str, text: &str) {
    println!("\n{} {}", icon, text.cyan().bold());
    println!("{}", "─".repeat(60).dimmed());
}

/// 打印键值对信息
pub fn print_info(key: &str, value: &str) {
    println!("  {:<16} {}", format!("{key}:").dimmed(), value);
}

/// 打印统计数字（带颜色）
pub fn print_stat(label: &str, value: &str) {
    println!("  {:<16} {}", format!("{label}:").dimmed(), value.green().bold());
}

/// 打印警告信息
pub fn print_warning(msg: &str) {
    eprintln!("  {} {}", "⚠".yellow().bold(), msg.yellow());
}

/// 打印错误信息（用户友好）
pub fn print_error(msg: &str) {
    eprintln!("\n  {} {}", "✗".red().bold(), msg.red().bold());
}

/// 打印成功信息
pub fn print_success(msg: &str) {
    println!("\n  {} {}", "✓".green().bold(), msg.green().bold());
}

// ── 时间格式化 ────────────────────────────────────────────────────────────────

/// 格式化耗时为人类可读的格式
pub fn format_elapsed(elapsed: std::time::Duration) -> String {
    let ms = elapsed.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", elapsed.as_secs_f64())
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{mins}m{secs}s")
    }
}

// ── 工具函数 ──────────────────────────────────────────────────────────────────

/// 验证输出路径安全性（防止路径遍历攻击）
pub fn validate_output_path(path: &str) -> anyhow::Result<std::path::PathBuf> {
    let path_buf = std::path::PathBuf::from(path);

    // 检查路径组件是否包含 ..
    for component in path_buf.components() {
        if let std::path::Component::ParentDir = component {
            return Err(anyhow::anyhow!("路径不允许包含 '..'，请使用相对路径或绝对路径"));
        }
    }

    // 如果是相对路径，转换为绝对路径
    let canonical = if path_buf.is_absolute() {
        path_buf.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("获取当前目录失败: {e}"))?
            .join(&path_buf)
    };

    Ok(canonical)
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("a long string here", 10), "a long st…");
    }

    #[test]
    fn format_elapsed_milliseconds() {
        let d = std::time::Duration::from_millis(42);
        assert_eq!(format_elapsed(d), "42ms");
    }

    #[test]
    fn format_elapsed_seconds() {
        let d = std::time::Duration::from_millis(1500);
        assert_eq!(format_elapsed(d), "1.5s");
    }

    #[test]
    fn format_elapsed_minutes() {
        let d = std::time::Duration::from_secs(125);
        assert_eq!(format_elapsed(d), "2m5s");
    }
}
