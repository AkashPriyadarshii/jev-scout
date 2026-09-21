//! Remembers the top pick per query so repeat runs show movement.
//! JSON file under ~/.jev-scout/, std only, no new dependency.

use std::collections::HashMap;

fn history_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|h| {
        let mut p = std::path::PathBuf::from(h);
        p.push(".jev-scout/history.json");
        p
    })
}

fn load() -> HashMap<String, String> {
    history_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Record this run's top pick. Returns the previous top pick, if different run.
pub fn record(query: &str, top_id: &str) -> Option<String> {
    let mut map = load();
    let prev = map.get(query).cloned().filter(|p| p != top_id);
    map.insert(query.to_string(), top_id.to_string());
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string(&map) {
            let _ = std::fs::write(path, s);
        }
    }
    prev
}
