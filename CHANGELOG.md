# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-18

### Added
- Initial project architecture and documentation skeleton.
- Candidate retrieval engine for GitHub REST API and crates.io API.
- Single fan-out evaluation pipeline using TypeSafe AI Jev System One model (`jev-latest`).
- Lexopt-based CLI interface with ANSI color card rendering and `--json` flag.
- Stdio JSON-RPC 2.0 Model Context Protocol (MCP) server mode (`--mcp`).
- Offline unit tests with mock JSON fixtures for reproducible CI testing.
