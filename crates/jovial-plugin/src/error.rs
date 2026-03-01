use thiserror::Error;

/// Errors that can occur during plugin execution.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("transform failed: {0}")]
    TransformFailed(String),

    #[error("unsupported node: {0}")]
    UnsupportedNode(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("walk error: {0}")]
    WalkError(String),

    #[error("{0}")]
    Other(String),
}

/// A diagnostic message emitted by a plugin.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}
