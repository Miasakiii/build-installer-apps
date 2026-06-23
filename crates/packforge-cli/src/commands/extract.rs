//! `extract` 子命令 — 提取为便携化包

use crate::utils::*;
use crate::OutputFormat;
use colored::*;

pub fn cmd_extract(
    name: &str,
    output: Option<&str>,
    include_optional: bool,
    zip: bool,
    compute_hashes: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let pb = create_spinner(&format!("正在提取 {name}..."));

    let output_dir = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

    let config = packforge_packager::ExtractConfig {
        options: packforge_core::ExtractOptions {
            include_optional,
            create_zip: zip,
            ..Default::default()
        },
        compute_hashes,
    };

    let start = std::time::Instant::now();
    let report = packforge_packager::extract_software(name, &output_dir, &config)?;
    let elapsed = start.elapsed();

    finish_spinner(&pb, &format!("提取完成: {} 个文件", report.package.total_files));

    match format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "extract": {
                    "total_files": report.package.total_files,
                    "total_size": report.package.total_size,
                    "total_size_human": packforge_scanner::filesystem::format_size(report.package.total_size),
                    "output_path": report.package.output_path,
                    "main_exe": report.package.main_exe,
                    "elapsed_ms": elapsed.as_millis(),
                },
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            print_banner("📦", "提取报告");
            println!();

            print_stat("文件总数", &report.package.total_files.to_string());
            print_stat(
                "总大小",
                &packforge_scanner::filesystem::format_size(report.package.total_size),
            );
            print_stat("输出目录", report.package.output_path.as_str());
            if let Some(ref exe) = report.package.main_exe {
                print_info("主程序", exe);
            }
            print_stat("耗时", &format_elapsed(elapsed));
            println!();

            // 警告信息
            if !report.package.warnings.is_empty() {
                println!("  {}", "警告:".yellow().bold());
                for w in &report.package.warnings {
                    print_warning(w);
                }
                println!();
            }

            print_success(&format!("便携包已提取到: {}", report.package.output_path));
            println!();
        }
        OutputFormat::Yaml => {
            println!("# YAML 输出尚未实现，使用 --format json");
            let json_output = serde_json::json!({
                "total_files": report.package.total_files,
                "output_path": report.package.output_path,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
    }

    Ok(())
}
