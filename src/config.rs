use crate::types::{LspError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for connecting to Jenkins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Jenkins instance URL (e.g., "https://jenkins.example.com")
    pub jenkins_url: String,
    /// Jenkins username
    pub username: String,
    /// Jenkins API token (preferred over password)
    pub api_token: String,
    /// Whether to skip TLS certificate verification (for self-signed certs)
    #[serde(default)]
    pub insecure: bool,
}

impl Config {
    /// Load configuration from environment variables or config file
    ///
    /// Priority order:
    /// 1. Environment variables (highest priority)
    /// 2. Config file at specified path
    /// 3. Config file at ~/.config/jenkinsfile-ls/config.toml
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        // Try environment variables first
        if let Some(config) = Self::from_env()? {
            return Ok(config);
        }

        // Try specified config file
        if let Some(path) = config_path {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // Try default config file location
        if let Some(config_dir) = dirs::config_dir() {
            let default_path = config_dir.join("jenkinsfile-ls").join("config.toml");
            if default_path.exists() {
                return Self::from_file(&default_path);
            }
        }

        Err(LspError::Config(
            "No configuration found. Set environment variables (JENKINS_URL, JENKINS_USER_ID, JENKINS_API_TOKEN) or create a config file.".to_string()
        ))
    }

    /// Load configuration from environment variables
    fn from_env() -> Result<Option<Self>> {
        let jenkins_url = std::env::var("JENKINS_URL")
            .or_else(|_| std::env::var("JENKINS_HOST"))
            .ok();

        let username = std::env::var("JENKINS_USER_ID")
            .or_else(|_| std::env::var("JENKINS_USERNAME"))
            .ok();

        let api_token = std::env::var("JENKINS_API_TOKEN")
            .or_else(|_| std::env::var("JENKINS_TOKEN"))
            .or_else(|_| std::env::var("JENKINS_PASSWORD"))
            .ok();

        let insecure = std::env::var("JENKINS_INSECURE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        // Only return config if all required fields are present
        match (jenkins_url, username, api_token) {
            (Some(jenkins_url), Some(username), Some(api_token)) => {
                Ok(Some(Self {
                    jenkins_url,
                    username,
                    api_token,
                    insecure,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Load configuration from a TOML file
    fn from_file(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| LspError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate that all required fields are present and valid
    fn validate(&self) -> Result<()> {
        if self.jenkins_url.is_empty() {
            return Err(LspError::Config("jenkins_url cannot be empty".to_string()));
        }
        if self.username.is_empty() {
            return Err(LspError::Config("username cannot be empty".to_string()));
        }
        if self.api_token.is_empty() {
            return Err(LspError::Config("api_token cannot be empty".to_string()));
        }

        // Validate URL format
        if !self.jenkins_url.starts_with("http://")
            && !self.jenkins_url.starts_with("https://")
        {
            return Err(LspError::Config(
                "jenkins_url must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = Config {
            jenkins_url: "https://jenkins.example.com".to_string(),
            username: "user".to_string(),
            api_token: "token123".to_string(),
            insecure: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_url() {
        let config = Config {
            jenkins_url: "not-a-url".to_string(),
            username: "user".to_string(),
            api_token: "token123".to_string(),
            insecure: false,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_fields() {
        let config = Config {
            jenkins_url: "https://jenkins.example.com".to_string(),
            username: "".to_string(),
            api_token: "token123".to_string(),
            insecure: false,
        };
        assert!(config.validate().is_err());
    }
}
