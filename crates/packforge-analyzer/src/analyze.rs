//! 分析编排模块
//!
//! 将扫描、分类、依赖分析、框架识别、可移植性评估组合为完整的分析流程。

use packforge_core::{
    AnalysisResult, FileEntry, RuntimeType, SoftwareProfile,
};

use crate::classifier::{classify_files, ClassifiedFiles};
use crate::framework;
use crate::portability::{assess_portability, PortabilityReport};
use crate::runtime;
use crate::Result;

/// 分析报告（完整版，含分类和评估）
#[derive(Debug)]
pub struct AnalysisReport {
    /// 分析结果（核心数据模型）
    pub result: AnalysisResult,
    /// 分类详情
    pub classified: ClassifiedFiles,
    /// 可移植性报告
    pub portability: PortabilityReport,
    /// 分析耗时
    pub elapsed: std::time::Duration,
}

/// 分析选项
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// 是否计算 SHA-256 哈希
    pub compute_hashes: bool,
    /// 是否解析 PE 依赖
    pub parse_pe_dependencies: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            compute_hashes: false,
            parse_pe_dependencies: true,
        }
    }
}

/// 执行完整分析流程
///
/// 流程：
/// 1. 扫描安装目录
/// 2. 分类文件
/// 3. 检测安装框架
/// 4. 检测运行时环境
/// 5. 评估可移植性
pub fn analyze_software(name: &str, options: &AnalysisOptions) -> Result<AnalysisReport> {
    let start = std::time::Instant::now();

    // 1. 扫描安装目录
    let scan_options = packforge_scanner::ScanOptions {
        compute_hashes: options.compute_hashes,
        parse_pe_dependencies: options.parse_pe_dependencies,
        ..Default::default()
    };
    let scan_report = packforge_scanner::filesystem::scan_software_by_name(name, &scan_options)
        .map_err(|e| crate::AnalyzerError::Io(std::io::Error::other(
            format!("扫描失败: {}", e),
        )))?;

    // 2. 构建 SoftwareProfile
    let install_dir = scan_report.install_dir.clone();
    let software = scan_report.software;
    let files = scan_report.files;

    let profile = SoftwareProfile {
        name: software.name.clone(),
        version: software.version.clone(),
        publisher: software.publisher.clone(),
        install_dir: install_dir.clone(),
        uninstall_string: software.uninstall_string.clone(),
        install_date: software.install_date,
        files: files.clone(),
        registry_entries: Vec::new(), // TODO: T2.1 注册表深度扫描
        services: Vec::new(),         // TODO: T2.3 服务扫描
        com_components: Vec::new(),   // TODO: T2.3 COM 扫描
        env_vars: Vec::new(),
        shortcuts: Vec::new(),
    };

    // 3. 分类文件
    let classified = classify_files(&files);

    // 4. 检测安装框架
    let install_framework = framework::detect_install_framework(
        install_dir.as_std_path(),
        software.uninstall_string.as_deref(),
    );

    // 5. 检测运行时环境
    let runtime_type = detect_primary_runtime(&files, install_dir.as_std_path());

    // 6. 收集外部依赖（非系统 DLL）
    let external_deps = collect_external_deps(&files);

    // 7. 评估可移植性
    let portability = assess_portability(&profile);

    // 8. 构建 AnalysisResult
    let result = AnalysisResult {
        software: profile,
        core_files: classified.core.clone(),
        optional_files: classified.plugins.clone(),
        user_data_files: classified.user_data.clone(),
        temp_files: classified.temp.clone(),
        core_registry: Vec::new(),
        user_registry: Vec::new(),
        external_dependencies: external_deps,
        install_framework,
        runtime: runtime_type,
        portability_score: portability.score,
        portability_issues: portability.issues.clone(),
    };

    let elapsed = start.elapsed();

    Ok(AnalysisReport {
        result,
        classified,
        portability,
        elapsed,
    })
}

/// 检测主要运行时环境
fn detect_primary_runtime(files: &[FileEntry], install_dir: &std::path::Path) -> Option<RuntimeType> {
    // 找到主 EXE 文件（通常是安装目录根目录下的第一个 exe）
    let main_exe = files.iter().find(|f| {
        f.file_type == packforge_core::FileType::Executable
            && !f.path.as_str().to_lowercase().contains("\\unins")
            && !f.path.as_str().to_lowercase().contains("\\uninstall")
    });

    if let Some(exe) = main_exe {
        let exe_path = exe.path.as_std_path();
        Some(runtime::detect_runtime(exe_path, install_dir))
    } else {
        None
    }
}

/// 收集外部依赖（非系统 DLL）
fn collect_external_deps(files: &[FileEntry]) -> Vec<packforge_core::Dependency> {
    use std::collections::HashMap;

    let mut dep_map: HashMap<String, u32> = HashMap::new();

    for file in files {
        for dep in &file.dependencies {
            if !crate::dependencies::is_system_dll(dep) {
                *dep_map.entry(dep.clone()).or_insert(0) += 1;
            }
        }
    }

    dep_map.into_keys().map(|name| packforge_core::Dependency {
            name: name.clone(),
            version: None,
            is_system: false,
            download_url: None,
        })
        .collect()
}

/// 格式化分析报告为人类可读文本
pub fn format_analysis_report(report: &AnalysisReport) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    let r = &report.result;

    // 软件信息
    writeln!(output, "📊 分析报告: {}", r.software.name).unwrap();
    writeln!(output, "{}", "=".repeat(60)).unwrap();
    if let Some(ver) = &r.software.version {
        writeln!(output, "版本:     {}", ver).unwrap();
    }
    if let Some(pub_) = &r.software.publisher {
        writeln!(output, "发布者:   {}", pub_).unwrap();
    }
    writeln!(output, "安装目录: {}", r.software.install_dir).unwrap();
    writeln!(output).unwrap();

    // 安装框架
    writeln!(output, "🔧 安装框架: {:?}", r.install_framework).unwrap();

    // 运行时
    if let Some(rt) = &r.runtime {
        writeln!(output, "⚙️  运行时:   {:?}", rt).unwrap();
    }
    writeln!(output).unwrap();

    // 文件分类统计
    writeln!(output, "📁 文件分类:").unwrap();
    writeln!(output, "  核心文件:     {:>5} 个", r.core_files.len()).unwrap();
    writeln!(output, "  依赖库:       {:>5} 个", report.classified.libraries.len()).unwrap();
    writeln!(output, "  插件:         {:>5} 个", r.optional_files.len()).unwrap();
    writeln!(output, "  配置文件:     {:>5} 个", report.classified.configs.len()).unwrap();
    writeln!(output, "  文档:         {:>5} 个", report.classified.docs.len()).unwrap();
    writeln!(output, "  用户数据:     {:>5} 个", r.user_data_files.len()).unwrap();
    writeln!(output, "  临时文件:     {:>5} 个", r.temp_files.len()).unwrap();
    writeln!(output, "  未分类:       {:>5} 个", report.classified.unknown.len()).unwrap();
    writeln!(output).unwrap();

    // 外部依赖
    if !r.external_dependencies.is_empty() {
        writeln!(output, "📦 外部依赖 ({} 个):", r.external_dependencies.len()).unwrap();
        for dep in &r.external_dependencies {
            writeln!(output, "  - {}", dep.name).unwrap();
        }
        writeln!(output).unwrap();
    }

    // 可移植性评估
    writeln!(output, "🎯 可移植性评分: {:.0}%", report.portability.score * 100.0).unwrap();
    writeln!(
        output,
        "   可便携化: {}",
        if report.portability.can_portable {
            "✅ 是"
        } else {
            "❌ 否"
        }
    )
    .unwrap();

    if !report.portability.issues.is_empty() {
        writeln!(output, "   问题:").unwrap();
        for issue in &report.portability.issues {
            writeln!(output, "     ⚠️  {:?}", issue).unwrap();
        }
    }

    if !report.portability.suggestions.is_empty() {
        writeln!(output, "   建议:").unwrap();
        for sug in &report.portability.suggestions {
            writeln!(output, "     💡 {}", sug).unwrap();
        }
    }

    writeln!(output).unwrap();
    writeln!(output, "⏱️  分析耗时: {}ms", report.elapsed.as_millis()).unwrap();

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_options_default() {
        let opts = AnalysisOptions::default();
        assert!(!opts.compute_hashes);
        assert!(opts.parse_pe_dependencies);
    }
}
