mod basic;
mod capital_context;
mod constraints;
mod digit_context;
mod evaluate;
mod geometry;
mod guarded;
mod math;
mod run_context;
mod sans_context;
mod types;

#[cfg(test)]
mod tests;

pub use evaluate::{
    evaluate_pair, evaluate_pair_with_config, evaluate_shaped_pair_with_config,
    evaluate_shaped_run_with_config,
};
pub use types::{Algorithm, AlgorithmOutput, AlgorithmSet, EvaluationConfig};
