//! PackForge Core 错误类型

use std::path::PathBuf;

/// Core 模块统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("IO 错误: {path:?} — {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("无效输入: {0}")]
    InvalidInput(String),

    #[error("未找到: {0}")]
    NotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("平台不支持: {0}")]
    UnsupportedPlatform(String),
}

/// Core 模块 Result 别名
pub type Result<T> = std::result::Result<T, CoreError>;

impl CoreError {
    /// 包装 IO 错误并附带路径上下文
    pub fn with_path(source: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
