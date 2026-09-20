use crate::types::{Candidate, EvaluatedCandidate, JevResponse};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ponytail: Jev scores are deterministic per (query, candidate set); cache 60s
// so repeated/MCP calls skip the ~1.1s API floor entirely.
const EVAL_CACHE_TTL: Duration = Duration::from_secs(60);
type EvalCache = HashMap<String, (Instant, Vec<EvaluatedCandidate>)>;
static EVAL_CACHE: Mutex<Option<EvalCache>> = Mutex::new(None);

pub fn evaluate_candidates(
    query: &str,
    candidates: Vec<Candidate>,
    api_key: &str,
) -> Result<Vec<EvaluatedCandidate>, String> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Deterministic cache key: query + candidate ids
    let mut ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    let key = format!("{}|{}", query.trim().to_lowercase(), ids.join(","));

    if let Ok(guard) = EVAL_CACHE.lock() {
        if let Some(map) = guard.as_ref() {
            if let Some((ts, hit)) = map.get(&key) {
                if ts.elapsed() < EVAL_CACHE_TTL {
                    return Ok(hit.clone());
                }
            }
        }
    }

    let evaluated = evaluate_via_api(query, &candidates, api_key)?;

    if let Ok(mut guard) = EVAL_CACHE.lock() {
        let map = guard.get_or_insert_with(EvalCache::new);
        map.retain(|_, (ts, _)| ts.elapsed() < EVAL_CACHE_TTL);
        map.insert(key, (Instant::now(), evaluated.clone()));
    }

    Ok(evaluated)
}

fn evaluate_via_api(
    query: &str,
    candidates: &[Candidate],
    api_key: &str,
) -> Result<Vec<EvaluatedCandidate>, String> {
    // Build Choice criteria for best_match
    let mut choice_criteria = serde_json::Map::new();
    let mut questions = serde_json::Map::new();

    for (idx, c) in candidates.iter().enumerate() {
        choice_criteria.insert(
            c.id.clone(),
            json!(format!("{}: {}", c.name, c.description)),
        );

        // Per-candidate Fit Score question (criteria is a list of levels)
        questions.insert(
            format!("fit_{}", idx),
            json!({
                "type": "score",
                "instructions": format!("Rate how well '{}' ({}) satisfies the user prompt: '{}'", c.name, c.description, query),
                "criteria": [
                    "Unrelated or completely different functional domain",
                    "Loosely related or missing core requested features/language",
                    "Strong match satisfying most constraints",
                    "Exact architectural and functional match"
                ]
            }),
        );

        // Per-candidate Modern/Active Maintenance Noul question
        questions.insert(
            format!("modern_{}", idx),
            json!({
                "type": "noul",
                "instructions": format!("Based on metadata (stars: {}, downloads: {}, pushed/updated: {}, language: {}, topics: {:?}), is '{}' an actively maintained modern tool?", c.stars, c.downloads, c.pushed_at, c.language, c.topics, c.name),
                "criteria": {
                    "true": "Actively maintained with modern tooling",
                    "false": "Deprecated, abandoned, or legacy code"
                }
            }),
        );
    }

    // Best match Choice question
    questions.insert(
        "best_match".to_string(),
        json!({
            "type": "choice",
            "instructions": format!("Which candidate is the single best recommendation for: '{}'?", query),
            "criteria": choice_criteria
        }),
    );

    let payload = json!({
        "model": "jev-latest",
        "state": {
            "query": query,
            "candidate_count": candidates.len(),
            // Trim derivable fields (install_cmd, url, ecosystem) to cut tokens
            // and reduce context rot: Jev only needs what it judges on.
            "candidates": candidates.iter().map(|c| json!({
                "id": c.id,
                "name": c.name,
                "description": c.description,
                "stars": c.stars,
                "downloads": c.downloads,
                "license": c.license,
                "updated_at": c.updated_at,
                "pushed_at": c.pushed_at,
                "language": c.language,
                "topics": c.topics
            })).collect::<Vec<_>>()
        },
        "questions": questions
    });

    let response = match ureq::post("https://api.typesafe.ai/v1/systemone")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(8))
        .send_json(payload)
    {
        Ok(resp) => resp,
        Err(ureq::Error::Status(code, resp)) => {
            let err_body = resp.into_string().unwrap_or_default();
            return Err(format!("TypeSafe Jev API HTTP {}: {}", code, err_body));
        }
        Err(e) => return Err(format!("Failed to call TypeSafe Jev API: {}", e)),
    };

    let jev_res: JevResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse Jev API response: {}", e))?;

    let answers = jev_res.answers.unwrap_or_default();
    let best_match_id = answers
        .get("best_match")
        .and_then(|v| v["choice"].as_str())
        .unwrap_or("")
        .to_string();

    let mut evaluated = Vec::new();

    for (idx, c) in candidates.iter().enumerate() {
        let fit_ans = answers.get(&format!("fit_{}", idx));
        let modern_ans = answers.get(&format!("modern_{}", idx));

        let fit_score = fit_ans.and_then(|v| v["score"].as_f64()).unwrap_or(2.5);

        let confidence = fit_ans
            .and_then(|v| v["confidence"].as_f64())
            .unwrap_or(0.8);

        let is_modern = modern_ans.and_then(|v| v["noul"].as_f64()).unwrap_or(0.7);

        let is_best = c.id == best_match_id;
        // Recency decay: stale (no push/update in 180 days) loses rank weight.
        // Date math belongs in host code, never in Jev (typesafe rule #2).
        let stale = is_stale_180d(if c.pushed_at.is_empty() {
            &c.updated_at
        } else {
            &c.pushed_at
        });
        let weighted_rank = fit_score * confidence * if stale { 0.9 } else { 1.0 };

        evaluated.push(EvaluatedCandidate {
            candidate: c.clone(),
            fit_score,
            is_modern,
            confidence,
            weighted_rank,
            is_best_match: is_best,
        });
    }

    // Sort descending by confidence-weighted score
    evaluated.sort_by(|a, b| {
        b.weighted_rank
            .partial_cmp(&a.weighted_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(evaluated)
}

/// Drop weak matches: fit below 2.5 or confidence below 0.5 gets filtered out
/// so low-quality recommendations never reach the user. Honest default.
pub fn filter_weak(mut evaluated: Vec<EvaluatedCandidate>) -> Vec<EvaluatedCandidate> {
    evaluated.retain(|e| e.fit_score >= 2.5 && e.confidence >= 0.5);
    evaluated
}

/// True if an ISO-8601 date ("YYYY-MM-DD...") is older than 180 days.
/// Julian-day arithmetic, no chrono dependency, deterministic.
fn is_stale_180d(iso: &str) -> bool {
    let date = iso.split('T').next().unwrap_or("");
    if date.len() < 10 {
        return false;
    }
    let y: i64 = date[0..4].parse().unwrap_or(0);
    let m: i64 = date[5..7].parse().unwrap_or(0);
    let d: i64 = date[8..10].parse().unwrap_or(0);
    if y == 0 || m == 0 || d == 0 {
        return false;
    }
    let days = julian_day(y, m, d);
    let now_days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|s| s.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    now_days - days > 180
}

fn julian_day(y: i64, m: i64, d: i64) -> i64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = y / 100;
    let b = 2 - a + a / 4;
    (36525 * (y + 4716)) / 100 + (306 * (m + 1)) / 10 + d + b - 1524
}
