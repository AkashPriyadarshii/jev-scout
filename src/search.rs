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

pub fn search_github(query: &str, limit: usize) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let encoded_query = query.replace(' ', "+");
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

    if let Ok(response) = request.call() {
        if let Ok(json) = response.into_json::<serde_json::Value>() {
            if let Some(items) = json["items"].as_array() {
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
    }

    candidates
}

pub fn search_crates_io(query: &str, limit: usize) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let encoded_query = query.replace(' ', "+");
    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page={}",
        encoded_query,
        limit.max(4)
    );

    let request = ureq::get(&url)
        .set("User-Agent", "jev-scout/0.1.0 (akashpriyadarshii)")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(5));

    if let Ok(response) = request.call() {
        if let Ok(json) = response.into_json::<serde_json::Value>() {
            if let Some(crates) = json["crates"].as_array() {
                for item in crates.iter().take(limit) {
                    let name = item["name"].as_str().unwrap_or("").to_string();
                    let description = item["description"]
                        .as_str()
                        .unwrap_or("No description provided")
                        .to_string();
                    let downloads = item["downloads"].as_u64().unwrap_or(0);
                    let updated_at = item["updated_at"].as_str().unwrap_or("").to_string();
                    let url = format!("https://crates.io/crates/{}", name);

                    if !name.is_empty() {
                        candidates.push(Candidate {
                            id: format!("crate:{}", name),
                            name: name.clone(),
                            description,
                            url,
                            stars: 0,
                            downloads,
                            license: "Rust / Crates.io".to_string(),
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
    }

    candidates
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
        "github" => search_github(query, total_limit),
        "crates" | "crates.io" | "rust" => search_crates_io(query, total_limit),
        _ => {
            // Default "all": fetch both in parallel
            let gh_limit = total_limit.div_ceil(2);
            let crates_limit = total_limit / 2;
            let (gh_q, cr_q) = (query.to_string(), query.to_string());
            let gh = std::thread::spawn(move || search_github(&gh_q, gh_limit));
            let cr = std::thread::spawn(move || search_crates_io(&cr_q, crates_limit));
            let mut both = gh.join().unwrap_or_default();
            both.extend(cr.join().unwrap_or_default());
            both
        }
    };

    cache_put(&key, &results);
    results
}
