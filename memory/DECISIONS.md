# Architectural Decisions Log - jev-scout

## ADR-001: Language Stack Selection
- **Status**: Accepted
- **Context**: Choosing between Rust, Go, and TypeScript for a zero-dependency, sub-second CLI and MCP server.
- **Decision**: Evaluated using TypeSafe Jev System One (`jev-1.13.0`). Jev picked `Rust` with $P=0.61$ over Go ($0.36$) and TypeScript ($0.03$).
- **Consequences**: Fast compilation, single self-contained binary (<3MB), zero runtime dependencies, instant cold starts.

## ADR-002: HTTP Client Selection
- **Status**: Accepted
- **Context**: Choosing between `ureq` (blocking) and `reqwest` (async/blocking).
- **Decision**: Evaluated using TypeSafe Jev. Jev picked `ureq` with $P=1.00$ (Confidence: $1.00$) over `reqwest`.
- **Consequences**: Zero Tokio async runtime bloat, minimal binary size, simple predictable blocking calls.

## ADR-003: CLI Parser Selection
- **Status**: Accepted
- **Context**: Choosing between `lexopt` (minimal zero-dep) and `clap` (derive macro).
- **Decision**: Evaluated using TypeSafe Jev. Jev picked `lexopt` with $P=0.98$ (Confidence: $0.96$) over `clap`.
- **Consequences**: Fastest compile times (<1s), zero macro expansion overhead, tiny binary footprint.

## ADR-004: Candidate Ranking Metric
- **Status**: Accepted
- **Context**: Choosing between raw score vs confidence-weighted ranking.
- **Decision**: Evaluated using TypeSafe Jev. Jev picked `confidence_weighted` ($P=1.00$, Confidence: $1.00$) over `score_only`.
- **Consequences**: Candidate rank is computed as $\text{Score} \times \text{Confidence}$, penalizing uncertain predictions and preventing false-positive recommendations.
