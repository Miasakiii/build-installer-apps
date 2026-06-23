//! `diff` 子命令 — 对比快照差异

use crate::utils::*;

pub fn cmd_diff(before_path: &str, after_path: &str) -> anyhow::Result<()> {
    print_banner("🔄", "对比快照差异");

    let pb = create_spinner("正在读取快照文件...");

    let before_json = std::fs::read_to_string(before_path)
        .map_err(|e| anyhow::anyhow!("读取快照文件失败 {}: {e}", before_path))?;
    let after_json = std::fs::read_to_string(after_path)
        .map_err(|e| anyhow::anyhow!("读取快照文件失败 {}: {e}", after_path))?;

    let before: packforge_scanner::snapshot::SystemSnapshot = serde_json::from_str(&before_json)
        .map_err(|e| anyhow::anyhow!("解析快照文件失败 {}: {e}", before_path))?;
    let after: packforge_scanner::snapshot::SystemSnapshot = serde_json::from_str(&after_json)
        .map_err(|e| anyhow::anyhow!("解析快照文件失败 {}: {e}", after_path))?;

    let diff = packforge_scanner::snapshot::diff_snapshots(&before, &after);

    finish_spinner(&pb, "对比完成");

    // 使用库层的格式化输出
    let report = packforge_scanner::snapshot::format_diff_report(&diff);
    println!("{report}");

    // 统计摘要
    println!();
    print_info("快照 1", before_path);
    print_info("快照 2", after_path);
    println!();

    Ok(())
}
