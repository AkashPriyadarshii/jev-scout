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

    let evaluated = if candidates.len() <= 3 {
        evaluate_via_api(query, &candidates, api_key)?
    } else {
        // Jev drops questions past ~15 per call: chunk, fan out per chunk, merge.
        let mut all = Vec::new();
        for chunk in candidates.chunks(3) {
            all.extend(evaluate_via_api(query, chunk, api_key)?);
        }
        all
    };

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
                "instructions": format!("Relevance of '{}' to '{}'?", c.name, query),
                "criteria": [
                    "Unrelated or completely different functional domain",
                    "Loosely related or missing core requested features/language",
                    "Strong match satisfying most constraints",
                    "Exact architectural and functional match"
                ]
            }),
        );

        // Per-candidate Docs Score question
        questions.insert(
            format!("doc_{}", idx),
            json!({
                "type": "score",
                "instructions": format!("Docs quality of '{}'?", c.name),
                "criteria": [
                    "No usable description",
                    "One-line or vague description",
                    "Clear description with purpose",
                    "Excellent docs with examples and links"
                ]
            }),
        );

        // Per-candidate Modern/Active Maintenance Noul question
        questions.insert(
            format!("modern_{}", idx),
            json!({
                "type": "noul",
                "instructions": format!("Is '{}' actively maintained?", c.name),
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
            "instructions": format!("Single best for '{}'?", query),
            "criteria": choice_criteria
        }),
    );

    let payload = json!({
        "model": "jev-1.13.0",
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

    let answers = jev_res
        .answers
        .ok_or_else(|| "Jev response missing answers".to_string())?;
    let best_match_id = answers
        .get("best_match")
        .and_then(|v| v["choice"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            eprintln!("Warning: Jev best_match missing, no top pick flagged.");
            String::new()
        });

    let mut evaluated = Vec::new();

    for (idx, c) in candidates.iter().enumerate() {
        // Composite fit: relevance, maturity, docs combined with code weights.
        // A dropped dimension skips the candidate loudly, never defaults.
        let prefixes = ["fit", "doc"];
        let mut fit_score = 0.0;
        let mut confidence = 1.0f64;
        let mut ok = true;
        for (i, w) in crate::policy::FIT_WEIGHTS.iter().map(|(_, w)| w).enumerate() {
            let ans = answers.get(&format!("{}_{}", prefixes[i], idx));
            match ans.and_then(|v| v["score"].as_f64()).filter(|s| (0.0..=3.0).contains(s)) {
                Some(s) => fit_score += s * w,
                None => {
                    eprintln!("Warning: Jev {}_{} for '{}' malformed, skipping.", prefixes[i], idx, c.id);
                    ok = false;
                    break;
                }
            }
            confidence = confidence.min(
                ans.and_then(|v| v["confidence"].as_f64()).unwrap_or(0.0),
            );
        }
        if !ok {
            continue;
        }
        let modern_ans = answers.get(&format!("modern_{}", idx));
        let is_modern = match modern_ans.and_then(|v| v["noul"].as_f64()).filter(|n| (0.0..=1.0).contains(n)) {
            Some(n) => n,
            None => {
                eprintln!("Warning: Jev modern_{} for '{}' malformed, skipping.", idx, c.id);
                continue;
            }
        };

        let is_best = c.id == best_match_id;
        // Recency decay: stale (no push/update in 180 days) loses rank weight.
        // Date math belongs in host code, never in Jev (typesafe rule #2).
        let stale = is_stale_180d(if c.pushed_at.is_empty() {
            &c.updated_at
        } else {
            &c.pushed_at
        });
        let weighted_rank = fit_score * confidence * if stale { crate::policy::STALE_PENALTY } else { 1.0 };

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

/// Drop weak matches below policy floors so low-quality picks never reach the user.
pub fn filter_weak(mut evaluated: Vec<EvaluatedCandidate>) -> Vec<EvaluatedCandidate> {
    evaluated.retain(|e| e.fit_score >= crate::policy::MIN_FIT && e.confidence >= crate::policy::MIN_CONFIDENCE);
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
    // JDN of unix epoch is 2440588; now_days counts from epoch.
    let now_days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|s| s.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    now_days - (days - 2440588) > crate::policy::STALE_DAYS
}

fn julian_day(y: i64, m: i64, d: i64) -> i64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = y / 100;
    let b = 2 - a + a / 4;
    (36525 * (y + 4716)) / 100 + (306 * (m + 1)) / 10 + d + b - 1524
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_boundary() {
        assert!(!is_stale_180d(""));
        assert!(!is_stale_180d("not-a-date"));
        assert!(!is_stale_180d("2026-09-21T00:00:00Z"));
        assert!(is_stale_180d("2020-01-01T00:00:00Z"));
        assert!(is_stale_180d("2020-01-01"));
    }

    #[test]
    fn weak_filter_thresholds() {
        let mk = |fit: f64, conf: f64| EvaluatedCandidate {
            candidate: crate::types::Candidate {
                id: "x".into(),
                name: "x".into(),
                description: "".into(),
                url: "".into(),
                stars: 0,
                downloads: 0,
                license: "".into(),
                updated_at: "".into(),
                pushed_at: "".into(),
                language: "".into(),
                topics: vec![],
                ecosystem: "".into(),
                install_cmd: "".into(),
            },
            fit_score: fit,
            is_modern: 1.0,
            confidence: conf,
            weighted_rank: fit * conf,
            is_best_match: false,
        };
        let out = filter_weak(vec![mk(2.5, 0.9), mk(1.4, 0.9), mk(2.5, 0.4)]);
        assert_eq!(out.len(), 1);
    }
}
