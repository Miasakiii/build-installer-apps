//! `scan` 子命令 — 深度扫描软件

use crate::utils::*;
use crate::OutputFormat;
use colored::*;

pub fn cmd_scan(
    name: &str,
    output: Option<&str>,
    compute_hashes: bool,
    no_pe: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let pb = create_spinner(&format!("正在扫描 {name}..."));

    let options = packforge_scanner::ScanOptions {
        compute_hashes,
        parse_pe_dependencies: !no_pe,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let report = packforge_scanner::filesystem::scan_software_by_name(name, &options)?;
    let elapsed = start.elapsed();

    finish_spinner(
        &pb,
        &format!(
            "扫描完成: {} 个文件, {}",
            report.total_files,
            packforge_scanner::filesystem::format_size(report.total_size)
        ),
    );

    match format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "software": {
                    "name": report.software.name,
                    "version": report.software.version,
                    "publisher": report.software.publisher,
                    "install_dir": report.install_dir,
                },
                "scan": {
                    "total_files": report.total_files,
                    "total_size": report.total_size,
                    "total_size_human": packforge_scanner::filesystem::format_size(report.total_size),
                    "elapsed_ms": report.elapsed.as_millis(),
                },
                "files": report.files.iter().map(|f| {
                    let mut entry = serde_json::json!({
                        "path": f.path,
                        "size": f.size,
                        "size_human": packforge_scanner::filesystem::format_size(f.size),
                        "type": format!("{:?}", f.file_type),
                        "is_executable": f.is_executable,
                    });
                    if let Some(hash) = &f.hash {
                        entry["hash"] = serde_json::json!(hash.iter().map(|b| format!("{:02x}", b)).collect::<String>());
                    }
                    if !f.dependencies.is_empty() {
                        entry["dependencies"] = serde_json::json!(f.dependencies);
                    }
                    entry
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            print_banner("🔍", &format!("扫描报告: {}", report.software.name));
            print_info(
                "版本",
                report.software.version.as_deref().unwrap_or("-"),
            );
            print_info(
                "发布者",
                report.software.publisher.as_deref().unwrap_or("-"),
            );
            print_info("安装目录", report.install_dir.as_str());
            println!();

            // 扫描统计
            print_stat(
                "文件总数",
                &report.total_files.to_string(),
            );
            print_stat(
                "总大小",
                &packforge_scanner::filesystem::format_size(report.total_size),
            );
            print_stat("耗时", &format_elapsed(elapsed));
            println!();

            // 文件类型统计
            let mut type_counts = std::collections::HashMap::new();
            for f in &report.files {
                *type_counts.entry(format!("{:?}", f.file_type)).or_insert(0) += 1;
            }
            println!("  {}", "文件类型分布:".cyan().bold());
            let mut sorted: Vec<_> = type_counts.into_iter().collect();
            sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
            for (ft, count) in &sorted {
                println!("    {:<20} {}", ft.dimmed(), count.to_string().green());
            }

            // PE 依赖列表（去重）
            let all_deps: std::collections::HashSet<String> = report
                .files
                .iter()
                .flat_map(|f| f.dependencies.iter().cloned())
                .collect();
            if !all_deps.is_empty() {
                println!();
                println!(
                    "  {}",
                    format!("DLL 依赖 ({} 个):", all_deps.len()).cyan().bold()
                );
                let mut deps: Vec<_> = all_deps.into_iter().collect();
                deps.sort();
                for dep in &deps {
                    let is_system = packforge_analyzer::dependencies::is_system_dll(dep);
                    if is_system {
                        println!("    {} {}", dep.dimmed(), "(系统)".dimmed());
                    } else {
                        println!("    {}", dep.yellow());
                    }
                }
            }

            // 大文件列表（Top 10）
            let mut largest: Vec<_> = report.files.iter().collect();
            largest.sort_by_key(|b| std::cmp::Reverse(b.size));
            println!();
            println!("  {}", "最大文件 (Top 10):".cyan().bold());
            for f in largest.iter().take(10) {
                let size_str = packforge_scanner::filesystem::format_size(f.size);
                let size_colored = if f.size > 10 * 1024 * 1024 {
                    size_str.red().to_string()
                } else if f.size > 1024 * 1024 {
                    size_str.yellow().to_string()
                } else {
                    size_str.green().to_string()
                };
                println!("    {:>12}  {}", size_colored, f.path.as_str().dimmed());
            }

            // 输出到文件
            if let Some(out) = output {
                let json = serde_json::to_string_pretty(&report.files)?;
                std::fs::write(out, &json)?;
                print_success(&format!("扫描结果已保存到: {out}"));
            }

            println!();
        }
        OutputFormat::Yaml => {
            println!("# YAML 输出尚未实现，使用 --format json");
            let json_output = serde_json::json!({
                "software": report.software,
                "total_files": report.total_files,
                "total_size": report.total_size,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
    }

    Ok(())
}
