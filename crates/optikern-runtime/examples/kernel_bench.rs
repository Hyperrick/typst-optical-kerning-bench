use std::hint::black_box;
use std::time::{Duration, Instant};

use optikern_runtime::{
    GlyphClass, PairEvidence, SideShape, compact_guarded, fallback_only, nearest_contour,
};

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1_000_000);
    let evidence = evidence_set();

    let nearest = measure(iterations, &evidence, nearest_contour);
    let fallback = measure(iterations, &evidence, fallback_only);
    let compact = measure(iterations, &evidence, compact_guarded);

    println!(
        concat!(
            "{{\n",
            "  \"iterations\": {},\n",
            "  \"pairsPerIteration\": {},\n",
            "  \"nearestNsPerPair\": {:.3},\n",
            "  \"fallbackNsPerPair\": {:.3},\n",
            "  \"compactNsPerPair\": {:.3}\n",
            "}}"
        ),
        iterations,
        evidence.len(),
        ns_per_pair(nearest, iterations, evidence.len()),
        ns_per_pair(fallback, iterations, evidence.len()),
        ns_per_pair(compact, iterations, evidence.len()),
    );
}

fn measure(
    iterations: usize,
    evidence: &[PairEvidence],
    evaluate: fn(PairEvidence) -> f32,
) -> Duration {
    for pair in evidence {
        black_box(evaluate(black_box(*pair)));
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for pair in evidence {
            black_box(evaluate(black_box(*pair)));
        }
    }
    start.elapsed()
}

fn ns_per_pair(duration: Duration, iterations: usize, pairs: usize) -> f64 {
    duration.as_nanos() as f64 / (iterations * pairs) as f64
}

fn evidence_set() -> [PairEvidence; 8] {
    let base = PairEvidence {
        left: GlyphClass::Lower,
        right: GlyphClass::Lower,
        metric_delta: 0.0,
        optical_delta: -0.04,
        nearest_delta: 0.0,
        target_gap: 0.23,
        gap_mad: 0.05,
        min_gap: 0.14,
        robust_gap: 0.29,
        x_height: 0.52,
        cap_height: 0.72,
        left_side: SideShape {
            roundness: 0.04,
            stemness: 0.40,
        },
        right_side: SideShape {
            roundness: 0.02,
            stemness: 0.70,
        },
        right_top_left_overhang: 0.0,
        monospaced: false,
    };

    [
        base,
        PairEvidence {
            left: GlyphClass::Upper,
            right: GlyphClass::Upper,
            metric_delta: -0.08,
            optical_delta: -0.11,
            ..base
        },
        PairEvidence {
            left: GlyphClass::Lower,
            right: GlyphClass::Upper,
            optical_delta: -0.09,
            right_top_left_overhang: 0.24,
            ..base
        },
        PairEvidence {
            left: GlyphClass::Digit,
            right: GlyphClass::Digit,
            optical_delta: -0.02,
            ..base
        },
        PairEvidence {
            left: GlyphClass::Upper,
            right: GlyphClass::Punctuation,
            metric_delta: -0.06,
            ..base
        },
        PairEvidence {
            min_gap: -0.08,
            nearest_delta: 0.12,
            optical_delta: 0.02,
            ..base
        },
        PairEvidence {
            target_gap: 0.28,
            x_height: 0.48,
            cap_height: 0.72,
            metric_delta: -0.11,
            optical_delta: -0.08,
            ..base
        },
        PairEvidence {
            monospaced: true,
            metric_delta: 0.0,
            ..base
        },
    ]
}
