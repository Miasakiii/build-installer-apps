//! Analyzer 模块错误类型

#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("PE 解析错误: {0}")]
    PeParse(String),

    #[error("依赖解析错误: {0}")]
    DependencyResolution(String),

    #[error("数据格式错误: {0}")]
    DataFormat(String),
}

pub type Result<T> = std::result::Result<T, AnalyzerError>;
