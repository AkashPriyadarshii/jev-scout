use crate::jev::evaluate_candidates;
use crate::search::search_candidates;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const PROTOCOL_VERSION: &str = "2025-03-26";
const SUPPORTED_PROTOCOLS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

pub fn run_mcp_server(api_key: &str) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Notifications have no id and must never be answered.
        if request.get("id").is_none() {
            continue;
        }

        let id = request["id"].clone();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => {
                // Negotiate: echo the client protocol version when supported,
                // otherwise fall back to our newest supported one.
                let requested = request
                    .pointer("/params/protocolVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let version = if SUPPORTED_PROTOCOLS.contains(&requested) {
                    requested
                } else {
                    PROTOCOL_VERSION
                };

                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": version,
                        "capabilities": {
                            "tools": { "listChanged": false }
                        },
                        "serverInfo": {
                            "name": "jev-scout",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                });
                respond(&mut stdout, &response)?;
            }
            "ping" => {
                respond(
                    &mut stdout,
                    &json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
                )?;
            }
            "tools/list" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "scout_repos",
                                "title": "Scout open-source repos and crates",
                                "description": "Search and score open-source repositories and crates matching natural-language prompts using TypeSafe Jev System One model. Zero hallucinations, grounded in real GitHub and crates.io metadata. Returns ranked candidates with fit score, confidence, maintenance probability, stars/downloads, and install commands.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "query": {
                                            "type": "string",
                                            "description": "The natural language requirement or description of the tool/library (e.g. 'fast sqlite tui in rust')."
                                        },
                                        "ecosystem": {
                                            "type": "string",
                                            "enum": ["all", "github", "crates"],
                                            "description": "Filter by target ecosystem. Defaults to 'all'."
                                        },
                                        "limit": {
                                            "type": "integer",
                                            "minimum": 1,
                                            "maximum": 10,
                                            "description": "Maximum number of ranked results to return (default 5)."
                                        },
                                        "strict": {
                                            "type": "boolean",
                                            "description": "Filter out weak matches (fit < 2.5 or confidence < 0.5). Default true."
                                        }
                                    },
                                    "required": ["query"]
                                }
                            }
                        ]
                    }
                });
                respond(&mut stdout, &response)?;
            }
            "tools/call" => {
                let tool_name = request
                    .pointer("/params/name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let args = request
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or(json!({}));

                if tool_name != "scout_repos" {
                    respond(
                        &mut stdout,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": format!("Method or tool '{}' not found", tool_name) }
                        }),
                    )?;
                    continue;
                }

                let query = args
                    .get("query")
                    .and_then(|q| q.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if query.is_empty() {
                    respond(
                        &mut stdout,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32602, "message": "Missing required argument: query" }
                        }),
                    )?;
                    continue;
                }

                let ecosystem = match args.get("ecosystem").and_then(|e| e.as_str()).unwrap_or("all") {
                    e @ ("all" | "github" | "crates") => e,
                    other => {
                        respond(
                            &mut stdout,
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": { "code": -32602, "message": format!("Invalid ecosystem '{}'. Use all, github, or crates.", other) }
                            }),
                        )?;
                        continue;
                    }
                };
                let limit = args
                    .get("limit")
                    .and_then(|l| l.as_u64())
                    .unwrap_or(5)
                    .clamp(1, 10) as usize;
                let strict = args.get("strict").and_then(|s| s.as_bool()).unwrap_or(true);

                let candidates = search_candidates(&query, ecosystem, 8);
                let evaluated = match evaluate_candidates(&query, candidates, api_key) {
                    Ok(res) => res,
                    Err(err) => {
                        respond(
                            &mut stdout,
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": { "code": -32000, "message": err }
                            }),
                        )?;
                        continue;
                    }
                };

                let evaluated = if strict {
                    crate::jev::filter_weak(evaluated)
                } else {
                    evaluated
                };
                let top_results: Vec<_> = evaluated.into_iter().take(limit).collect();

                // structuredContent (2025-03-26+) lets agents consume typed data
                // without re-parsing the display text.
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "isError": false,
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&top_results)?
                            }
                        ],
                        "structuredContent": top_results
                    }
                });
                respond(&mut stdout, &response)?;
            }
            _ => {
                respond(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("Method '{}' not found", method) }
                    }),
                )?;
            }
        }
    }

    Ok(())
}

fn respond(stdout: &mut io::Stdout, response: &Value) -> io::Result<()> {
    writeln!(stdout, "{}", serde_json::to_string(response)?)?;
    stdout.flush()
}
