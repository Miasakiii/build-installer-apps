//! Scanner 模块错误类型

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("IO 错误: {path:?} — {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("注册表访问错误: {0}")]
    Registry(String),

    #[error("软件未找到: {0}")]
    SoftwareNotFound(String),

    #[error("PE 解析错误: {0}")]
    PeParse(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),
}

pub type Result<T> = std::result::Result<T, ScannerError>;

impl ScannerError {
    pub fn io(source: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
