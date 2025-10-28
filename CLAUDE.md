# Claude Code Instructions for jenkinsfile-ls

## Pre-Commit Checks

Before making any git commit, always run:

1. `cargo fmt` - Format all code according to Rust style guidelines
2. `cargo check` - Verify that the code compiles successfully

These checks ensure code quality and prevent committing broken code.

## Workflow

```bash
# After making changes
cargo fmt
cargo check

# Only commit if cargo check passes
git add <files>
git commit -m "..."
```
