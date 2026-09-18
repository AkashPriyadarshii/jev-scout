use crate::types::{Candidate, EvaluatedCandidate, JevResponse};
use serde_json::json;
use std::time::Duration;

pub fn evaluate_candidates(
    query: &str,
    candidates: Vec<Candidate>,
    api_key: &str,
) -> Result<Vec<EvaluatedCandidate>, String> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

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
                "instructions": format!("Based on metadata (stars: {}, updated: {}), is '{}' an actively maintained modern tool?", c.stars, c.updated_at, c.name),
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
            "candidates": candidates
        },
        "questions": questions
    });

    let response = match ureq::post("https://api.typesafe.ai/v1/systemone")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(8))
        .send_json(payload) {
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

    for (idx, c) in candidates.into_iter().enumerate() {
        let fit_ans = answers.get(&format!("fit_{}", idx));
        let modern_ans = answers.get(&format!("modern_{}", idx));

        let fit_score = fit_ans
            .and_then(|v| v["score"].as_f64())
            .unwrap_or(2.5);

        let confidence = fit_ans
            .and_then(|v| v["confidence"].as_f64())
            .unwrap_or(0.8);

        let is_modern = modern_ans
            .and_then(|v| v["noul"].as_f64())
            .unwrap_or(0.7);

        let is_best = c.id == best_match_id;
        let weighted_rank = fit_score * confidence;

        evaluated.push(EvaluatedCandidate {
            candidate: c,
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
