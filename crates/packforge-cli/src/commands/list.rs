//! `list` 子命令 — 列出已安装软件

use crate::utils::*;
use crate::OutputFormat;
use colored::*;

pub fn cmd_list(
    query: Option<String>,
    _show_system: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let pb = create_spinner("正在扫描注册表...");

    let start = std::time::Instant::now();
    let software = match query {
        Some(ref q) => packforge_scanner::discovery::search_software(q)?,
        None => packforge_scanner::discovery::list_installed_software()?,
    };
    let elapsed = start.elapsed();

    finish_spinner(&pb, &format!("发现 {} 个软件", software.len()));

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&software)?);
        }
        OutputFormat::Table => {
            println!();
            println!(
                "  {:<50} {:<15} {:<30}",
                "名称".cyan().bold(),
                "版本".cyan().bold(),
                "发布者".cyan().bold()
            );
            println!("{}", "─".repeat(95).dimmed());
            for s in &software {
                println!(
                    "  {:<50} {:<15} {:<30}",
                    truncate(&s.name, 48),
                    s.version.as_deref().unwrap_or("-").dimmed(),
                    s.publisher.as_deref().unwrap_or("-").dimmed()
                );
            }
            println!();
            print_stat("总数", &software.len().to_string());
            print_info("耗时", &format_elapsed(elapsed));
            println!();
        }
        OutputFormat::Yaml => {
            println!("# YAML 输出尚未实现，使用 --format json");
            println!("{}", serde_json::to_string_pretty(&software)?);
        }
    }

    Ok(())
}
