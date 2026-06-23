//! Packager 模块错误类型

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PackagerError {
    #[error("IO 错误: {path:?} — {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("IO 错误: {0}")]
    IoSimple(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("模板渲染错误: {0}")]
    Template(String),

    #[error("压缩错误: {0}")]
    Compression(String),

    #[error("ZIP 错误: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("编译错误: {0}")]
    Compile(String),

    #[error("配置错误: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, PackagerError>;
