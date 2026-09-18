#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Candidate {
        pub id: String,
        pub name: String,
        pub description: String,
        pub url: String,
        pub stars: u64,
        pub license: String,
        pub updated_at: String,
        pub ecosystem: String,
        pub install_cmd: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EvaluatedCandidate {
        pub candidate: Candidate,
        pub fit_score: f64,
        pub is_modern: f64,
        pub confidence: f64,
        pub weighted_rank: f64,
        pub is_best_match: bool,
    }

    #[test]
    fn test_confidence_weighted_ranking() {
        let c1 = Candidate {
            id: "repo/a".to_string(),
            name: "repo-a".to_string(),
            description: "A fast database".to_string(),
            url: "https://github.com/repo/a".to_string(),
            stars: 500,
            license: "MIT".to_string(),
            updated_at: "2026-09-01".to_string(),
            ecosystem: "github".to_string(),
            install_cmd: "gh repo clone repo/a".to_string(),
        };

        let c2 = Candidate {
            id: "repo/b".to_string(),
            name: "repo-b".to_string(),
            description: "Another tool".to_string(),
            url: "https://github.com/repo/b".to_string(),
            stars: 200,
            license: "Apache-2.0".to_string(),
            updated_at: "2026-08-01".to_string(),
            ecosystem: "github".to_string(),
            install_cmd: "gh repo clone repo/b".to_string(),
        };

        // repo-a: fit 3.8, conf 0.95 -> weighted 3.61
        // repo-b: fit 4.0, conf 0.50 -> weighted 2.00 (penalized for low confidence)
        let mut candidates = vec![
            EvaluatedCandidate {
                candidate: c2,
                fit_score: 4.0,
                is_modern: 0.8,
                confidence: 0.50,
                weighted_rank: 4.0 * 0.50,
                is_best_match: false,
            },
            EvaluatedCandidate {
                candidate: c1,
                fit_score: 3.8,
                is_modern: 0.95,
                confidence: 0.95,
                weighted_rank: 3.8 * 0.95,
                is_best_match: true,
            },
        ];

        candidates.sort_by(|a, b| {
            b.weighted_rank
                .partial_cmp(&a.weighted_rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        assert_eq!(candidates[0].candidate.name, "repo-a");
        assert!(candidates[0].weighted_rank > candidates[1].weighted_rank);
    }

    #[test]
    fn test_jev_response_deserialization() {
        let raw_json = r#"{
            "model": "jev-1.13.0",
            "answers": {
                "best_match": {
                    "type": "choice",
                    "choice": "crate:ratatui",
                    "confidence": 0.98
                },
                "fit_0": {
                    "type": "score",
                    "score": 3.9,
                    "confidence": 0.95
                },
                "modern_0": {
                    "type": "noul",
                    "noul": 0.99
                }
            }
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(raw_json).expect("valid json");
        assert_eq!(parsed["answers"]["best_match"]["choice"], "crate:ratatui");
        assert_eq!(parsed["answers"]["fit_0"]["score"], 3.9);
        assert_eq!(parsed["answers"]["modern_0"]["noul"], 0.99);
    }
}
