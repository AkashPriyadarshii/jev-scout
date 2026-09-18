use crate::jev::evaluate_candidates;
use crate::search::search_candidates;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        match method {
            "initialize" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "jev-scout",
                            "version": "0.1.0"
                        }
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
            }
            "tools/list" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "scout_repos",
                                "description": "Search and score open-source repositories and crates matching natural-language prompts using TypeSafe Jev System One model. Zero hallucinations, grounded in real GitHub and crates.io metadata.",
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
                                            "type": "number",
                                            "description": "Maximum number of ranked results to return (default 5)."
                                        }
                                    },
                                    "required": ["query"]
                                }
                            }
                        ]
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
            }
            "tools/call" => {
                let tool_name = request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                if tool_name == "scout_repos" {
                    let args = request
                        .get("params")
                        .and_then(|p| p.get("arguments"))
                        .cloned()
                        .unwrap_or(json!({}));

                    let query = args
                        .get("query")
                        .and_then(|q| q.as_str())
                        .unwrap_or("");

                    let ecosystem = args
                        .get("ecosystem")
                        .and_then(|e| e.as_str())
                        .unwrap_or("all");

                    let limit = args
                        .get("limit")
                        .and_then(|l| l.as_u64())
                        .unwrap_or(5) as usize;

                    let candidates = search_candidates(query, ecosystem, 8);
                    let evaluated = match evaluate_candidates(query, candidates, api_key) {
                        Ok(res) => res,
                        Err(err) => {
                            let err_resp = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32000,
                                    "message": err
                                }
                            });
                            writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                            stdout.flush()?;
                            continue;
                        }
                    };

                    let top_results: Vec<_> = evaluated.into_iter().take(limit).collect();
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&top_results)?
                                }
                            ]
                        }
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                    stdout.flush()?;
                } else {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method or tool '{}' not found", tool_name)
                        }
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                    stdout.flush()?;
                }
            }
            _ => {
                // Ignore notifications or unknown methods
            }
        }
    }

    Ok(())
}
