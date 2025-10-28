use serde::{Deserialize, Serialize};
use thiserror::Error;

/// CSRF crumb returned by Jenkins for authenticated requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crumb {
    /// The crumb value to include in requests
    pub crumb: String,
    /// The header field name (typically "Jenkins-Crumb")
    #[serde(rename = "crumbRequestField")]
    pub crumb_request_field: String,
}

/// Result of validating a Jenkinsfile
#[derive(Debug)]
pub enum ValidationResult {
    /// Jenkinsfile is valid
    Success,
    /// Jenkinsfile has errors with diagnostic information
    Error(String),
}

/// Errors that can occur during LSP operations
#[derive(Error, Debug)]
pub enum LspError {
    #[error("Jenkins API error: {0}")]
    JenkinsApi(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    // we never emit this one
    // #[error("Parse error: {0}")]
    // Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, LspError>;
