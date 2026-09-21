//! One place for every threshold. Tune here, nowhere else.

/// Minimum fit score (0-3 scale, probability-weighted level position) to survive.
pub const MIN_FIT: f64 = 1.5;
/// Minimum answer confidence to survive filtering.
pub const MIN_CONFIDENCE: f64 = 0.5;
/// Stale penalty: candidates untouched this long lose 10% rank weight.
pub const STALE_DAYS: i64 = 180;
pub const STALE_PENALTY: f64 = 0.9;

/// Composite fit dimensions and code-owned weights. Sums to 1.0.
/// Maturity rides the deterministic stale penalty, not a question.
pub const FIT_WEIGHTS: &[(&str, f64)] = &[
    ("fit", 0.7),
    ("doc", 0.3),
];
