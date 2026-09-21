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

pub fn search_candidates(query: &str, ecosystem: &str, total_limit: usize) -> Vec<Candidate> {
    let key = format!(
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
        _ => {
            // Default "all": fetch both in parallel
            let gh_limit = total_limit.div_ceil(2);
            let crates_limit = total_limit / 2;
            let (gh_q, cr_q) = (query.to_string(), query.to_string());
            let gh = std::thread::spawn(move || search_github(&gh_q, gh_limit));
            let cr = std::thread::spawn(move || search_crates_io(&cr_q, crates_limit));
            let mut both = gh
                .join()
                .map(|r| r.unwrap_or_else(|e| {
                    eprintln!("Warning: {}", e);
                    Vec::new()
                }))
                .unwrap_or_default();
            both.extend(
                cr.join()
                    .map(|r| r.unwrap_or_else(|e| {
                        eprintln!("Warning: {}", e);
                        Vec::new()
                    }))
                    .unwrap_or_default(),
            );
            both
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
}
