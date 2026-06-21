use optikern_core::AlgorithmSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchReport {
    pub schema_version: u32,
    pub font_manifest_commit: String,
    pub fonts: Vec<BenchFont>,
    pub pair_count: usize,
    #[serde(default)]
    pub word_count: usize,
    pub results: Vec<AlgorithmSet>,
    pub failures: Vec<BenchFailure>,
    pub runtime_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchFont {
    pub id: String,
    pub family: String,
    pub category: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchFailure {
    pub font_id: String,
    pub pair: String,
    pub reason: String,
}
