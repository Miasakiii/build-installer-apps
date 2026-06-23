//! 提取编排模块
//!
//! 将扫描、分析、打包组合为完整的提取流程。

use std::path::Path;

use packforge_core::ExtractOptions;

use crate::portable::{create_portable_package, PackageReport};
use crate::Result;

/// 提取报告
#[derive(Debug)]
pub struct ExtractReport {
    /// 打包报告
    pub package: PackageReport,
    /// 分析耗时
    pub analysis_elapsed: std::time::Duration,
    /// 打包耗时
    pub packaging_elapsed: std::time::Duration,
    /// 总耗时
    pub total_elapsed: std::time::Duration,
}

/// 提取选项
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ExtractConfig {
    /// 打包选项
    pub options: ExtractOptions,
    /// 是否计算 SHA-256 哈希
    pub compute_hashes: bool,
}


/// 执行完整的提取流程
///
/// 流程：
/// 1. 扫描软件（注册表 + 文件系统）
/// 2. 分析文件分类 + 依赖 + 可移植性
/// 3. 创建便携化包
pub fn extract_software(
    name: &str,
    output_dir: &Path,
    config: &ExtractConfig,
) -> Result<ExtractReport> {
    let total_start = std::time::Instant::now();

    // 1. 分析软件
    let analysis_start = std::time::Instant::now();

    let analysis_options = packforge_analyzer::AnalysisOptions {
        compute_hashes: config.compute_hashes,
        parse_pe_dependencies: true,
    };

    let analysis_report = packforge_analyzer::analyze_software(name, &analysis_options)
        .map_err(|e| {
            crate::PackagerError::Config(format!("分析软件失败: {}", e))
        })?;

    let analysis_elapsed = analysis_start.elapsed();

    // 2. 检查可移植性
    if !analysis_report.portability.can_portable {
        return Err(crate::PackagerError::Config(format!(
            "软件 {} 可移植性评分过低 ({:.0}%)，不建议便携化。使用 --force 强制提取。",
            name,
            analysis_report.portability.score * 100.0
        )));
    }

    // 3. 创建便携化包
    let packaging_start = std::time::Instant::now();

    let package_report = create_portable_package(
        &analysis_report.result,
        output_dir,
        &config.options,
    )?;

    let packaging_elapsed = packaging_start.elapsed();
    let total_elapsed = total_start.elapsed();

    Ok(ExtractReport {
        package: package_report,
        analysis_elapsed,
        packaging_elapsed,
        total_elapsed,
    })
}

/// 格式化提取报告为人类可读文本
pub fn format_extract_report(report: &ExtractReport) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    writeln!(output, "📦 提取完成!").unwrap();
    writeln!(output, "{}", "=".repeat(60)).unwrap();
    writeln!(output).unwrap();

    writeln!(output, "📁 输出目录: {}", report.package.output_path).unwrap();
    if let Some(exe) = &report.package.main_exe {
        writeln!(output, "🚀 主程序:   {}", exe).unwrap();
    }
    writeln!(output).unwrap();

    writeln!(output, "📊 统计:").unwrap();
    writeln!(output, "  文件数量: {} 个", report.package.total_files).unwrap();
    writeln!(
        output,
        "  总大小:   {}",
        packforge_scanner::filesystem::format_size(report.package.total_size)
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(output, "⏱️  耗时:").unwrap();
    writeln!(
        output,
        "  分析: {}ms",
        report.analysis_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        output,
        "  打包: {}ms",
        report.packaging_elapsed.as_millis()
    )
    .unwrap();
    writeln!(
        output,
        "  总计: {}ms",
        report.total_elapsed.as_millis()
    )
    .unwrap();

    if !report.package.warnings.is_empty() {
        writeln!(output).unwrap();
        writeln!(output, "⚠️  警告:").unwrap();
        for warning in &report.package.warnings {
            writeln!(output, "  - {}", warning).unwrap();
        }
    }

    writeln!(output).unwrap();
    writeln!(output, "💡 使用方法:").unwrap();
    writeln!(
        output,
        "  Windows: 双击 {}\\launcher.bat",
        report.package.output_path
    )
    .unwrap();
    writeln!(
        output,
        "  Linux:   chmod +x {}/launcher.sh && ./launcher.sh",
        report.package.output_path
    )
    .unwrap();

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_config_default() {
        let config = ExtractConfig::default();
        assert!(!config.compute_hashes);
        assert!(!config.options.create_zip);
        assert!(!config.options.include_optional);
    }
}
