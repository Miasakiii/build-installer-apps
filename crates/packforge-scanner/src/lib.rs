//! PackForge Scanner — 文件系统 & 注册表扫描引擎
//!
//! 负责收集已安装软件的完整状态快照：文件、注册表、服务、COM、环境变量。

pub mod discovery;
pub mod error;
pub mod filesystem;
pub mod registry;
pub mod services;
pub mod snapshot;

pub use error::{ScannerError, Result};
pub use filesystem::{ScanOptions, ScanReport};
