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

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct JevRequest {
    pub model: String,
    pub state: serde_json::Value,
    pub questions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct JevResponse {
    #[allow(dead_code)]
    pub model: Option<String>,
    pub answers: Option<serde_json::Map<String, serde_json::Value>>,
}
