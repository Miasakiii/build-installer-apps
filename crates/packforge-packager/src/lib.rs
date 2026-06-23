//! PackForge Packager — 打包输出引擎
//!
//! 根据分析结果生成可安装的包：便携化 ZIP、Inno Setup 脚本。

pub mod error;
pub mod extract;
pub mod inno;
pub mod manifest;
pub mod portable;

pub use error::{PackagerError, Result};
pub use extract::{extract_software, ExtractConfig, ExtractReport};
pub use portable::{PackageReport};
