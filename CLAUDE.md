# CLAUDE.md - Developer Guidelines for jev-scout

## Overview
`jev-scout` is a sub-second CLI and Stdio MCP server written in Rust that discovers real, active open-source repositories and crates matching natural-language prompts using TypeSafe AI's Jev System One model (`jev-latest`). It eliminates LLM hallucinations by searching real registries (GitHub REST API, crates.io) first, then scoring candidate fit in a single fan-out request.

## Stack Decisions (Jev-Validated)
- **Language**: Rust (2021 edition)
- **HTTP Client**: `ureq` (blocking, lightweight, zero async-runtime bloat)
- **CLI Parser**: `lexopt` (zero-dependency, minimal binary footprint)
- **JSON**: `serde`, `serde_json`
- **Ranking**: Confidence-weighted score (`score * confidence`)

## Architecture
- **Single Statically-Linked Binary**: Zero runtime external daemon dependencies, zero Docker, zero databases.
- **Candidate Discovery (`src/search.rs`)**: Queries GitHub REST API (leveraging `gh auth token` when available) and crates.io API.
- **Decision Engine (`src/jev.rs`)**: Dispatches a single fan-out request to `POST https://api.typesafe.ai/v1/systemone` evaluating `Score` (fit 1-4), `Noul` (modern maintenance), and `Choice` (best overall candidate).
- **Dual Interface (`src/main.rs`, `src/mcp.rs`)**:
  - Terminal CLI with ANSI color tables, flags (`--ecosystem`, `--limit`, `--json`).
  - Stdio JSON-RPC 2.0 MCP server (`--mcp`) exposing `scout_repos`.

## Build and Test Commands
- **Build**: `cargo build --release`
- **Unit Tests**: `cargo test` (offline mock tests, zero token spend)
- **Clippy**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt --check`

## Design & Ponytail Constraints
- Minimum code that solves the problem.
- Keep network timeouts strict (5 seconds per upstream API call).
- No unrequested dependencies.
- Zero paid APIs required ($0 budget).
