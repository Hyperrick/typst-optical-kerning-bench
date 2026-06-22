mod basic;
mod constraints;
mod evaluate;
mod geometry;
mod guarded;
mod math;
mod run_context;
mod types;

#[cfg(test)]
mod tests;

pub use evaluate::{
    evaluate_pair, evaluate_pair_with_config, evaluate_shaped_pair_with_config,
    evaluate_shaped_run_with_config,
};
pub use types::{Algorithm, AlgorithmOutput, AlgorithmSet, EvaluationConfig};
