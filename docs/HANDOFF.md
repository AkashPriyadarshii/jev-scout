# Maintainer Handoff Runbook - jev-scout

## Overview
This runbook explains how to build, test, package, and release `jev-scout`.

## Prerequisites
- Rust 1.80+ (`cargo`, `rustc`)
- GitHub CLI (`gh`) for authenticated API requests
- TypeSafe AI API Key (`TYPESAFE_API_KEY`)

## Local Development & Testing

```bash
# Clone the repository
cd C:\Users\saves\Desktop\jev-scout

# Run offline unit tests (does not require API keys or network)
cargo test

# Run a live test search
export TYPESAFE_API_KEY="your_key"
cargo run -- "fast sqlite viewer in rust"

# Run MCP server smoke test
cargo run -- --mcp
```

## Release Checklist
1. Bump version in `Cargo.toml` and update `CHANGELOG.md`.
2. Run `cargo fmt --check` and `cargo clippy -- -D warnings`.
3. Verify test coverage: `cargo test`.
4. Publish crate to crates.io: `cargo publish`.
5. Create GitHub release: `gh release create v0.1.0 --title "v0.1.0" --notes "..."`.
