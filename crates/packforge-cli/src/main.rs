//! PackForge CLI — 安装包提取器 & 安装程序生成器
//!
//! 命令行入口，提供 list / scan / analyze / extract / package / snapshot / diff 命令。
//!
//! # 快速开始
//!
//! ```bash
//! # 列出所有已安装软件
//! packforge list
//!
//! # 搜索特定软件
//! packforge list --query "7-Zip"
//!
//! # 深度扫描
//! packforge scan "7-Zip"
//!
//! # 分析可移植性
//! packforge analyze "7-Zip"
//!
//! # 提取为便携化包
//! packforge extract "7-Zip" --output ./output
//!
//! # 打包为 ZIP
//! packforge extract "7-Zip" --zip
//! ```

mod commands;
mod utils;

use clap::{Parser, Subcommand};
use colored::*;
use tracing_subscriber::EnvFilter;

use commands::*;
use utils::{format_elapsed, print_error};

// ── CLI 定义 ──────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "packforge",
    about = "📦 PackForge — 安装包提取器 & 安装程序生成器",
    version,
    long_about = "从已安装软件的状态出发，逆向生成可分发的安装程序或便携化包。\n\
                   解决\"装完就删，要用时又得重新找\"的痛点。\n\n\
                   🚀 快速开始:\n\
                   packforge list                    # 列出已安装软件\n\
                   packforge extract \"7-Zip\"          # 提取为便携包\n\
                   packforge analyze \"Git\"             # 分析可移植性",
    after_help = "示例:\n  \
                  packforge list --query \"Visual\"     # 搜索含 Visual 的软件\n  \
                  packforge scan \"7-Zip\" --hash       # 扫描并计算哈希\n  \
                  packforge extract \"Git\" --zip --hash # 提取并打包为 ZIP",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 输出格式
    ///
    /// table: 人类可读的表格格式（默认）
    /// json:  结构化 JSON 输出（适合脚本/管道）
    /// yaml:  YAML 格式输出
    #[arg(short, long, default_value = "table", global = true, value_enum)]
    format: OutputFormat,

    /// 详细输出（显示调试信息）
    #[arg(short, long, global = true)]
    verbose: bool,

    /// 禁用彩色输出
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 📋 列出已安装软件
    ///
    /// 从 Windows 注册表中读取已安装软件列表。
    /// 支持按名称/发布者模糊搜索。
    ///
    /// 示例:
    ///   packforge list
    ///   packforge list --query "7-Zip"
    ///   packforge list --show-system
    List {
        /// 搜索关键词（模糊匹配名称/发布者）
        #[arg(short, long)]
        query: Option<String>,

        /// 显示系统组件（默认隐藏 Windows 更新和系统组件）
        #[arg(long)]
        show_system: bool,
    },

    /// 🔍 深度扫描指定软件
    ///
    /// 扫描软件安装目录，收集文件清单、类型、大小等信息。
    /// 可选计算 SHA-256 哈希和解析 PE 依赖。
    ///
    /// 示例:
    ///   packforge scan "7-Zip"
    ///   packforge scan "7-Zip" --hash
    ///   packforge scan "7-Zip" --output scan.json
    Scan {
        /// 软件名称（支持模糊匹配）
        name: String,

        /// 将扫描结果保存为 JSON 文件
        #[arg(short, long)]
        output: Option<String>,

        /// 计算文件 SHA-256 哈希（大文件会较慢）
        #[arg(long)]
        hash: bool,

        /// 不解析 PE 文件的 DLL 依赖（加快扫描速度）
        #[arg(long)]
        no_pe: bool,
    },

    /// 📊 分析软件结构 & 可移植性
    ///
    /// 深入分析软件的安装框架、运行时、文件分类、可移植性评分。
    /// 分析结果用于决定如何打包。
    ///
    /// 示例:
    ///   packforge analyze "7-Zip"
    ///   packforge analyze "Git" --hash
    Analyze {
        /// 软件名称（支持模糊匹配）
        name: String,

        /// 仅分析，不执行后续打包操作
        #[arg(long)]
        dry_run: bool,

        /// 计算文件 SHA-256 哈希
        #[arg(long)]
        hash: bool,
    },

    /// 📦 提取为便携化包
    ///
    /// 将已安装软件提取为可在其他机器上运行的便携化包。
    /// 自动分析、过滤用户数据、生成启动脚本。
    ///
    /// 示例:
    ///   packforge extract "7-Zip"
    ///   packforge extract "7-Zip" --output ./my-packages
    ///   packforge extract "7-Zip" --zip --hash
    Extract {
        /// 软件名称（支持模糊匹配）
        name: String,

        /// 输出目录（默认: 当前目录）
        #[arg(short, long)]
        output: Option<String>,

        /// 包含可选组件（文档、示例等）
        #[arg(long)]
        include_optional: bool,

        /// 打包为 ZIP 压缩包
        #[arg(long)]
        zip: bool,

        /// 计算文件 SHA-256 哈希
        #[arg(long)]
        hash: bool,
    },

    /// 🛠️ 生成安装程序
    ///
    /// 完整流程：扫描 → 分析 → 生成 manifest → 生成脚本 → 编译
    ///
    /// 示例:
    ///   packforge package "7-Zip"
    ///   packforge package "7-Zip" --pkg-format inno --app-name "7-Zip" --publisher "Igor Pavlov"
    ///   packforge package "7-Zip" --no-compile（只生成脚本，不编译）
    Package {
        /// 软件名称（支持模糊匹配）
        name: String,

        /// 安装程序格式
        #[arg(long = "pkg-format", default_value = "inno", value_enum)]
        pkg_format: PackageFormat,

        /// 应用名称（覆盖检测到的名称）
        #[arg(long)]
        app_name: Option<String>,

        /// 发布者/作者（覆盖检测到的发布者）
        #[arg(long)]
        publisher: Option<String>,

        /// 应用版本（覆盖检测到的版本）
        #[arg(long)]
        app_version: Option<String>,

        /// 输出目录（默认: ./output）
        #[arg(short, long)]
        output: Option<String>,

        /// 只生成脚本，不调用编译器
        #[arg(long)]
        no_compile: bool,

        /// 不创建桌面快捷方式
        #[arg(long)]
        no_desktop_shortcut: bool,

        /// 不创建开始菜单快捷方式
        #[arg(long)]
        no_start_menu: bool,
    },

    /// 📸 拍摄系统快照
    ///
    /// 记录当前系统状态（注册表、文件系统），用于安装前后对比。
    /// 可以精确定位软件安装时创建的所有文件和注册表项。
    ///
    /// 示例:
    ///   packforge snapshot --output before.json
    ///   # ... 安装软件 ...
    ///   packforge snapshot --output after.json
    ///   packforge diff before.json after.json
    Snapshot {
        /// 快照输出路径
        #[arg(short, long)]
        output: String,
    },

    /// 🔄 对比两次快照差异
    ///
    /// 对比安装前后的系统快照，找出新增的文件、注册表项和服务。
    ///
    /// 示例:
    ///   packforge diff before.json after.json
    Diff {
        /// 安装前的快照文件
        before: String,
        /// 安装后的快照文件
        after: String,
    },

    /// 📋 导出软件清单报告
    ///
    /// 列出所有已安装软件并导出为 JSON/CSV 报告。
    ///
    /// 示例:
    ///   packforge report
    ///   packforge report --output software-list.json
    ///   packforge report --format csv --output software-list.csv
    Report {
        /// 输出文件路径
        #[arg(short, long)]
        output: Option<String>,

        /// 报告格式 (json, csv)
        #[arg(long, default_value = "json")]
        report_format: String,
    },

    /// 📦 批量提取多个软件
    ///
    /// 批量提取多个已安装软件为便携化包。
    ///
    /// 示例:
    ///   packforge batch-extract "7-Zip" "Git" "Node.js"
    ///   packforge batch-extract --list-file software-list.txt --output ./packages
    BatchExtract {
        /// 软件名称列表
        names: Vec<String>,

        /// 从文件读取软件名称（每行一个）
        #[arg(long = "list-file")]
        list_file: Option<String>,

        /// 输出目录
        #[arg(short, long, default_value = "./packages")]
        output: String,

        /// 同时打包为 ZIP
        #[arg(long)]
        zip: bool,
    },

    /// 🛠️ 批量生成安装程序
    ///
    /// 批量为多个软件生成安装程序脚本。
    ///
    /// 示例:
    ///   packforge batch-package "7-Zip" "Git" "Node.js"
    ///   packforge batch-package --list-file software-list.txt --output ./installers
    BatchPackage {
        /// 软件名称列表
        names: Vec<String>,

        /// 从文件读取软件名称（每行一个）
        #[arg(long = "list-file")]
        list_file: Option<String>,

        /// 安装程序格式
        #[arg(long = "pkg-format", default_value = "inno", value_enum)]
        pkg_format: PackageFormat,

        /// 输出目录
        #[arg(short, long, default_value = "./installers")]
        output: String,

        /// 只生成脚本，不编译
        #[arg(long)]
        no_compile: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum PackageFormat {
    /// Inno Setup 脚本 (.iss)
    Inno,
    /// NSIS 脚本 (.nsi) — 暂未实现
    Nsis,
    /// WiX 项目 (.wxs) — 暂未实现
    Wix,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    /// 人类可读的表格格式
    Table,
    /// 结构化 JSON 输出
    Json,
    /// YAML 格式输出
    Yaml,
}

// ── 入口 ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // 如果指定了 --no-color，禁用彩色
    if cli.no_color {
        colored::control::set_override(false);
    }

    // 初始化日志（仅在 --verbose 时显示调试信息）
    let filter = if cli.verbose {
        EnvFilter::new("packforge=debug")
    } else {
        EnvFilter::new("packforge=warn")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let start = std::time::Instant::now();

    let result = match cli.command {
        Commands::List { query, show_system } => list::cmd_list(query, show_system, cli.format),
        Commands::Scan { name, output, hash, no_pe } => scan::cmd_scan(&name, output.as_deref(), hash, no_pe, cli.format),
        Commands::Analyze { name, dry_run, hash } => analyze::cmd_analyze(&name, dry_run, hash, cli.format),
        Commands::Extract {
            name,
            output,
            include_optional,
            zip,
            hash,
        } => extract::cmd_extract(&name, output.as_deref(), include_optional, zip, hash, cli.format),
        Commands::Package {
            name,
            pkg_format,
            app_name,
            publisher,
            app_version,
            output,
            no_compile,
            no_desktop_shortcut,
            no_start_menu,
        } => package::cmd_package(
            &name,
            pkg_format,
            app_name.as_deref(),
            publisher.as_deref(),
            app_version.as_deref(),
            output.as_deref(),
            no_compile,
            no_desktop_shortcut,
            no_start_menu,
            cli.format,
        ),
        Commands::Snapshot { output } => snapshot::cmd_snapshot(&output),
        Commands::Diff { before, after } => diff::cmd_diff(&before, &after),
        Commands::Report { output, report_format } => report::cmd_report(output.as_deref(), &report_format, cli.format),
        Commands::BatchExtract { names, list_file, output, zip } => {
            match report::resolve_names(&names, list_file.as_deref()) {
                Ok(names) => batch::cmd_batch_extract(&names, &output, zip, cli.format),
                Err(e) => Err(e),
            }
        }
        Commands::BatchPackage { names, list_file, pkg_format, output, no_compile } => {
            match report::resolve_names(&names, list_file.as_deref()) {
                Ok(names) => batch::cmd_batch_package(&names, pkg_format, &output, no_compile, cli.format),
                Err(e) => Err(e),
            }
        }
    };

    match result {
        Ok(()) => {
            if cli.verbose {
                println!(
                    "\n  {} 总耗时: {}",
                    "⏱".dimmed(),
                    format_elapsed(start.elapsed()).dimmed()
                );
            }
        }
        Err(e) => {
            print_error(&format!("{e}"));

            // 显示错误链
            if cli.verbose {
                for cause in e.chain().skip(1) {
                    eprintln!("    {} {}", "↳".dimmed(), format!("{cause}").dimmed());
                }
            } else {
                eprintln!(
                    "    {} 使用 {} 查看详细错误信息",
                    "↳".dimmed(),
                    "--verbose".yellow()
                );
            }

            std::process::exit(1);
        }
    }
}
