mod config;
mod diagnostics;
mod jenkins;
mod server;
mod types;

use config::Config;
use jenkins::JenkinsClient;
use server::Backend;
use tower_lsp::{LspService, Server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jenkinsfile_ls=info,tower_lsp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Starting jenkinsfile-ls v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = match Config::load(None) {
        Ok(config) => {
            tracing::info!("Configuration loaded successfully");
            tracing::debug!("Jenkins URL: {}", config.jenkins_url);
            tracing::debug!("Username: {}", config.username);
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("\nPlease set the following environment variables:");
            eprintln!(
                "  JENKINS_URL         - Jenkins instance URL (e.g., https://jenkins.example.com)"
            );
            eprintln!("  JENKINS_USER_ID     - Jenkins username");
            eprintln!("  JENKINS_API_TOKEN   - Jenkins API token");
            eprintln!("\nOptional:");
            eprintln!("  JENKINS_INSECURE    - Set to '1' or 'true' to skip TLS verification");
            eprintln!("\nOr create a config file at: ~/.config/jenkinsfile-ls/config.toml");
            std::process::exit(1);
        }
    };

    // Create Jenkins client
    let jenkins_client = match JenkinsClient::new(config) {
        Ok(client) => {
            tracing::info!("Jenkins client initialized");
            client
        }
        Err(e) => {
            eprintln!("Failed to initialize Jenkins client: {}", e);
            std::process::exit(1);
        }
    };

    // Create LSP service
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client, jenkins_client));

    tracing::info!("LSP server starting on stdio");

    Server::new(stdin, stdout, socket).serve(service).await;

    tracing::info!("LSP server shutting down");
}
