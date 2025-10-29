# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

jenkinsfile-ls is a Language Server Protocol (LSP) implementation that validates Jenkinsfiles by communicating with a remote Jenkins instance. It's built in Rust using tower-lsp for LSP functionality and reqwest for HTTP communication.

## Pre-Commit Checks

Before making any git commit, always run:

1. `cargo fmt` - Format all code according to Rust style guidelines
2. `cargo check` - Verify that the code compiles successfully

These checks ensure code quality and prevent committing broken code.

## Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Run clippy with CI settings (treat warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings
```

If you're about to `git commit`, always `cargo fmt` and `cargo clippy` and `cargo test` first.

### Running the LSP Server
```bash
# Set required environment variables
export JENKINS_URL="https://jenkins.example.com"
export JENKINS_USER_ID="your-username"
export JENKINS_API_TOKEN="your-api-token"

# Optional: Enable debug logging
export RUST_LOG=jenkinsfile_ls=debug,tower_lsp=info

# Run the server
cargo run
```

### Using Nix
```bash
# Enter development shell
nix develop

# Build Nix package
nix build

# Binary will be in ./result/bin/jenkinsfile-ls
```

## Architecture

The codebase follows a modular architecture with clear separation of concerns:

### Core Flow
1. **main.rs** - Entry point that:
   - Initializes tracing/logging (outputs to stderr for LSP compatibility)
   - Loads configuration from environment variables or ~/.config/jenkinsfile-ls/config.toml
   - Creates JenkinsClient with reqwest HTTP client
   - Sets up tower-lsp Server on stdio and starts event loop

2. **server.rs** - LSP protocol handler (Backend struct):
   - Implements tower_lsp::LanguageServer trait
   - Maintains document_map (DashMap<Url, String>) cache of open files
   - On didOpen/didSave: calls validate_document() which sends content to JenkinsClient
   - Converts ValidationResult into LSP diagnostics and publishes to editor

3. **jenkins.rs** - Jenkins API client (JenkinsClient struct):
   - get_crumb(): Fetches CSRF token from /crumbIssuer/api/json
   - validate_jenkinsfile(): POSTs multipart form to /pipeline-model-converter/validate
   - validate(): Convenience method combining crumb + validation
   - Handles authentication via basic auth (username + API token)
   - Gracefully handles missing crumb endpoint for Jenkins without CSRF protection

4. **diagnostics.rs** - Response parser:
   - parse_jenkins_response(): Uses regex to extract line/column/message from Jenkins error format
   - Converts "WorkflowScript: N: message @ line L, column C." to LSP Diagnostic
   - Returns empty Vec for successful validation

5. **config.rs** - Configuration management:
   - Priority order: env vars > specified config file > ~/.config/jenkinsfile-ls/config.toml
   - Supports multiple env var names for compatibility (JENKINS_URL/JENKINS_HOST, etc.)
   - Validates URL format and required fields

6. **types.rs** - Shared types:
   - Crumb: CSRF token from Jenkins
   - ValidationResult: Success or Error(String)
   - LspError: Error types with thiserror for better error messages
   - Result<T>: Type alias for std::result::Result<T, LspError>

### Key Design Patterns
- **Async throughout**: Uses tokio for async runtime, all Jenkins API calls are async
- **Error handling**: Custom LspError enum with specific variants for Auth, JenkinsApi, Config, etc.
- **Document caching**: DashMap provides concurrent HashMap for storing open document content
- **LSP event-driven**: Responds to didOpen/didChange/didSave/didClose events from editor
- **Validation strategy**: Only validates on open and save (not on every keystroke)

### Testing Strategy
- Unit tests in each module using #[cfg(test)]
- Tests focus on: config validation, diagnostic parsing, client creation
- No integration tests yet (would require mock Jenkins server)

## Common Development Tasks

### Adding New Diagnostic Patterns
If Jenkins returns errors in a different format, update the regex in diagnostics.rs:21 and add corresponding tests.

### Supporting Additional Jenkins Endpoints
Add new methods to JenkinsClient in jenkins.rs. Follow the pattern of get_crumb() for error handling (Auth, JenkinsApi, Network errors).

### Adding LSP Capabilities
Implement additional tower_lsp::LanguageServer trait methods in server.rs. Update ServerCapabilities in initialize() method.

### Modifying Configuration Options
1. Add field to Config struct in config.rs
2. Update from_env() to read environment variable
3. Update validation in validate() method
4. Document in README.md

## CI/CD

The project uses GitHub Actions (.github/workflows/ci.yml) with two jobs:
- **rust**: Runs clippy, build, tests, and fmt check using Nix devshell
- **nix-build**: Builds the Nix package

All checks must pass before merging.
