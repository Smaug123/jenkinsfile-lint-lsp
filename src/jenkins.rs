use crate::config::Config;
use crate::types::{Crumb, LspError, Result, ValidationResult};
use reqwest::{Client, multipart};
use std::time::Duration;

/// Jenkins API client for validating Jenkinsfiles
pub struct JenkinsClient {
    config: Config,
    client: Client,
}

impl JenkinsClient {
    /// Create a new Jenkins client with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(config.insecure)
            .build()?;

        Ok(Self { config, client })
    }

    /// Fetch CSRF crumb from Jenkins
    ///
    /// The crumb is required for POST requests to Jenkins to prevent CSRF attacks.
    /// Some Jenkins instances may not require a crumb if CSRF protection is disabled.
    pub async fn get_crumb(&self) -> Result<Crumb> {
        let url = format!("{}/crumbIssuer/api/json", self.config.jenkins_url);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .send()
            .await?;

        if response.status().is_success() {
            let crumb: Crumb = response.json().await?;
            Ok(crumb)
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(LspError::Auth(
                "Authentication failed. Check your credentials.".to_string(),
            ))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(LspError::JenkinsApi(
                "Crumb issuer endpoint not found. CSRF protection may be disabled.".to_string(),
            ))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(LspError::JenkinsApi(format!(
                "Failed to get crumb: {} - {}",
                status, body
            )))
        }
    }

    /// Validate a Jenkinsfile by sending it to Jenkins
    ///
    /// Returns the raw response text from Jenkins which can be parsed for errors.
    pub async fn validate_jenkinsfile(&self, content: &str, crumb: &Crumb) -> Result<String> {
        let url = format!(
            "{}/pipeline-model-converter/validate",
            self.config.jenkins_url
        );

        // Create multipart form with Jenkinsfile content
        let form = multipart::Form::new().text("jenkinsfile", content.to_string());

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .header(&crumb.crumb_request_field, &crumb.crumb)
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            Ok(body)
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(LspError::Auth(
                "Authentication failed during validation.".to_string(),
            ))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(LspError::JenkinsApi(
                "Validation endpoint not found. Ensure pipeline-model-definition plugin is installed.".to_string(),
            ))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(LspError::JenkinsApi(format!(
                "Validation request failed: {} - {}",
                status, body
            )))
        }
    }

    /// Validate a Jenkinsfile and return a ValidationResult
    ///
    /// This is a convenience method that combines getting the crumb and validating.
    pub async fn validate(&self, content: &str) -> Result<ValidationResult> {
        // Try to get crumb, but continue if it fails (some Jenkins instances don't require it)
        let crumb = match self.get_crumb().await {
            Ok(crumb) => crumb,
            Err(LspError::JenkinsApi(_)) => {
                // If crumb endpoint doesn't exist, try with empty crumb
                Crumb {
                    crumb: String::new(),
                    crumb_request_field: "Jenkins-Crumb".to_string(),
                }
            }
            Err(e) => return Err(e),
        };

        let response = self.validate_jenkinsfile(content, &crumb).await?;

        if response.contains("Jenkinsfile successfully validated.") {
            Ok(ValidationResult::Success)
        } else {
            Ok(ValidationResult::Error(response))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jenkins_client_creation() {
        let config = Config {
            jenkins_url: "https://jenkins.example.com".to_string(),
            username: "test".to_string(),
            api_token: "token".to_string(),
            insecure: false,
        };

        let client = JenkinsClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_jenkins_client_with_insecure() {
        let config = Config {
            jenkins_url: "https://jenkins.example.com".to_string(),
            username: "test".to_string(),
            api_token: "token".to_string(),
            insecure: true,
        };

        let client = JenkinsClient::new(config);
        assert!(client.is_ok());
    }
}
