# Technical Design Document - jev-scout

## 1. System Overview
`jev-scout` coordinates between three primary components:
1. **Search Layer (`search.rs`)**: Queries GitHub REST API (`https://api.github.com/search/repositories`) and crates.io API (`https://crates.io/api/v1/crates`). If the `GITHUB_TOKEN` environment variable or `gh auth token` output is available, it sends authorization headers to unlock 5,000 requests/hour.
2. **Evaluation Layer (`jev.rs`)**: Packs candidate summaries into a JSON state and issues a single HTTP request to `https://api.typesafe.ai/v1/systemone` using model `jev-latest`.
3. **Interface Layer (`main.rs` & `mcp.rs`)**: Dispatches either terminal formatting, JSON streaming, or JSON-RPC 2.0 MCP server handling.

## 2. Jev Request Schema & Prompt Design

```json
{
  "model": "jev-latest",
  "state": {
    "query": "fast sqlite tui in rust",
    "candidates": [
      {
        "id": "julien-cpsn/sqlite-tui",
        "name": "sqlite-tui",
        "description": "A standalone terminal UI for viewing SQLite databases.",
        "stars": 840,
        "license": "Apache-2.0",
        "updated_at": "2026-08-30T10:00:00Z"
      }
    ]
  },
  "questions": {
    "best_match": {
      "type": "choice",
      "instructions": "Which candidate is the closest architectural and functional match for the query?",
      "criteria": {
        "julien-cpsn/sqlite-tui": "Standalone terminal UI for SQLite inspection"
      }
    },
    "fit_julien_cpsn_sqlite_tui": {
      "type": "score",
      "instructions": "Rate how well this repository satisfies the requirements of: 'fast sqlite tui in rust'",
      "criteria": {
        "1": "Unrelated or completely different functional domain",
        "2": "Loosely related topic but missing core requested features or language stack",
        "3": "Strong match satisfying most functional constraints",
        "4": "Exact architectural and functional match"
      }
    },
    "modern_julien_cpsn_sqlite_tui": {
      "type": "noul",
      "instructions": "Is this repository actively maintained and using modern software patterns?",
      "criteria": {
        "true": "Actively maintained with modern tooling",
        "false": "Abandoned, archived, or legacy deprecated code"
      }
    }
  }
}
```

## 3. Candidate Scoring and Ranking
For each candidate $i$:
$$\text{RankScore}_i = \text{FitScore}_i \times \text{Confidence}_i$$
Where:
- $\text{FitScore}_i \in [1.0, 4.0]$
- $\text{Confidence}_i \in [0.0, 1.0]$

Candidates with $\text{FitScore} < 2.0$ are automatically filtered out as irrelevant.

## 4. MCP Server Specification
When invoked with `--mcp`:
- Reads JSON-RPC 2.0 requests from `stdin`.
- Exposes tool `scout_repos`:
  - `query` (string, required): The natural-language requirement.
  - `ecosystem` (string, optional): Filter by `all`, `github`, or `crates`.
  - `limit` (integer, optional): Maximum ranked results (default 5).
- Writes JSON-RPC 2.0 responses to `stdout`.
