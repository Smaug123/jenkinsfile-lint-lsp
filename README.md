# jenkinsfile-ls

A Language Server Protocol (LSP) implementation for validating Jenkinsfiles using a remote Jenkins instance.

## Features

- **Real-time validation**: Validates Jenkinsfiles by sending them to your Jenkins instance
- **LSP compliance**: Works with any LSP-compatible editor (Neovim, VS Code, etc.)
- **Flexible configuration**: Environment variables or config file support
- **Secure**: Supports API tokens and self-signed certificates
- **Fast**: Async validation with minimal editor blocking

## Requirements

- A Jenkins instance with the [Pipeline Model Definition Plugin](https://plugins.jenkins.io/pipeline-model-definition/)
- Jenkins user credentials (username + API token)
- Rust 1.70+ (for building from source)

## Installation

### From Source

```bash
git clone https://github.com/yourusername/jenkinsfile-ls.git
cd jenkinsfile-ls
cargo install --path .
```

### Using Nix

```bash
nix build
# Binary will be in ./result/bin/jenkinsfile-ls
```

## Configuration

### Environment Variables (Recommended)

```bash
export JENKINS_URL="https://jenkins.example.com"
export JENKINS_USER_ID="your-username"
export JENKINS_API_TOKEN="your-api-token"

# Optional: Skip TLS verification for self-signed certificates
export JENKINS_INSECURE="1"
```

**Alternative variable names** (for compatibility):
- `JENKINS_URL` or `JENKINS_HOST`
- `JENKINS_USER_ID` or `JENKINS_USERNAME`
- `JENKINS_API_TOKEN`, `JENKINS_TOKEN`, or `JENKINS_PASSWORD`

### Config File

Create `~/.config/jenkinsfile-ls/config.toml`:

```toml
jenkins_url = "https://jenkins.example.com"
username = "your-username"
api_token = "your-api-token"
insecure = false  # Set to true for self-signed certificates
```

### Getting a Jenkins API Token

1. Log in to Jenkins
2. Click your username (top right)
3. Click "Configure"
4. Under "API Token", click "Add new Token"
5. Give it a name and click "Generate"
6. Copy the token (you won't see it again!)

## Editor Setup

### Neovim

Add to your Neovim config:

```lua
-- Using vim.lsp.start() (Neovim 0.8+)
vim.api.nvim_create_autocmd("FileType", {
  pattern = "jenkinsfile",
  callback = function()
    vim.lsp.start({
      name = "jenkinsfile-ls",
      cmd = { "jenkinsfile-ls" },
      root_dir = vim.fs.dirname(vim.fs.find({ "Jenkinsfile" }, { upward = true })[1]),
    })
  end,
})

-- Or using lspconfig
require("lspconfig.configs").jenkinsfile_ls = {
  default_config = {
    cmd = { "jenkinsfile-ls" },
    filetypes = { "jenkinsfile" },
    root_dir = require("lspconfig.util").find_git_ancestor,
    single_file_support = true,
  },
}

require("lspconfig").jenkinsfile_ls.setup({})
```

**Set filetype for Jenkinsfiles:**

```lua
vim.filetype.add({
  filename = {
    ["Jenkinsfile"] = "jenkinsfile",
  },
  pattern = {
    [".*%.jenkinsfile"] = "jenkinsfile",
  },
})
```

### VS Code

Install the LSP extension and add to `settings.json`:

```json
{
  "jenkinsfile-ls.enable": true,
  "jenkinsfile-ls.serverPath": "/path/to/jenkinsfile-ls",
  "jenkinsfile-ls.trace.server": "verbose"
}
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "jenkinsfile"
scope = "source.jenkinsfile"
file-types = ["Jenkinsfile", "jenkinsfile"]
language-servers = ["jenkinsfile-ls"]

[language-server.jenkinsfile-ls]
command = "jenkinsfile-ls"
```

## Usage

The LSP server runs automatically when you open a Jenkinsfile in a configured editor.

### Validation Triggers

- **On open**: Validates immediately when you open a file
- **On save**: Re-validates when you save changes

### Logging

Set the `RUST_LOG` environment variable for debugging:

```bash
RUST_LOG=debug jenkinsfile-ls
```

Or from your editor, set the environment before starting:

```bash
export RUST_LOG=jenkinsfile_ls=debug,tower_lsp=info
nvim Jenkinsfile
```

## How It Works

1. Editor opens a Jenkinsfile and starts the LSP server
2. Server loads configuration from environment or config file
3. On save, server sends the file content to Jenkins' validation endpoint:
   - `GET /crumbIssuer/api/json` - Get CSRF token
   - `POST /pipeline-model-converter/validate` - Validate Jenkinsfile
4. Server parses Jenkins' response for errors
5. Diagnostics are displayed in the editor

## Example Error Output

Jenkins returns errors like:

```
WorkflowScript: 46: unexpected token: } @ line 46, column 1.
```

The LSP server converts this to editor diagnostics showing:
- Line 46, column 1
- Error message: "unexpected token: }"
- Severity: ERROR

## Troubleshooting

### "Failed to load configuration"

Make sure you've set the required environment variables or created a config file.

### "Authentication failed"

- Verify your username and API token are correct
- Try generating a new API token in Jenkins
- Check that your Jenkins user has permission to access the validation endpoint

### "Validation endpoint not found"

Your Jenkins instance needs the [Pipeline Model Definition Plugin](https://plugins.jenkins.io/pipeline-model-definition/). Install it from Jenkins Plugin Manager.

### SSL Certificate Errors

If using self-signed certificates, set `JENKINS_INSECURE=1` or `insecure = true` in config.

### No diagnostics appearing

Check the LSP server logs:
- In Neovim: `:LspLog`
- Set `RUST_LOG=debug` for detailed logging to stderr

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Formatting and Linting

```bash
cargo fmt
cargo clippy
```

### Using Nix Devshell

```bash
nix develop
# or with direnv
direnv allow
```

## Architecture

- **main.rs**: Entry point, configuration loading, server startup
- **server.rs**: LSP protocol implementation (tower-lsp)
- **jenkins.rs**: Jenkins API client (crumb fetching, validation)
- **diagnostics.rs**: Parse Jenkins errors into LSP diagnostics
- **config.rs**: Configuration management
- **types.rs**: Shared data structures and error types

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.

## Acknowledgments

Inspired by the [jenkinsfile-linter.lua](./jenkinsfile-linter.lua) Neovim plugin.
