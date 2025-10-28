use crate::diagnostics::parse_jenkins_response;
use crate::jenkins::JenkinsClient;
use crate::types::{LspError, ValidationResult};
use dashmap::DashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// LSP backend for Jenkinsfile validation
pub struct Backend {
    /// LSP client for sending notifications and diagnostics
    client: Client,
    /// Jenkins API client
    jenkins_client: Arc<JenkinsClient>,
    /// Document cache mapping URI to content
    document_map: Arc<DashMap<Url, String>>,
}

impl Backend {
    /// Create a new LSP backend with the given Jenkins client
    pub fn new(client: Client, jenkins_client: JenkinsClient) -> Self {
        Self {
            client,
            jenkins_client: Arc::new(jenkins_client),
            document_map: Arc::new(DashMap::new()),
        }
    }

    /// Validate a document and publish diagnostics
    async fn validate_document(&self, uri: Url) {
        // Get document content from cache
        let content = match self.document_map.get(&uri) {
            Some(content) => content.clone(),
            None => {
                tracing::warn!("Document not found in cache: {}", uri);
                return;
            }
        };

        tracing::info!("Validating document: {}", uri);

        // Perform validation
        match self.jenkins_client.validate(&content).await {
            Ok(ValidationResult::Success) => {
                tracing::info!("Validation successful: {}", uri);
                // Clear diagnostics
                self.client.publish_diagnostics(uri, Vec::new(), None).await;
            }
            Ok(ValidationResult::Error(response)) => {
                tracing::info!("Validation returned errors: {}", uri);
                // Parse errors and publish diagnostics
                let diagnostics = parse_jenkins_response(&response);
                self.client
                    .publish_diagnostics(uri, diagnostics, None)
                    .await;
            }
            Err(LspError::Auth(msg)) => {
                tracing::error!("Authentication error: {}", msg);
                self.client
                    .show_message(
                        MessageType::ERROR,
                        format!("Jenkins authentication failed: {}", msg),
                    )
                    .await;
            }
            Err(e) => {
                tracing::error!("Validation error: {}", e);
                self.client
                    .show_message(MessageType::ERROR, format!("Validation failed: {}", e))
                    .await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("Initializing Jenkinsfile LSP server");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "jenkinsfile-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Jenkinsfile LSP server initialized");
        self.client
            .log_message(MessageType::INFO, "Jenkinsfile LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Jenkinsfile LSP server");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;

        tracing::info!("Document opened: {}", uri);

        // Store document content
        self.document_map.insert(uri.clone(), content);

        // Validate immediately on open
        self.validate_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // Update document content (FULL sync, so we take the last change)
        if let Some(change) = params.content_changes.into_iter().last() {
            tracing::debug!("Document changed: {}", uri);
            self.document_map.insert(uri, change.text);
        }

        // We don't validate on change, only on save
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::info!("Document saved: {}", uri);

        // Validate on save
        self.validate_document(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::info!("Document closed: {}", uri);

        // Remove from cache
        self.document_map.remove(&uri);

        // Clear diagnostics
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}
