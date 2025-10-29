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
    /// Document cache mapping URI to (content, version)
    document_map: Arc<DashMap<Url, (String, i32)>>,
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
        // Get document content and version from cache (snapshot)
        let (content, version) = match self.document_map.get(&uri) {
            Some(entry) => entry.clone(),
            None => {
                tracing::warn!("Document not found in cache: {}", uri);
                return;
            }
        };

        tracing::info!("Validating document: {} (version {})", uri, version);

        // Perform validation
        match self.jenkins_client.validate(&content).await {
            Ok(ValidationResult::Success) => {
                tracing::info!("Validation successful: {}", uri);

                // Check if document version is still current before publishing
                if let Some(current) = self.document_map.get(&uri)
                    && current.1 != version
                {
                    tracing::debug!(
                        "Discarding stale diagnostics for {} (validated v{}, current v{})",
                        uri,
                        version,
                        current.1
                    );
                    return;
                }

                // Clear diagnostics
                self.client
                    .publish_diagnostics(uri, Vec::new(), Some(version))
                    .await;
            }
            Ok(ValidationResult::Error(response)) => {
                tracing::info!("Validation returned errors: {}", uri);

                // Check if document version is still current before publishing
                if let Some(current) = self.document_map.get(&uri)
                    && current.1 != version
                {
                    tracing::debug!(
                        "Discarding stale diagnostics for {} (validated v{}, current v{})",
                        uri,
                        version,
                        current.1
                    );
                    return;
                }

                // Parse errors and publish diagnostics
                let diagnostics = parse_jenkins_response(&response);
                self.client
                    .publish_diagnostics(uri, diagnostics, Some(version))
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
        let version = params.text_document.version;

        tracing::info!("Document opened: {} (version {})", uri, version);

        // Store document content and version
        self.document_map.insert(uri.clone(), (content, version));

        // Validate immediately on open
        self.validate_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // Update document content (FULL sync, so we take the last change)
        if let Some(change) = params.content_changes.into_iter().last() {
            tracing::debug!("Document changed: {} (version {})", uri, version);
            self.document_map.insert(uri, (change.text, version));
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
