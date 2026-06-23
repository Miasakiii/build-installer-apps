//! `analyze` 子命令 — 分析软件结构 & 可移植性

use crate::utils::*;
use crate::OutputFormat;
use colored::*;

pub fn cmd_analyze(name: &str, dry_run: bool, compute_hashes: bool, format: OutputFormat) -> anyhow::Result<()> {
    let pb = create_spinner(&format!("正在分析 {name}..."));

    let options = packforge_analyzer::AnalysisOptions {
        compute_hashes,
        parse_pe_dependencies: true,
    };

    let start = std::time::Instant::now();
    let report = packforge_analyzer::analyze_software(name, &options)?;
    let elapsed = start.elapsed();

    finish_spinner(&pb, &format!("分析完成 (可移植性: {:.0}%)", report.result.portability_score * 100.0));

    match format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "software": {
                    "name": report.result.software.name,
                    "version": report.result.software.version,
                    "publisher": report.result.software.publisher,
                    "install_dir": report.result.software.install_dir,
                },
                "analysis": {
                    "install_framework": format!("{:?}", report.result.install_framework),
                    "runtime": report.result.runtime.as_ref().map(|r| format!("{:?}", r)),
                    "portability_score": report.result.portability_score,
                    "can_portable": report.portability.can_portable,
                    "issues": report.portability.issues.iter().map(|i| format!("{:?}", i)).collect::<Vec<_>>(),
                    "suggestions": report.portability.suggestions,
                },
                "files": {
                    "core": report.result.core_files.len(),
                    "libraries": report.classified.libraries.len(),
                    "plugins": report.result.optional_files.len(),
                    "configs": report.classified.configs.len(),
                    "docs": report.classified.docs.len(),
                    "user_data": report.result.user_data_files.len(),
                    "temp": report.result.temp_files.len(),
                    "unknown": report.classified.unknown.len(),
                },
                "external_dependencies": report.result.external_dependencies.iter().map(|d| &d.name).collect::<Vec<_>>(),
                "elapsed_ms": report.elapsed.as_millis(),
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            print_banner("📊", &format!("分析报告: {}", report.result.software.name));
            print_info(
                "版本",
                report.result.software.version.as_deref().unwrap_or("-"),
            );
            print_info(
                "发布者",
                report.result.software.publisher.as_deref().unwrap_or("-"),
            );
            print_info("安装目录", report.result.software.install_dir.as_str());
            println!();

            // 安装框架
            println!("  {}", "安装框架:".cyan().bold());
            println!("    {}", format!("{:?}", report.result.install_framework).yellow());
            if let Some(ref runtime) = report.result.runtime {
                println!("    运行时: {}", format!("{runtime:?}").yellow());
            }

            // 可移植性评分
            println!();
            println!("  {}", "可移植性:".cyan().bold());
            let score = report.result.portability_score;
            let score_str = if score >= 0.8 {
                format!("{:.0}%", score * 100.0).green().bold().to_string()
            } else if score >= 0.5 {
                format!("{:.0}%", score * 100.0).yellow().bold().to_string()
            } else {
                format!("{:.0}%", score * 100.0).red().bold().to_string()
            };
            print_stat("评分", &score_str);

            if report.portability.can_portable {
                println!(
                    "  {} {}",
                    "✓".green().bold(),
                    "可生成便携化包".green()
                );
            } else {
                println!(
                    "  {} {}",
                    "⚠".yellow().bold(),
                    "便携化可能不完整".yellow()
                );
                for issue in &report.portability.issues {
                    print_warning(&format!("{issue:?}"));
                }
            }

            for suggestion in &report.portability.suggestions {
                println!("    {} {}", "💡".dimmed(), suggestion.dimmed());
            }

            // 文件分类统计
            println!();
            println!("  {}", "文件分类:".cyan().bold());
            let categories = [
                ("核心文件", report.result.core_files.len()),
                ("库文件", report.classified.libraries.len()),
                ("插件", report.result.optional_files.len()),
                ("配置", report.classified.configs.len()),
                ("文档", report.classified.docs.len()),
                ("用户数据", report.result.user_data_files.len()),
                ("临时文件", report.result.temp_files.len()),
                ("未分类", report.classified.unknown.len()),
            ];
            for (label, count) in &categories {
                if *count > 0 {
                    println!("    {:<16} {}", format!("{label}:").dimmed(), count.to_string().green());
                }
            }

            // 外部依赖
            if !report.result.external_dependencies.is_empty() {
                println!();
                println!(
                    "  {}",
                    format!("外部依赖 ({} 个):", report.result.external_dependencies.len()).cyan().bold()
                );
                for dep in &report.result.external_dependencies {
                    println!("    {}", dep.name.yellow());
                }
            }

            // 耗时
            println!();
            print_info("耗时", &format_elapsed(elapsed));
            println!();
        }
        OutputFormat::Yaml => {
            println!("# YAML 输出尚未实现，使用 --format json");
            let json_output = serde_json::json!({
                "software": report.result.software.name,
                "portability_score": report.result.portability_score,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
    }

    if dry_run {
        println!("  {} 仅分析模式，不执行打包", "ℹ".blue().bold());
        println!();
    }

    Ok(())
}
