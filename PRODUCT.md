# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Static hand-built HTML + CSS, zero framework, zero build step, zero JS frameworks; single-page marketing site. (Inference: user said "max best site", terse; taste-skill maps the developer-tool audience to native CSS; the sibling jev-seo site used the same pattern and was accepted. Confirmed by no objection.)

## Users

Developers and AI coding agents who ask LLMs for open-source libraries and get hallucinated package names or six-year-old abandoned repos. Primary scene: a terminal or an agent context where discovery must be grounded, fast, and typed.

## Product Purpose

jev-scout is a zero-hallucination open-source repo and crate discovery tool: it queries real registries (GitHub REST API, crates.io), scores candidates with TypeSafe AI Jev System One in a single speculative fan-out call, and returns verified, installable repositories with exact stars, licenses, and clone commands. Dual surface: terminal CLI + stdio MCP server for autonomous agents.

## Positioning

The mechanism a neighboring product cannot copy: decoupling discovery from decision. Grounding first (only real live metadata), then one fan-out Jev call judging fit (Score 1-4), maintenance freshness (Noul probability), and best match (Choice). No paid search proxies, no browser automation, no local databases, no text generation. Zero hallucinated packages by construction.

## Capabilities

- Query: `jev-scout "fast sqlite tui in rust"`
- `--ecosystem all|github|crates`, `-e`
- `--limit`, `-n` (max results)
- `--json`, `-j` (machine-readable output)
- `--no-filter` (show weak matches; default filters fit < 2.5 or confidence < 0.5)
- `--mcp` (stdio JSON-RPC 2.0 MCP server, `scout_repos` tool with structuredContent)
- Honest metrics: ⭐ stars (GitHub), ⬇ downloads (crates.io), pushed_at, language, topics
- Recency decay ×0.9 for candidates stale 180+ days
- Ranking: score × confidence, deterministic

## Evidence

- MIT license, Rust 2021, deps: ureq, lexopt, serde only
- Single static binary < 5MB; cold latency ~2.0-2.3s (GitHub ~0.85s + Jev ~1.0s serial floor); warm MCP repeat ~0.3s (TTL 60s in-memory caches)
- 2/2 tests passing, clippy clean; version 0.1.1
- Badge color language: #0d9488 teal
- Core eval set winners: sabiql (sqlite tui), aginxbrowser (headless no chromium), Expenso (expense tracker), jsonschema-family (json schema rust)
- Example verified install: `cargo install jev-scout`
- Author: Akash Priyadarshi (Patna, Bihar, India)