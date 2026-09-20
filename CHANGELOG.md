# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-09-20

### Fixed
- `--version` and `--help` printed a hardcoded "0.1.0". Both now read `CARGO_PKG_VERSION`.

## [0.2.0] - 2026-09-20

### Added
- Parallel GitHub + crates.io candidate fetch (`std::thread::spawn`) — cold latency 8.9s → ~2s.
- In-memory TTL 60s caches for search candidates AND Jev evaluation — MCP warm repeats ~0.3s.
- `--no-filter` flag and `strict` MCP arg to disable weak-match filtering.
- Rich candidate metadata: `pushed_at`, `language`, GitHub `topics`, crates.io `downloads`.
- Honest metrics display: ⭐ stars (GitHub), ⬇ downloads (crates.io).
- Recency decay (×0.9) for candidates stale 180+ days (host-side Julian-day math, zero deps).
- MCP protocol compliance: version negotiation (2024-11-05/2025-03-26/2025-06-18), `ping`, silent notification handling, `structuredContent`+`isError`, proper JSON-RPC error codes (-32601/-32602/-32000), limit clamping.
- Transparent CLI timing: `search Xms + eval Yms`.

### Changed
- Jev state payload trimmed (derivable fields dropped) — fewer tokens, less context rot.
- `modern` Noul receives pushed_at/downloads/language/topics as input.
- Weak matches (fit < 2.5 or confidence < 0.5) filtered by default; noisy rows removed from every output.
- `serverInfo.version` sourced from Cargo.toml via `env!()`.

## [0.1.0] - 2026-09-18

### Added
- Initial project architecture and documentation skeleton.
- Candidate retrieval engine for GitHub REST API and crates.io API.
- Single fan-out evaluation pipeline using TypeSafe AI Jev System One model (`jev-latest`).
- Lexopt-based CLI interface with ANSI color card rendering and `--json` flag.
- Stdio JSON-RPC 2.0 Model Context Protocol (MCP) server mode (`--mcp`).
- Offline unit tests with mock JSON fixtures for reproducible CI testing.
