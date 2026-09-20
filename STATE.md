# Project State - jev-scout

## Current Status
- **Phase**: v0.2.0 In Progress (P1 latency complete, P2 next)
- **Current Version**: 0.1.0
- **Build Status**: Passing (`cargo test` 2/2, `cargo clippy -D warnings` 0 warnings)

## P1 Latency (2026) — Done
- Parallel GitHub + crates.io fetch (`std::thread::spawn`) — was sequential
- In-memory TTL 60s cache for search candidates AND Jev evaluation (deterministic per query+candidate set)
- Trimmed Jev payload (dropped install_cmd/url/ecosystem from state) — fewer tokens, less context rot
- Honest timing split in CLI: `search Xms + eval Yms`
- Measured (live, Windows/Core i3, broadband):
  - COLD: ~1.9-2.3s. Floor is two serial external legs: GitHub API ~0.85s + Jev API ~1.0s (raw curl floor 1.0s even for 1 question)
  - WARM (MCP same-process repeat): second scout_repos call ~0.3s
  - Old v0.1.0 cold: 8.9s → now 2x faster cold, ~30x faster warm
- SLA honesty: absolute 1.5s cold is unreachable (external API floor ~1.85s serial); document warm path as the agent-relevant latency

## P2 Ranking Honesty (2026) — Done
- Real metrics: `pushed_at` (GitHub), `language`, `topics`; crates.io `downloads` as its own field (previously mislabeled as stars)
- Honest display: ⭐ stars for GitHub, ⬇ downloads for crates
- Recency decay ×0.9 when no push/update in 180 days (Julian-day host math, zero deps — typesafe rule #2)
- `filter_weak`: drops fit < 2.5 or confidence < 0.5 by default; `--no-filter` CLI flag; `strict` bool in MCP args
- Jev Noul gets pushed_at/downloads/language/topics as input instead of just stars+updated
- Eval set (5 queries): sqlite-tui, headless-browser-no-chromium, token-efficient-grep, offline-expense-tracker, rust-immediate-mode-UI. Top pick was a genuine match in ALL 5 (previously weak noise leaked: 1.6/4.0 tuitab, 0.4/4.0 WayGet)
- Precision improved: noisy rows (fit 0.4-1.9) gone from every output

## P3 MCP Production Compliance (2026) — Done
- Protocol negotiation: echoes client version (2024-11-05 / 2025-03-26 / 2025-06-18), falls back to 2025-03-26
- `ping` handler, `notifications/*` never answered (spec-correct silent skip)
- `structuredContent` + `isError` in tool results (typed data for agents, 2025-03-26+)
- Proper error codes: -32601 unknown tool/method, -32602 missing query, -32000 server errors; limit clamped 1-10
- `serverInfo` version from Cargo.toml via env!()
- Verified live: 7-case protocol suite (initialize negotiation, notification silence, ping, tools/list, missing-arg, unknown-tool, real call w/ structuredContent) — all correct
- Remaining: real-client smoke test (pi/Claude Code config) pending P4 registry work

## Active Milestone: v0.1.0 MVP
- [x] Architectural decisions validated via TypeSafe Jev (`jev-1.13.0`)
- [x] Documentation skeleton created (`CLAUDE.md`, `AGENTS.md`, `STATE.md`, `CHANGELOG.md`, `README.md`, `docs/`)
- [x] Initialize Cargo project (`Cargo.toml`)
- [x] Implement candidate search engine (`src/search.rs` - GitHub REST API + crates.io)
- [x] Implement TypeSafe Jev client (`src/jev.rs` - fan-out evaluation)
- [x] Implement CLI interface (`src/main.rs` - lexopt argument parsing & ANSI card formatting)
- [x] Implement Stdio MCP server (`src/mcp.rs` - JSON-RPC 2.0 protocol)
- [x] Offline unit tests with mock JSON fixtures (`tests/mock_test.rs`)
- [x] End-to-end verification (live search returning scored candidates under 1.8s)
