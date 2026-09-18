# Architecture Specification - jev-scout

## 1. Module Boundaries

```text
┌─────────────────────────────────────────────────────────────┐
│                         CLI / MCP                           │
│  src/main.rs (lexopt args)  │  src/mcp.rs (JSON-RPC 2.0)    │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Candidate Aggregator                     │
│                       src/search.rs                         │
│   - GitHub REST API (Search Repositories)                   │
│   - crates.io API (Search Crates)                           │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 TypeSafe Jev Decision Layer                 │
│                        src/jev.rs                           │
│   - Single Speculative Fan-out Request                      │
│   - Choice (Best Match) + Score (Fit) + Noul (Modern)       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Ranking & Renderer                      │
│   - Confidence-weighted calculation (score * confidence)    │
│   - ANSI terminal card formatter or JSON output             │
└─────────────────────────────────────────────────────────────┘
```

## 2. Thread & Network Model
- The binary uses synchronous, blocking I/O powered by `ureq`.
- Thread concurrency: Candidate retrieval queries GitHub and crates.io sequentially or in parallel via lightweight OS threads (`std::thread::spawn`), keeping dependencies near zero without pulling in Tokio.
- Timeout policy: Strict 5-second connection and read timeouts on all outgoing HTTP calls.

## 3. Error Recovery & Graceful Degradation
- **Missing Token**: If `GITHUB_TOKEN` is not found, attempt `gh auth token` via child process. If unavailable, proceed with unauthenticated requests (60 req/hr).
- **Rate Limit Exceeded (HTTP 429 / 403)**: Output clear instructions on how to authenticate.
- **TypeSafe Unreachable**: Display a descriptive error message indicating that `TYPESAFE_API_KEY` must be configured.
