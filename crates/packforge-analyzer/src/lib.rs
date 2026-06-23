//! PackForge Analyzer — 智能分析引擎
//!
//! 负责：文件分类、依赖检测、安装框架识别、运行时识别、可移植性评估、用户数据剥离。

pub mod analyze;
pub mod classifier;
pub mod dependencies;
pub mod error;
pub mod framework;
pub mod portability;
pub mod runtime;

pub use analyze::{analyze_software, AnalysisOptions, AnalysisReport};
pub use error::{AnalyzerError, Result};
