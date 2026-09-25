<!--
Title: jev-scout - Zero-Hallucination Open Source Repo and Library Scout
Description: Fast CLI and MCP server in Rust that discovers real, actively maintained open-source repos and crates matching natural-language prompts using TypeSafe AI Jev System One scoring.
Keywords: typesafe-ai, jev, github-search, crates-io, rust, cli, mcp-server, coding-agents, repo-finder, llm-guardrails
-->

<div align="center">

# jev-scout

**Support:** fuel the next build — [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/AkashPriyadarshi)

**Zero-hallucination open-source repo and crate discovery powered by TypeSafe AI Jev.**

[![License: MIT](https://img.shields.io/badge/license-MIT-0d9488.svg?style=flat-square)](LICENSE)
[![TypeSafe Jev](https://img.shields.io/badge/TypeSafe-Jev-0d9488.svg?style=flat-square)](https://typesafe.ai)
[![Rust](https://img.shields.io/badge/rust-2021-0d9488.svg?style=flat-square)](Cargo.toml)
[![MCP](https://img.shields.io/badge/MCP-JSON--RPC_2.0-0d9488.svg?style=flat-square)](https://modelcontextprotocol.io)

**Author:** [Akash Priyadarshi](https://github.com/AkashPriyadarshii)

[Why](#why) • [Quickstart](#quickstart) • [How it works](#how-it-works) • [CLI Flags](#cli-flags) • [Architecture](#architecture) • [Non-Goals](#non-goals) • [Ecosystem](#ecosystem)

</div>

[![stars](https://img.shields.io/github/stars/AkashPriyadarshii/jev-scout?style=flat-square&label=stars)](https://github.com/AkashPriyadarshii/jev-scout/stargazers) [![crates.io](https://img.shields.io/crates/v/jev-scout?style=flat-square)](https://crates.io/crates/jev-scout) [![downloads](https://img.shields.io/crates/d/jev-scout?style=flat-square)](https://crates.io/crates/jev-scout) [![release](https://img.shields.io/github/v/release/AkashPriyadarshii/jev-scout?style=flat-square&label=release)](https://github.com/AkashPriyadarshii/jev-scout/releases)

---

## Why

When developers and AI coding agents ask LLMs for open-source libraries, general-purpose models routinely hallucinate non-existent package names or recommend abandoned six-year-old repositories. GitHub's native search relies on rigid keyword matching that fails on conceptual queries.

`jev-scout` solves this by decoupling discovery from decision:
- **Grounding first:** Queries real package registries (GitHub REST API, crates.io) to fetch actual live metadata.
- **System One scoring:** Uses TypeSafe AI's `jev-latest` in a single speculative fan-out call (~1s API floor) to score architectural fit, license suitability, and maintenance freshness.
- **Zero hallucinated packages:** You only get verified, installable repositories with exact stars, licenses, and clone commands.
- **Dual surface:** Works as an interactive terminal CLI for developers and as a stdio MCP server for autonomous coding agents (Claude Code, Antigravity).

---

## Quickstart

### Installation

```bash
cargo install jev-scout
```

Or build from source:

```bash
git clone https://github.com/AkashPriyadarshii/jev-scout.git
cd jev-scout
cargo build --release
```

### Setup API Key

`jev-scout` requires a TypeSafe AI API key:

```bash
export TYPESAFE_API_KEY="your_typesafe_key"
```

### Usage

Search for repositories matching natural language concepts:

```bash
# Search crates and repositories
jev-scout "fast sqlite tui in rust"

# Filter by ecosystem
jev-scout "headless browser without chromium" --ecosystem rust

# Output raw JSON for scripts and agents
jev-scout "token efficient grep for coding agents" --json

# Show all candidates, skip weak-match filtering
jev-scout "sqlite tui" --no-filter
```

### MCP Server (for AI coding agents)

Start jev-scout as a stdio MCP server exposing the `scout_repos` tool, then point any MCP client at it:

```bash
# Claude Code
claude mcp add jev-scout -- bash -c "export TYPESAFE_API_KEY=$TYPESAFE_API_KEY && jev-scout --mcp"

# pi
# add an mcpServers entry: {"command": "jev-scout", "args": ["--mcp"], "env": {"TYPESAFE_API_KEY": "..."}}
```

The `scout_repos` tool takes `query` (required), plus optional `ecosystem` (`all`/`github`/`crates`), `limit` (1-10), and `strict` (filter weak matches, default true). Results include `structuredContent` for typed consumption.

---

## How it works

```text
User Query: "fast sqlite tui in rust"
   │
   ├─► 1. Grounded Search (GitHub REST API + crates.io)
   │      Pulls top candidate repos with stars, licenses, and commit dates.
   │
   ├─► 2. Speculative Fan-out Call (POST https://api.typesafe.ai/v1/systemone)
   │      Evaluates all candidates in a single ~1s call with typed primitives:
   │      - Score(fit): 1 (unrelated) to 4 (exact architectural fit)
   │      - Noul(modern): Calibrated probability of active maintenance
   │      - Choice(best_match): Single top candidate
   │
   └─► 3. Deterministic Ranking & Output
          Sorts by confidence-weighted score (score * confidence).
          Renders terminal cards or JSON.
```

---

## CLI Flags

| Flag | Short | Default | Description |
|---|---|---|---|
| `--ecosystem` | `-e` | `all` | Target ecosystem (`all`, `github`, `crates`) |
| `--limit` | `-n` | `5` | Maximum number of ranked results to return |
| `--json` | `-j` | `false` | Output machine-readable JSON to stdout |
| `--no-filter` | | `false` | Show all candidates, skip weak-match filtering |
| `--mcp` | | `false` | Start as a stdio Model Context Protocol (MCP) server |
| `--help` | `-h` | | Print help information |
| `--version` | `-v` | | Print version |

---

## Architecture

```text
jev-scout/
├── Cargo.toml          # Rust dependencies: ureq, lexopt, serde
├── README.md           # Documentation
├── CLAUDE.md           # Development rules
├── AGENTS.md           # Agent directives
├── STATE.md            # Active project state
├── CHANGELOG.md        # Version history
├── src/
│   ├── main.rs         # Lexopt argument parsing and terminal display
│   ├── jev.rs          # TypeSafe API client (Choice, Score, Noul)
│   ├── search.rs       # Candidate retriever (GitHub REST + crates.io)
│   └── mcp.rs          # Stdio JSON-RPC 2.0 MCP server handler
└── tests/
    └── mock_test.rs    # Offline unit tests using recorded fixtures
```

---

## Latency (honest numbers)

Measured live on an Intel Core i3 / Windows 11 / broadband, against the real APIs:

- Cold (new process): ~2.0-2.3s. The floor is two serial external legs: GitHub search API ~0.9s + Jev API ~1.0s (raw curl floor of 1.0s even for a single question). Search runs in parallel threads; the Jev call is a single fan-out request.
- Warm (MCP session repeat, TTL 60s caches): ~0.3s. Both search results and Jev evaluations are cached in-process.
- Filters and ranking run instantly once candidates are fetched.

## Non-Goals

- **No text generation:** `jev-scout` does not write code summaries or essays. It returns verified repo metadata and typed scores.
- **No heavy local databases:** Zero SQLite, Postgres, or caching daemons required.
- **No browser automation:** No Chromium, Playwright, or web scraping dependencies.
- **No paid search engines:** Relies on official GitHub and crates.io APIs rather than paid search proxies.

---

## Ecosystem

- [design-genius](https://github.com/AkashPriyadarshii/design-genius) - Design systems and UI craft documentation.
- [akash-design-engineering](https://github.com/AkashPriyadarshii/akash-design-engineering) - Advanced front-end architecture and animations.
- [tdlib-android](https://github.com/AkashPriyadarshii/tdlib-android) - Precompiled TDLib for Android with zero-dependency builds.
- [kharcha](https://github.com/AkashPriyadarshii/kharcha) - India-first offline UPI expense tracker.

---

## Author

**Akash Priyadarshi**  
Patna, Bihar, India  
- GitHub: [AkashPriyadarshii](https://github.com/AkashPriyadarshii)  
- Portfolio: [akashpriyadarshi.vercel.app](https://akashpriyadarshi.vercel.app)  
- LinkedIn: [akashpriyadarshii](https://linkedin.com/in/akashpriyadarshii)  
- Resume: [akashpriyadarshii.github.io/Resume](https://akashpriyadarshii.github.io/Resume/)

<p align="center">
  <img src="https://api.star-history.com/svg?repos=AkashPriyadarshii/jev-scout&type=Date" width="600" alt="star history" />
</p>

## Social

- X / Twitter: [@Akash__ydv001](https://x.com/Akash__ydv001)  
- Threads: [@akash.priyadarshii](https://www.threads.net/@akash.priyadarshii)  
- Instagram: [@akash.priyadarshii](https://www.instagram.com/akash.priyadarshii/)  
- Reddit: [u/akashpriyadarshi](https://reddit.com/user/akashpriyadarshi)

---

*Zero hallucinations, verified repositories, sub-second decisions.*