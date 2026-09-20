mod jev;
mod mcp;
mod search;
mod types;

use lexopt::prelude::*;
use std::process;
use std::time::Instant;

fn print_help() {
    let help = format!(
        r#"jev-scout {} - Zero-hallucination open-source repo and crate scout

USAGE:
    jev-scout [OPTIONS] <QUERY>

ARGS:
    <QUERY>                 Natural language description of what you are searching for

OPTIONS:
    -e, --ecosystem <NAME>  Target ecosystem: 'all' (default), 'github', or 'crates'
    -n, --limit <NUM>       Maximum results to show (default: 5)
    -j, --json              Output machine-readable JSON to stdout
        --no-filter         Show all candidates, skip weak-match filtering
        --mcp               Run as a stdio Model Context Protocol (MCP) server
    -h, --help              Print help information
    -v, --version           Print version information

EXAMPLES:
    jev-scout "fast sqlite tui in rust"
    jev-scout "headless browser without chromium" --ecosystem rust
    jev-scout "token efficient grep for coding agents" --json
"#,
        env!("CARGO_PKG_VERSION")
    );
    println!("{}", help);
}

fn main() {
    let mut query: Option<String> = None;
    let mut ecosystem = "all".to_string();
    let mut limit = 5usize;
    let mut json_mode = false;
    let mut no_filter = false;
    let mut mcp_mode = false;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    }) {
        match arg {
            Short('e') | Long("ecosystem") => {
                ecosystem = parser.value().unwrap().string().unwrap();
            }
            Short('n') | Long("limit") => {
                limit = parser.value().unwrap().parse().unwrap_or(5);
            }
            Short('j') | Long("json") => {
                json_mode = true;
            }
            Long("no-filter") => {
                no_filter = true;
            }
            Long("mcp") => {
                mcp_mode = true;
            }
            Short('h') | Long("help") => {
                print_help();
                process::exit(0);
            }
            Short('v') | Long("version") => {
                println!("jev-scout {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            Value(val) => {
                query = Some(val.string().unwrap_or_default());
            }
            _ => {
                eprintln!("Error: unexpected argument {:?}", arg);
                print_help();
                process::exit(1);
            }
        }
    }

    let api_key = match std::env::var("TYPESAFE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            eprintln!("Error: TYPESAFE_API_KEY environment variable is not set.");
            eprintln!("Please get an API key from https://typesafe.ai and export it:");
            eprintln!("  export TYPESAFE_API_KEY=\"your_key\"");
            process::exit(1);
        }
    };

    if mcp_mode {
        if let Err(e) = mcp::run_mcp_server(&api_key) {
            eprintln!("MCP Server Error: {}", e);
            process::exit(1);
        }
        return;
    }

    let query_str = match query {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            print_help();
            process::exit(1);
        }
    };

    let start_time = Instant::now();

    if !json_mode {
        println!("🔍 Scouting repositories for: \"{}\"", query_str);
    }

    let search_start = Instant::now();
    let candidates = search::search_candidates(&query_str, &ecosystem, 8);
    let search_ms = search_start.elapsed().as_millis();
    if candidates.is_empty() {
        if json_mode {
            println!("[]");
        } else {
            println!("No candidates found matching query.");
        }
        return;
    }

    let eval_start = Instant::now();
    let evaluated = match jev::evaluate_candidates(&query_str, candidates, &api_key) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    };
    let eval_ms = eval_start.elapsed().as_millis();
    let elapsed = start_time.elapsed();

    let top_results: Vec<_> = if no_filter {
        evaluated.into_iter().take(limit).collect()
    } else {
        jev::filter_weak(evaluated)
            .into_iter()
            .take(limit)
            .collect()
    };

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&top_results).unwrap());
        return;
    }

    println!(
        "Found {} candidates evaluated in {:.0}ms via TypeSafe Jev (search {:.0}ms + eval {:.0}ms):\n",
        top_results.len(),
        elapsed.as_millis(),
        search_ms,
        eval_ms
    );

    for (rank, item) in top_results.iter().enumerate() {
        let best_tag = if item.is_best_match {
            " \x1b[32;1m[BEST MATCH]\x1b[0m"
        } else {
            ""
        };

        println!(
            "\x1b[1m#{}\x1b[0m  \x1b[36;1m{}\x1b[0m{}",
            rank + 1,
            item.candidate.name,
            best_tag
        );
        println!(
            "    {} | \x1b[35m{}\x1b[0m | Updated: {}",
            if item.candidate.ecosystem == "crates.io" {
                format!("⬇ {}", format_num(item.candidate.downloads))
            } else {
                format!("⭐ {}", format_num(item.candidate.stars))
            },
            item.candidate.license,
            item.candidate
                .updated_at
                .chars()
                .take(10)
                .collect::<String>()
        );
        if !item.candidate.topics.is_empty() {
            println!(
                "    \x1b[90m{}\x1b[0m",
                item.candidate
                    .topics
                    .iter()
                    .map(|t| format!("#{}", t))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        println!("    {}", item.candidate.description);
        println!(
            "    Fit: \x1b[32m{:.1}/4.0\x1b[0m (Conf: {:.2}) | Active: \x1b[34m{:.0}%\x1b[0m",
            item.fit_score,
            item.confidence,
            item.is_modern * 100.0
        );
        println!("    URL: \x1b[4m{}\x1b[0m", item.candidate.url);
        println!(
            "    \x1b[90mCommand:\x1b[0m {}\n",
            item.candidate.install_cmd
        );
    }
}

fn format_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
