//! `batch-extract` 和 `batch-package` 子命令 — 批量操作

use crate::utils::*;
use crate::{OutputFormat, PackageFormat};
use colored::*;

pub fn cmd_batch_extract(
    names: &[String],
    output: &str,
    zip: bool,
    output_format: OutputFormat,
) -> anyhow::Result<()> {
    print_banner("📦", "批量提取软件");

    // 验证输出路径
    let output_dir = validate_output_path(output)?;
    std::fs::create_dir_all(&output_dir)?;

    let mut results = Vec::new();
    let total = names.len();

    for (i, name) in names.iter().enumerate() {
        println!();
        println!("  {} [{}/{}] 正在处理: {}", "▶".cyan(), i + 1, total, name.bold());

        let pb = create_spinner("正在分析...");
        let analysis_options = packforge_analyzer::AnalysisOptions {
            compute_hashes: false,
            parse_pe_dependencies: true,
        };

        match packforge_analyzer::analyze_software(name, &analysis_options) {
            Ok(report) => {
                finish_spinner(&pb, "分析完成");

                let pb = create_spinner("正在提取...");
                let extract_options = packforge_core::ExtractOptions {
                    include_optional: false,
                    include_docs: false,
                    create_zip: zip,
                    launcher_type: packforge_core::LauncherType::Batch,
                };

                let package_output_dir = output_dir.join(&report.result.software.name);
                match packforge_packager::portable::create_portable_package(
                    &report.result,
                    &package_output_dir,
                    &extract_options,
                ) {
                    Ok(package) => {
                        finish_spinner(&pb, "提取完成");
                        results.push(serde_json::json!({
                            "name": name,
                            "status": STATUS_SUCCESS,
                            "output_path": package.output_path,
                            "total_files": package.total_files,
                            "total_size": package.total_size,
                        }));
                        print_success(&format!("已提取到: {}", package.output_path));
                    }
                    Err(e) => {
                        finish_spinner(&pb, "提取失败");
                        results.push(serde_json::json!({
                            "name": name,
                            "status": STATUS_ERROR,
                            "error": e.to_string(),
                        }));
                        print_error(&format!("提取失败: {e}"));
                    }
                }
            }
            Err(e) => {
                finish_spinner(&pb, "分析失败");
                results.push(serde_json::json!({
                    "name": name,
                    "status": STATUS_ERROR,
                    "error": e.to_string(),
                }));
                print_error(&format!("分析失败: {e}"));
            }
        }
    }

    // 输出摘要
    match output_format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "total": total,
                "success": results.iter().filter(|r| r["status"] == STATUS_SUCCESS).count(),
                "failed": results.iter().filter(|r| r["status"] == STATUS_ERROR).count(),
                "output_dir": output,
                "results": results,
            });
            println!();
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            println!();
            println!("  {}", "📊 批量提取摘要:".bold());
            print_stat("总数", &total.to_string());
            print_stat("成功", &results.iter().filter(|r| r["status"] == STATUS_SUCCESS).count().to_string());
            print_stat("失败", &results.iter().filter(|r| r["status"] == STATUS_ERROR).count().to_string());
            print_stat("输出目录", output);
            println!();
            print_success("批量提取完成！");
            println!();
        }
        OutputFormat::Yaml => {
            return Err(anyhow::anyhow!("YAML 输出暂未实现，请使用 --format json"));
        }
    }

    Ok(())
}

pub fn cmd_batch_package(
    names: &[String],
    format: PackageFormat,
    output: &str,
    no_compile: bool,
    output_format: OutputFormat,
) -> anyhow::Result<()> {
    if format == PackageFormat::Nsis || format == PackageFormat::Wix {
        let fmt_str = match format {
            PackageFormat::Nsis => "NSIS",
            PackageFormat::Wix => "WiX/MSI",
            _ => unreachable!(),
        };
        return Err(anyhow::anyhow!("{fmt_str} 格式暂未实现，请使用 --format inno"));
    }

    print_banner("🛠️", "批量生成安装程序");

    // 验证输出路径
    let output_dir = validate_output_path(output)?;
    std::fs::create_dir_all(&output_dir)?;

    let mut results = Vec::new();
    let total = names.len();

    for (i, name) in names.iter().enumerate() {
        println!();
        println!("  {} [{}/{}] 正在处理: {}", "▶".cyan(), i + 1, total, name.bold());

        let pb = create_spinner("正在分析...");
        let analysis_options = packforge_analyzer::AnalysisOptions {
            compute_hashes: false,
            parse_pe_dependencies: true,
        };

        match packforge_analyzer::analyze_software(name, &analysis_options) {
            Ok(report) => {
                finish_spinner(&pb, "分析完成");

                let pb = create_spinner("正在构建清单...");
                let build_options = packforge_packager::manifest::PackageBuildOptions::default();

                match packforge_packager::manifest::build_manifest(&report, &build_options) {
                    Ok(manifest) => {
                        finish_spinner(&pb, "清单构建完成");

                        let pb = create_spinner("正在生成脚本...");
                        let app_name = manifest.name.clone();
                        let iss_path = output_dir.join(format!("{app_name}.iss"));

                        match packforge_packager::inno::save_inno_script(&manifest, &iss_path) {
                            Ok(_) => {
                                finish_spinner(&pb, "脚本生成完成");
                                results.push(serde_json::json!({
                                    "name": name,
                                    "app_name": app_name,
                                    "status": STATUS_SUCCESS,
                                    "script_path": iss_path.display().to_string(),
                                }));
                                print_success(&format!("脚本已保存: {}", iss_path.display()));
                            }
                            Err(e) => {
                                finish_spinner(&pb, "生成失败");
                                results.push(serde_json::json!({
                                    "name": name,
                                    "status": STATUS_ERROR,
                                    "error": e.to_string(),
                                }));
                                print_error(&format!("生成失败: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        finish_spinner(&pb, "清单构建失败");
                        results.push(serde_json::json!({
                            "name": name,
                            "status": STATUS_ERROR,
                            "error": e.to_string(),
                        }));
                        print_error(&format!("清单构建失败: {e}"));
                    }
                }
            }
            Err(e) => {
                finish_spinner(&pb, "分析失败");
                results.push(serde_json::json!({
                    "name": name,
                    "status": STATUS_ERROR,
                    "error": e.to_string(),
                }));
                print_error(&format!("分析失败: {e}"));
            }
        }
    }

    // 输出摘要
    match output_format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "total": total,
                "success": results.iter().filter(|r| r["status"] == STATUS_SUCCESS).count(),
                "failed": results.iter().filter(|r| r["status"] == STATUS_ERROR).count(),
                "output_dir": output,
                "no_compile": no_compile,
                "results": results,
            });
            println!();
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            println!();
            println!("  {}", "📊 批量打包摘要:".bold());
            print_stat("总数", &total.to_string());
            print_stat("成功", &results.iter().filter(|r| r["status"] == STATUS_SUCCESS).count().to_string());
            print_stat("失败", &results.iter().filter(|r| r["status"] == STATUS_ERROR).count().to_string());
            print_stat("输出目录", output);
            if no_compile {
                print_stat("编译", "仅生成脚本");
            }
            println!();
            print_success("批量打包完成！");
            println!();
        }
        OutputFormat::Yaml => {
            return Err(anyhow::anyhow!("YAML 输出暂未实现，请使用 --format json"));
        }
    }

    Ok(())
}
