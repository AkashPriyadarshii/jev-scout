use crate::types::Candidate;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ponytail: whole-map TTL cache (no LRU eviction). Memory is bounded by distinct
// queries per 60s window; switch to true LRU if MCP sessions show unbounded growth.
const CACHE_TTL: Duration = Duration::from_secs(60);
type Cache = HashMap<String, (Instant, Vec<Candidate>)>;
static SEARCH_CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn cache_get(key: &str) -> Option<Vec<Candidate>> {
    let guard = SEARCH_CACHE.lock().ok()?;
    let map = guard.as_ref()?;
    let (ts, hits) = map.get(key)?;
    if ts.elapsed() > CACHE_TTL {
        return None;
    }
    Some(hits.clone())
}

fn cache_put(key: &str, value: &[Candidate]) {
    if let Ok(mut guard) = SEARCH_CACHE.lock() {
        let map = guard.get_or_insert_with(Cache::new);
        map.retain(|_, (ts, _)| ts.elapsed() < CACHE_TTL);
        map.insert(key.to_string(), (Instant::now(), value.to_vec()));
    }
}

pub fn get_github_token() -> Option<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            return Some(token.trim().to_string());
        }
    }
    if let Ok(token) = std::env::var("GH_TOKEN") {
        if !token.trim().is_empty() {
            return Some(token.trim().to_string());
        }
    }
    // Fallback: query gh CLI
    if let Ok(output) = Command::new("gh").args(["auth", "token"]).output() {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

/// Percent-encode a query string. Keeps unreserved + space-as-plus.
fn encode_query(query: &str) -> String {
    let mut out = String::new();
    for b in query.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn search_github(query: &str, limit: usize) -> Result<Vec<Candidate>, String> {
    let mut candidates = Vec::new();
    let encoded_query = encode_query(query);
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page={}",
        encoded_query,
        limit.max(4)
    );

    let mut request = ureq::get(&url)
        .set("User-Agent", "jev-scout/0.1.0")
        .set("Accept", "application/vnd.github.v3+json")
        .timeout(Duration::from_secs(5));

    if let Some(token) = get_github_token() {
        request = request.set("Authorization", &format!("Bearer {}", token));
    }

    match request.call() {
        Err(ureq::Error::Status(403, _)) => {
            return Err("GitHub search rate limited (403). Run `gh auth login`, then retry.".to_string())
        }
        Err(ureq::Error::Status(429, _)) => {
            return Err("GitHub search rate limited (429). Wait a minute, then retry.".to_string())
        }
        Err(ureq::Error::Transport(e)) if e.to_string().contains("timed out") => {
            return Err("GitHub search timed out. Retry on a better connection.".to_string())
        }
        Err(e) => return Err(format!("GitHub search failed: {}", e)),
        Ok(response) => {
            let json: serde_json::Value = response
                .into_json()
                .map_err(|e| format!("GitHub returned bad JSON: {}", e))?;
            let items = json["items"]
                .as_array()
                .ok_or_else(|| "GitHub response has no items".to_string())?;
            for item in items.iter().take(limit) {
                let full_name = item["full_name"].as_str().unwrap_or("").to_string();
                let name = item["name"].as_str().unwrap_or("").to_string();
                let description = item["description"]
                    .as_str()
                    .unwrap_or("No description provided")
                    .to_string();
                let html_url = item["html_url"].as_str().unwrap_or("").to_string();
                let stars = item["stargazers_count"].as_u64().unwrap_or(0);
                let license = item["license"]["spdx_id"]
                    .as_str()
                    .unwrap_or("None")
                    .to_string();
                let updated_at = item["updated_at"].as_str().unwrap_or("").to_string();

                if !full_name.is_empty() {
                    let topics: Vec<String> = item["topics"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    candidates.push(Candidate {
                        id: full_name.clone(),
                        name,
                        description,
                        url: html_url,
                        stars,
                        downloads: 0,
                        license,
                        updated_at,
                        pushed_at: item["pushed_at"].as_str().unwrap_or("").to_string(),
                        language: item["language"].as_str().unwrap_or("").to_string(),
                        topics,
                        ecosystem: "github".to_string(),
                        install_cmd: format!("gh repo clone {}", full_name),
                    });
                }
            }
        }
    }

    Ok(candidates)
}

pub fn search_crates_io(query: &str, limit: usize) -> Result<Vec<Candidate>, String> {
    let mut candidates = Vec::new();
    let encoded_query = encode_query(query);
    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page={}",
        encoded_query,
        limit.max(4)
    );

    let request = ureq::get(&url)
        .set("User-Agent", "jev-scout/0.1.0 (akashpriyadarshii)")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(5));

    match request.call() {
        Err(ureq::Error::Status(429, _)) => {
            return Err("crates.io rate limited (429). Wait a minute, then retry.".to_string())
        }
        Err(ureq::Error::Transport(e)) if e.to_string().contains("timed out") => {
            return Err("crates.io timed out. Retry on a better connection.".to_string())
        }
        Err(e) => return Err(format!("crates.io search failed: {}", e)),
        Ok(response) => {
            let json: serde_json::Value = response
                .into_json()
                .map_err(|e| format!("crates.io returned bad JSON: {}", e))?;
            let crates = json["crates"]
                .as_array()
                .ok_or_else(|| "crates.io response has no crates".to_string())?;
            for item in crates.iter().take(limit) {
                let name = item["name"].as_str().unwrap_or("").to_string();
                let description = item["description"]
                    .as_str()
                    .unwrap_or("No description provided")
                    .to_string();
                let downloads = item["downloads"].as_u64().unwrap_or(0);
                let updated_at = item["updated_at"].as_str().unwrap_or("").to_string();
                let url = format!("https://crates.io/crates/{}", name);
                let license = item["license"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Unknown")
                    .to_string();

                if !name.is_empty() {
                    candidates.push(Candidate {
                        id: format!("crate:{}", name),
                        name: name.clone(),
                        description,
                        url,
                        stars: 0,
                        downloads,
                        license,
                        updated_at,
                        pushed_at: "".to_string(),
                        language: "Rust".to_string(),
                        topics: Vec::new(),
                        ecosystem: "crates.io".to_string(),
                        install_cmd: format!("cargo add {}", name),
                    });
                }
            }
        }
    }

    Ok(candidates)
}

/// DuckDuckGo HTML web results: docs, blogs, tutorials beyond code homes.
/// Empty parse = loud error, never fake rows (bot challenges return 200).
pub fn search_duckduckgo(query: &str, limit: usize) -> Result<Vec<Candidate>, String> {
    let mut candidates = Vec::new();
    let form = format!("q={}&kl=us-en", encode_query(query));
    let html = ureq::post("https://html.duckduckgo.com/html/")
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(Duration::from_secs(8))
        .send_string(&form)
        .map_err(|e| format!("DuckDuckGo search failed: {}", e))?
        .into_string()
        .map_err(|e| format!("DuckDuckGo returned bad body: {}", e))?;

    let lower = html.to_lowercase();
    if lower.contains("anomaly") || lower.contains("captcha") {
        return Err("DuckDuckGo bot challenge, no results recorded.".to_string());
    }

    for block in html.split("result__title").skip(1).take(limit) {
        if let Some(c) = parse_ddg_block(block) {
            candidates.push(c);
        }
    }

    if candidates.is_empty() {
        return Err("DuckDuckGo returned no parseable results, possibly blocked.".to_string());
    }
    Ok(candidates)
}

fn parse_ddg_block(block: &str) -> Option<Candidate> {
    let href = block.find("href=\"").and_then(|s| {
        let rest = &block[s + 6..];
        rest.find('"').map(|e| rest[..e].to_string())
    })?;
    let url = if let Some(pos) = href.find("uddg=") {
        let rem = &href[pos + 5..];
        rem[..rem.find('&').unwrap_or(rem.len())].to_string()
    } else if href.starts_with("http") {
        href
    } else {
        return None;
    };
    let title = block
        .find('>')
        .map(|s| {
            let rest = &block[s + 1..];
            rest[..rest.find('<').unwrap_or(rest.len())]
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if title.is_empty() || url.is_empty() {
        return None;
    }
    Some(Candidate {
        id: format!("web:{}", url),
        name: title,
        description: "Web result via DuckDuckGo".to_string(),
        url: url.clone(),
        stars: 0,
        downloads: 0,
        license: "Web".to_string(),
        updated_at: "".to_string(),
        pushed_at: "".to_string(),
        language: "".to_string(),
        topics: Vec::new(),
        ecosystem: "web".to_string(),
        install_cmd: url,
    })
}

pub fn search_candidates(query: &str, ecosystem: &str, total_limit: usize) -> Vec<Candidate> {    let key = format!(
        "{}|{}|{}",
        query.trim().to_lowercase(),
        ecosystem,
        total_limit
    );
    if let Some(hit) = cache_get(&key) {
        return hit;
    }

    let results = match ecosystem.to_lowercase().as_str() {
        "github" => search_github(query, total_limit).unwrap_or_else(|e| {
            eprintln!("Warning: {}", e);
            Vec::new()
        }),
        "crates" | "crates.io" | "rust" => search_crates_io(query, total_limit).unwrap_or_else(|e| {
            eprintln!("Warning: {}", e);
            Vec::new()
        }),
        "web" => search_duckduckgo(query, total_limit).unwrap_or_else(|e| {
            eprintln!("Warning: {}", e);
            Vec::new()
        }),
        _ => {
            // Default "all": fetch all three sources in parallel
            let gh_limit = total_limit.div_ceil(3).max(2);
            let crates_limit = total_limit.div_ceil(3).max(2);
            let web_limit = total_limit.div_ceil(3).max(2);
            let (gh_q, cr_q, web_q) = (query.to_string(), query.to_string(), query.to_string());
            let gh = std::thread::spawn(move || search_github(&gh_q, gh_limit));
            let cr = std::thread::spawn(move || search_crates_io(&cr_q, crates_limit));
            let web = std::thread::spawn(move || search_duckduckgo(&web_q, web_limit));
            let mut all = gh
                .join()
                .map(|r| r.unwrap_or_else(|e| {
                    eprintln!("Warning: {}", e);
                    Vec::new()
                }))
                .unwrap_or_default();
            for handle in [cr, web] {
                all.extend(
                    handle
                        .join()
                        .map(|r| r.unwrap_or_else(|e| {
                            eprintln!("Warning: {}", e);
                            Vec::new()
                        }))
                        .unwrap_or_default(),
                );
            }
            all
        }
    };

    cache_put(&key, &results);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_specials() {
        assert_eq!(encode_query("rust grep"), "rust+grep");
        assert_eq!(encode_query("a&b"), "a%26b");
        assert_eq!(encode_query("c#"), "c%23");
        assert_eq!(encode_query("tokio-rs"), "tokio-rs");
    }

    #[test]
    fn parses_duckduckgo_fixture() {
        let good = r#"" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage&rut=x">Example Page</a>"#;
        let c = parse_ddg_block(good).expect("fixture must parse");
        assert_eq!(c.name, "Example Page");
        assert!(c.url.contains("example.com"));
        assert_eq!(c.ecosystem, "web");
        assert!(parse_ddg_block("no link here").is_none());
        assert!(parse_ddg_block(r#"" href="/relative/path">T</a>"#).is_none());
    }
}
