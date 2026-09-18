# Product Requirements Document (PRD) - jev-scout

## 1. Problem Statement
Developers and autonomous AI coding agents face a chronic problem: when searching for libraries and tools, general-purpose LLMs hallucinate non-existent packages or suggest abandoned repositories. Meanwhile, native registry search engines (GitHub, crates.io) only support exact or keyword-based queries, failing to understand architectural and conceptual descriptions (such as "fast headless browser without chromium").

## 2. Target Users
1. **Developers**: Running terminal queries for libraries and CLI tools in their day-to-day work.
2. **AI Coding Agents (Claude Code, Antigravity, OpenClaw)**: Autonomous agents that need to locate unbloated, verified packages and repositories without leaving their execution loop.

## 3. Goals & Success Criteria
- **Zero Hallucinations**: Every suggested library or repository must be grounded in an actual GitHub repo or crates.io crate.
- **Sub-Second Latency**: End-to-end execution must complete under 1,500ms on broadband connections.
- **₹0 Budget**: No paid search engine proxies (Search1API, Serper). Uses official GitHub REST and crates.io APIs.
- **Single Static Binary**: Compiles to a self-contained executable under 5MB with zero external runtime dependencies.
- **Dual Interface**: Serves both interactive human terminal use and machine-readable Stdio MCP tool calls.

## 4. Key Functional Requirements
- **FR-1 (Candidate Retrieval)**: Fetch up to 10 candidates from GitHub REST API and crates.io API matching the query terms.
- **FR-2 (Jev System One Evaluation)**: Submit candidate summaries to TypeSafe AI's `POST /v1/systemone` in a single fan-out call evaluating:
  - `Score`: Architectural fit on a 1 to 4 scale.
  - `Noul`: Calibrated probability that the repository is actively maintained.
  - `Choice`: Top overall recommendation among candidates.
- **FR-3 (Confidence-Weighted Ranking)**: Sort candidates by `score * confidence` to penalize uncertain predictions.
- **FR-4 (Terminal Output)**: Print ANSI-highlighted cards displaying score, confidence, stars, license, and clone/install commands.
- **FR-5 (Machine Output)**: Support `--json` flag for scripting and pipelines.
- **FR-6 (MCP Server)**: Support `--mcp` flag to expose a JSON-RPC 2.0 `scout_repos` tool over standard input/output.
