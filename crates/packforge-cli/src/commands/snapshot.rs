//! `snapshot` 子命令 — 拍摄系统快照

use crate::utils::*;

pub fn cmd_snapshot(output: &str) -> anyhow::Result<()> {
    print_banner("📸", "拍摄系统快照");

    let pb = create_spinner("正在记录系统状态...");

    let start = std::time::Instant::now();
    let snapshot = packforge_scanner::snapshot::take_snapshot()?;
    let elapsed = start.elapsed();

    finish_spinner(&pb, &format!(
        "快照完成: {} 个软件, {} 个目录, {} 个环境变量",
        snapshot.software.len(),
        snapshot.program_files.len(),
        snapshot.environment.len()
    ));

    let json = serde_json::to_string_pretty(&snapshot)?;
    std::fs::write(output, &json)?;

    print_info("已安装软件", &snapshot.software.len().to_string());
    print_info("Program Files 目录", &snapshot.program_files.len().to_string());
    print_info("环境变量", &snapshot.environment.len().to_string());
    print_info("耗时", &format_elapsed(elapsed));
    print_success(&format!("快照已保存到: {output}"));
    println!();

    Ok(())
}
