//! `package` 子命令 — 生成安装程序

use crate::utils::*;
use crate::{OutputFormat, PackageFormat};
use colored::*;

#[allow(clippy::too_many_arguments)]
pub fn cmd_package(
    name: &str,
    pkg_format: PackageFormat,
    app_name: Option<&str>,
    publisher: Option<&str>,
    app_version: Option<&str>,
    output: Option<&str>,
    no_compile: bool,
    _no_desktop_shortcut: bool,
    _no_start_menu: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    if pkg_format == PackageFormat::Nsis || pkg_format == PackageFormat::Wix {
        let fmt_str = match pkg_format {
            PackageFormat::Nsis => "NSIS",
            PackageFormat::Wix => "WiX/MSI",
            _ => unreachable!(),
        };
        return Err(anyhow::anyhow!("{fmt_str} 格式暂未实现，请使用 --format inno"));
    }

    print_banner("🛠️", &format!("生成安装程序: {name}"));

    // Step 1: 分析
    let pb = create_spinner("正在分析软件...");
    let analysis_options = packforge_analyzer::AnalysisOptions {
        compute_hashes: false,
        parse_pe_dependencies: true,
    };
    let report = packforge_analyzer::analyze_software(name, &analysis_options)?;
    finish_spinner(&pb, "分析完成");

    // Step 2: 构建清单
    let pb = create_spinner("正在构建打包清单...");
    let build_options = packforge_packager::manifest::PackageBuildOptions::default();
    let mut manifest = packforge_packager::manifest::build_manifest(&report, &build_options)?;

    // 应用用户覆盖
    if let Some(aname) = app_name {
        manifest.name = aname.to_string();
    }
    if let Some(pub_name) = publisher {
        manifest.publisher = Some(pub_name.to_string());
    }
    if let Some(ver) = app_version {
        manifest.version = ver.to_string();
    }
    finish_spinner(&pb, "清单构建完成");

    // Step 3: 生成脚本
    let pb = create_spinner("正在生成 Inno Setup 脚本...");
    let output_dir = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("./output"));
    std::fs::create_dir_all(&output_dir)?;
    let iss_path = output_dir.join(format!("{}.iss", manifest.name));
    packforge_packager::inno::save_inno_script(&manifest, &iss_path)?;
    finish_spinner(&pb, "脚本生成完成");

    // Step 4: 编译（可选）
    if !no_compile {
        let pb = create_spinner("正在编译安装程序...");
        let compile_options = packforge_packager::inno::InnoCompileOptions::default();
        match packforge_packager::inno::compile_inno_setup(&iss_path, &compile_options) {
            Ok(exe_path) => {
                finish_spinner(&pb, "编译完成");
                print_success(&format!("安装程序已生成: {}", exe_path));
            }
            Err(e) => {
                finish_spinner(&pb, "编译失败");
                print_warning(&format!("编译失败: {e}"));
                print_info("脚本路径", &iss_path.display().to_string());
                println!("  {} 手动编译: iscc.exe {}", "💡".dimmed(), iss_path.display());
            }
        }
    } else {
        print_info("脚本路径", &iss_path.display().to_string());
        println!("  {} 跳过编译（--no-compile）", "ℹ".blue());
    }

    match format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "software": {
                    "name": report.result.software.name,
                    "version": report.result.software.version,
                },
                "manifest": {
                    "name": manifest.name,
                    "version": manifest.version,
                    "publisher": manifest.publisher,
                    "total_files": manifest.entries.len(),
                },
                "script_path": iss_path.display().to_string(),
                "compiled": !no_compile,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Table => {
            println!();
            print_banner("📋", "打包摘要");
            print_info("应用名称", &manifest.name);
            print_info("版本", &manifest.version);
            print_info("发布者", manifest.publisher.as_deref().unwrap_or("-"));
            print_info("文件数", &manifest.entries.len().to_string());
            print_info("脚本", &iss_path.display().to_string());
            println!();
        }
        OutputFormat::Yaml => {
            println!("# YAML 输出尚未实现，使用 --format json");
        }
    }

    Ok(())
}
