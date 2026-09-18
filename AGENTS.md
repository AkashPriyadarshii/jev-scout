# Project Rules & Agent Instructions - jev-scout

## Context & Persona
- Author: Akash Priyadarshi (Patna, Bihar, India)
- GitHub: AkashPriyadarshii
- Philosophy: Ponytail principles (minimum code, zero bloat, single binary, ₹0 budget)

## Architecture & Code Standards
- **Zero Hallucination Constraint**: Never generate repo names from memory. Always search GitHub or crates.io first, then score real candidates with Jev.
- **Latency First**: The entire CLI command must return in < 1,500ms total. Use a single fan-out request to Jev.
- **Memory & Resource Discipline**: Low memory footprint on Intel Core i3 (8GB RAM). Zero heavy local databases or headless browsers.
- **Error Handling**: Handle network timeouts gracefully. If GitHub rate limit is hit, inform the user with actionable instructions (`gh auth login`). If Jev is unreachable or `TYPESAFE_API_KEY` is missing, fail fast with a clear error message.

## Commit Hygiene
- Imperative conventional commits: `feat:`, `fix:`, `docs:`, `chore:`, `perf:`.
- No em dashes in commit messages or docs.
- Terse and direct subject lines (<72 chars).
