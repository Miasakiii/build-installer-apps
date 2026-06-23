//! `report` 子命令 — 导出软件清单报告

use crate::utils::*;
use crate::OutputFormat;
use colored::*;

/// 解析软件名称列表（从命令行参数或文件）
pub fn resolve_names(names: &[String], file: Option<&str>) -> anyhow::Result<Vec<String>> {
    let mut result: Vec<String> = names.to_vec();

    if let Some(path) = file {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("读取文件失败: {e}"))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                result.push(trimmed.to_string());
            }
        }
    }

    if result.is_empty() {
        return Err(anyhow::anyhow!("未指定软件名称，请通过参数或 --file 指定"));
    }

    Ok(result)
}

pub fn cmd_report(output: Option<&str>, report_format: &str, output_format: OutputFormat) -> anyhow::Result<()> {
    print_banner("📋", "导出软件清单报告");

    let pb = create_spinner("正在扫描已安装软件...");
    let installed = packforge_scanner::discovery::list_installed_software()
        .map_err(|e| anyhow::anyhow!("扫描注册表失败: {e}"))?;
    finish_spinner(&pb, &format!("发现 {} 个软件", installed.len()));

    // 过滤系统组件
    let software: Vec<_> = installed.iter().filter(|s| !s.is_system_component).collect();

    // 验证输出路径
    if let Some(path) = output {
        validate_output_path(path)?;
    }

    match report_format {
        "json" => {
            let json_output = serde_json::json!({
                "total": software.len(),
                "software": software.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "version": s.version,
                        "publisher": s.publisher,
                        "install_dir": s.install_dir,
                        "estimated_size_kb": s.estimated_size_kb,
                    })
                }).collect::<Vec<_>>(),
            });
            let content = serde_json::to_string_pretty(&json_output)?;

            if let Some(path) = output {
                std::fs::write(path, &content)?;
                print_success(&format!("报告已保存到: {path}"));
            } else {
                println!("{content}");
            }
        }
        "csv" => {
            // 直接写入文件或标准输出，避免中间缓冲区
            if let Some(path) = output {
                let mut wtr = csv::Writer::from_path(path)?;
                wtr.write_record(["name", "version", "publisher", "install_dir", "estimated_size_kb"])?;
                for s in &software {
                    wtr.write_record([
                        &s.name,
                        s.version.as_deref().unwrap_or(""),
                        s.publisher.as_deref().unwrap_or(""),
                        s.install_dir.as_ref().map(|d| d.as_str()).unwrap_or(""),
                        &s.estimated_size_kb.map(|kb| kb.to_string()).unwrap_or_default(),
                    ])?;
                }
                wtr.flush()?;
                print_success(&format!("报告已保存到: {path}"));
            } else {
                let mut wtr = csv::Writer::from_writer(std::io::stdout());
                wtr.write_record(["name", "version", "publisher", "install_dir", "estimated_size_kb"])?;
                for s in &software {
                    wtr.write_record([
                        &s.name,
                        s.version.as_deref().unwrap_or(""),
                        s.publisher.as_deref().unwrap_or(""),
                        s.install_dir.as_ref().map(|d| d.as_str()).unwrap_or(""),
                        &s.estimated_size_kb.map(|kb| kb.to_string()).unwrap_or_default(),
                    ])?;
                }
                wtr.flush()?;
            }
        }
        _ => {
            return Err(anyhow::anyhow!("不支持的格式: {report_format}，请使用 json 或 csv"));
        }
    }

    // Table 摘要
    if output_format == OutputFormat::Table {
        println!();
        println!("  {}", "📊 软件清单摘要:".bold());
        print_stat("已安装软件总数", &software.len().to_string());
        if let Some(path) = output {
            print_stat("输出文件", path);
        }
        println!();

        // 显示前 10 个软件
        println!("  {}", "前 10 个软件:".dimmed());
        for (i, s) in software.iter().take(10).enumerate() {
            let version = s.version.as_deref().unwrap_or("-");
            println!("    {}. {} v{}", i + 1, s.name.cyan(), version.dimmed());
        }
        if software.len() > 10 {
            println!("    {} 还有 {} 个软件...", "...".dimmed(), software.len() - 10);
        }
        println!();
    }

    Ok(())
}
