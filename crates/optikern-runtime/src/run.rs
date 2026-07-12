use crate::{DEAD_ZONE, RunPair, normalize, preservation::preserve_metric_prior};

pub fn compact_guarded_run(pairs: &mut [RunPair], target_gap: f32, gap_mad: f32, x_to_cap: f32) {
    let sans = target_gap <= 0.235 && x_to_cap >= 0.72;
    let compact_sans = sans && target_gap <= 0.210;
    let upper_pairs = pairs.iter().filter(|pair| pair.is_upper_upper()).count();
    let strong_upper = pairs
        .iter()
        .filter(|pair| pair.is_upper_upper() && pair.metric_delta < -0.050)
        .count();
    let strong_mixed = pairs
        .iter()
        .filter(|pair| pair.is_mixed_case() && pair.metric_delta < -0.050)
        .count();
    let mixed_pairs = pairs.iter().filter(|pair| pair.is_mixed_case()).count();
    let lower_pairs = pairs.iter().filter(|pair| pair.is_lower_lower()).count();
    let letter_pairs = pairs.iter().filter(|pair| pair.is_lower_involved()).count();
    let metricless_lower = pairs
        .iter()
        .filter(|pair| pair.is_lower_lower() && pair.is_metricless())
        .count();
    let optical_tightening_lower = pairs
        .iter()
        .filter(|pair| pair.is_lower_lower() && pair.optical_delta < -DEAD_ZONE)
        .count();
    let multi_char_lower = pairs
        .iter()
        .filter(|pair| pair.is_lower_lower() && pair.has_multi_char_cluster())
        .count();
    let connected_lower = pairs
        .iter()
        .filter(|pair| pair.is_lower_lower() && pair.min_gap < -0.020)
        .count();
    let sans_lower_run = sans
        && lower_pairs >= 5
        && lower_pairs == letter_pairs
        && metricless_lower >= lower_pairs.saturating_sub(2);
    let serif = SerifRun {
        target_gap,
        gap_mad,
        wide_lower: !sans
            && target_gap >= 0.220
            && x_to_cap < 0.72
            && lower_pairs >= 4
            && connected_lower == 0,
        ligature_lower: !sans
            && target_gap >= 0.240
            && x_to_cap < 0.72
            && lower_pairs >= 6
            && multi_char_lower > 0
            && metricless_lower >= lower_pairs.saturating_sub(1)
            && connected_lower == 0,
    };

    let digit_run_pairs = pairs.iter().filter(|pair| pair.is_digit_run()).count();
    let metricful_digits = pairs
        .iter()
        .filter(|pair| pair.is_digit_run() && !pair.is_metricless())
        .count();
    let loose_digits = pairs
        .iter()
        .filter(|pair| {
            pair.is_digit_digit()
                && pair.is_metricless()
                && pair.min_gap >= (target_gap * 0.44).clamp(0.095, 0.125)
        })
        .count();
    let script = ScriptRun::from_pairs(pairs, target_gap, x_to_cap);

    for pair in pairs {
        adjust_script_run(pair, target_gap, &script);
        relax_sans_lower_run(
            pair,
            compact_sans,
            sans_lower_run,
            lower_pairs,
            optical_tightening_lower,
        );
        adjust_case_run(
            pair,
            sans,
            compact_sans,
            upper_pairs,
            strong_upper,
            strong_mixed,
            mixed_pairs,
            lower_pairs,
        );
        adjust_digit_run(
            pair,
            sans,
            target_gap,
            digit_run_pairs,
            metricful_digits,
            loose_digits,
        );
        adjust_serif_cap_run(pair, sans, target_gap, upper_pairs, strong_upper);
        adjust_wide_serif_lower_run(pair, &serif);
        pair.delta = preserve_metric_prior(pair.metric_delta, normalize(pair.delta));
    }
}

fn relax_sans_lower_run(
    pair: &mut RunPair,
    compact_sans: bool,
    active: bool,
    lower_pairs: usize,
    optical_tightening_lower: usize,
) {
    if !active || !pair.is_lower_lower() || pair.delta >= 0.020 || pair.metric_delta.abs() >= 0.025
    {
        return;
    }

    let target = if compact_sans {
        if optical_tightening_lower == 0 && lower_pairs >= 7 {
            0.011
        } else if optical_tightening_lower > 0 && lower_pairs < 8 {
            -0.035
        } else {
            return;
        }
    } else if lower_pairs >= 8 {
        -0.014
    } else {
        -0.010
    };
    pair.delta = pair.delta.max(target);
}

#[derive(Debug, Clone, Copy)]
struct ScriptRun {
    connected: bool,
    ligature: bool,
    ligature_opening: f32,
    mixed: bool,
    severe_metricless_mixed: bool,
    lower_pairs: usize,
    lower_compaction: bool,
    upper_run: bool,
}

impl ScriptRun {
    fn from_pairs(pairs: &[RunPair], target_gap: f32, x_to_cap: f32) -> Self {
        let script_profile = target_gap <= 0.205 && x_to_cap < 0.72;
        let letter_pairs = pairs.iter().filter(|pair| pair.is_lower_involved()).count();
        let connected_pairs = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.min_gap < -0.020)
            .count();
        let multi_char_pairs = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.has_multi_char_cluster())
            .count();
        let connected_multi_char_pairs = pairs
            .iter()
            .filter(|pair| {
                pair.is_lower_involved() && pair.has_multi_char_cluster() && pair.min_gap < -0.020
            })
            .count();
        let optical_openings = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.optical_delta > DEAD_ZONE)
            .count();
        let metric_tightenings = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.metric_delta < -DEAD_ZONE)
            .count();
        let max_cluster_chars = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved())
            .map(|pair| pair.max_cluster_chars())
            .max()
            .unwrap_or(1);
        let opened_pairs = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.min_gap < -0.020 && pair.delta > 0.030)
            .count();
        let positive_opening = pairs
            .iter()
            .filter(|pair| pair.is_lower_involved() && pair.min_gap < -0.020)
            .map(|pair| pair.delta.max(0.0))
            .sum::<f32>();
        let mixed_pairs = pairs.iter().filter(|pair| pair.is_mixed_case()).count();
        let lower_pairs = pairs.iter().filter(|pair| pair.is_lower_lower()).count();
        let metricless_lower = pairs
            .iter()
            .filter(|pair| pair.is_lower_lower() && pair.is_metricless())
            .count();
        let connected_lower = pairs
            .iter()
            .filter(|pair| pair.is_lower_lower() && pair.min_gap < -0.020)
            .count();
        let opening_lower = pairs
            .iter()
            .filter(|pair| pair.is_lower_lower() && pair.optical_delta > DEAD_ZONE)
            .count();
        let upper_pairs = pairs.iter().filter(|pair| pair.is_upper_upper()).count();
        let connected_upper = pairs
            .iter()
            .filter(|pair| pair.is_upper_upper() && pair.min_gap < -0.020)
            .count();
        let opened_upper = pairs
            .iter()
            .filter(|pair| pair.is_upper_upper() && pair.min_gap < -0.020 && pair.delta > 0.030)
            .count();

        let ligature = script_profile
            && letter_pairs >= 3
            && multi_char_pairs > 0
            && connected_pairs >= letter_pairs.saturating_sub(1)
            && connected_multi_char_pairs > 0;
        let connected_ratio = if letter_pairs == 0 {
            1.0
        } else {
            connected_pairs as f32 / letter_pairs as f32
        };
        let base_opening = (target_gap * 0.150 * connected_ratio).clamp(0.018, 0.027);
        let ligature_opening =
            if letter_pairs <= 3 && max_cluster_chars >= 3 && optical_openings == 0 {
                base_opening.max((target_gap * 0.240).clamp(0.034, 0.040))
            } else if letter_pairs >= 6 && metric_tightenings >= 2 {
                base_opening.min((target_gap * 0.090).clamp(0.013, 0.018))
            } else if letter_pairs >= 6
                && metric_tightenings == 0
                && optical_openings == 0
                && connected_pairs == letter_pairs
            {
                base_opening.min((target_gap * 0.115).clamp(0.016, 0.020))
            } else {
                base_opening
            };

        Self {
            connected: letter_pairs >= 3
                && connected_pairs >= 2
                && opened_pairs >= 2
                && positive_opening >= 0.080,
            ligature,
            ligature_opening,
            mixed: script_profile && mixed_pairs >= 2,
            severe_metricless_mixed: pairs.iter().any(|pair| {
                pair.is_mixed_case() && pair.metric_delta >= -DEAD_ZONE && pair.delta <= -0.080
            }),
            lower_pairs,
            lower_compaction: script_profile
                && lower_pairs >= 4
                && lower_pairs == letter_pairs
                && metricless_lower == lower_pairs
                && connected_lower >= lower_pairs.saturating_sub(1)
                && opening_lower == 0,
            upper_run: script_profile
                && upper_pairs >= 5
                && connected_upper >= 2
                && opened_upper >= 2
                && pairs
                    .iter()
                    .filter(|pair| pair.is_upper_upper())
                    .all(|pair| pair.is_metricless()),
        }
    }
}

fn adjust_script_run(pair: &mut RunPair, target_gap: f32, run: &ScriptRun) {
    let connected_opening = pair.min_gap < -0.020 && pair.delta > 0.0;
    let capped_connected = run.connected && pair.is_lower_involved() && connected_opening;
    if capped_connected {
        pair.delta = 0.0;
    }

    if run.mixed && pair.is_mixed_case() {
        if connected_opening {
            pair.delta = 0.0;
            return;
        }
        let target = if pair.metric_delta < -DEAD_ZONE {
            if pair.optical_delta > pair.metric_delta {
                pair.metric_delta
            } else {
                (pair.metric_delta - 0.014).max(-0.125)
            }
        } else {
            -(target_gap * 0.24).clamp(0.035, 0.055)
        };
        pair.delta = pair.delta.min(target);
        if run.severe_metricless_mixed && pair.metric_delta >= -DEAD_ZONE && pair.delta > -0.080 {
            let amount = (target_gap * 0.08).clamp(0.010, 0.016);
            let floor = -(target_gap * 0.38).clamp(0.052, 0.070);
            pair.delta = (pair.delta - amount).max(floor);
        }
    }

    if run.mixed
        && run.severe_metricless_mixed
        && run.lower_pairs >= 2
        && pair.is_lower_lower()
        && pair.metric_delta >= -DEAD_ZONE
        && pair.min_gap < -0.020
    {
        pair.delta = pair.delta.min(-(target_gap * 0.085).clamp(0.010, 0.018));
    }

    if run.upper_run && pair.is_upper_upper() && connected_opening {
        pair.delta = pair.delta.min((target_gap * 0.12).clamp(0.018, 0.026));
    }

    if run.lower_compaction
        && pair.is_lower_lower()
        && pair.is_metricless()
        && pair.optical_delta <= DEAD_ZONE
        && pair.min_gap < -0.020
    {
        pair.delta = pair.delta.min(-(target_gap * 0.065).clamp(0.008, 0.012));
    }

    let metric_floor = (target_gap * 0.14).clamp(0.020, 0.028);
    if run.ligature
        && pair.is_lower_involved()
        && pair.min_gap < -0.020
        && pair.metric_delta >= -metric_floor
    {
        pair.delta = pair.delta.max(run.ligature_opening);
    }
}

#[derive(Debug, Clone, Copy)]
struct SerifRun {
    target_gap: f32,
    gap_mad: f32,
    wide_lower: bool,
    ligature_lower: bool,
}

fn adjust_wide_serif_lower_run(pair: &mut RunPair, run: &SerifRun) {
    if run.ligature_lower {
        if !pair.is_lower_lower() || !pair.is_metricless() || pair.right_cluster_chars > 1 {
            return;
        }
        let safe_min = (run.target_gap * 0.34).clamp(0.075, 0.110);
        if pair.min_gap <= safe_min {
            if pair.optical_delta.abs() < DEAD_ZONE {
                pair.delta = pair.delta.max(0.0);
            }
            return;
        }
        pair.delta = pair
            .delta
            .min(-(run.target_gap * 0.098).clamp(0.024, 0.031));
        return;
    }

    let near_touch = (run.target_gap * 0.060).clamp(0.010, 0.018);
    if run.wide_lower
        && pair.is_lower_lower()
        && pair.metric_delta > DEAD_ZONE
        && pair.delta > 0.0
        && (0.0..=near_touch).contains(&pair.min_gap)
    {
        pair.delta = 0.0;
        return;
    }

    if !run.wide_lower
        || !pair.is_lower_lower()
        || !pair.is_metricless()
        || pair.min_gap <= (run.target_gap * 0.22).clamp(0.045, 0.065)
    {
        return;
    }
    pair.delta = pair.delta.min(-(run.gap_mad * 0.25).clamp(0.008, 0.016));
}

#[allow(clippy::too_many_arguments)]
fn adjust_case_run(
    pair: &mut RunPair,
    sans: bool,
    compact_sans: bool,
    upper_pairs: usize,
    strong_upper: usize,
    strong_mixed: usize,
    mixed_pairs: usize,
    lower_pairs: usize,
) {
    if sans && pair.is_upper_upper() && pair.metric_delta < -0.050 && strong_upper >= 2 {
        let amount = match (upper_pairs >= 4, strong_upper) {
            (true, 4..) => 0.036,
            (true, 3) => 0.024,
            (false, 4..) => 0.026,
            _ => 0.012,
        };
        pair.delta = (pair.delta - amount).max(-0.125);
    } else if sans
        && pair.is_mixed_case()
        && pair.metric_delta < -0.050
        && strong_mixed >= 2
        && !compact_sans
    {
        pair.delta = (pair.delta - 0.024).max(-0.130);
    }

    if compact_sans && mixed_pairs >= 2 && lower_pairs >= 3 {
        if pair.is_lower_lower() && pair.delta > -0.018 {
            pair.delta = (pair.delta - 0.010).max(-0.018);
        } else if pair.is_upper_lower() && pair.is_metricless() && pair.delta > -0.074 {
            pair.delta = (pair.delta - 0.010).max(-0.074);
        }
    } else if sans && strong_mixed >= 2 {
        if pair.is_lower_lower() && pair.delta > -0.040 {
            pair.delta = (pair.delta - 0.012).max(-0.040);
        } else if pair.is_upper_lower() && pair.is_metricless() && pair.delta > -0.045 {
            pair.delta = (pair.delta - 0.010).max(-0.045);
        }
    }
}

fn adjust_digit_run(
    pair: &mut RunPair,
    sans: bool,
    target_gap: f32,
    digit_run_pairs: usize,
    metricful_digits: usize,
    loose_digits: usize,
) {
    if !pair.is_digit_run() || !pair.is_metricless() {
        return;
    }

    let sans_digit_run = sans && digit_run_pairs >= 5 && metricful_digits == 0;
    let serif_digit_run = !sans && target_gap >= 0.255 && digit_run_pairs >= 5 && loose_digits >= 2;
    if sans_digit_run {
        let amount = (target_gap * 0.083).clamp(0.014, 0.020);
        let floor = if pair.is_digit_digit() {
            (-target_gap * 0.30).clamp(-0.070, -0.055)
        } else {
            (-target_gap * 0.145).clamp(-0.032, -0.024)
        };
        pair.delta = (pair.delta - amount).max(floor);
    } else if serif_digit_run && pair.is_digit_digit() {
        let amount = (target_gap * 0.060).clamp(0.014, 0.018);
        let floor = (-target_gap * 0.21).clamp(-0.060, -0.050);
        pair.delta = (pair.delta - amount).max(floor);
    }
}

fn adjust_serif_cap_run(
    pair: &mut RunPair,
    sans: bool,
    target_gap: f32,
    upper_pairs: usize,
    strong_upper: usize,
) {
    let serif_cap_run = !sans && target_gap >= 0.225 && upper_pairs >= 4 && strong_upper >= 2;
    if serif_cap_run && pair.is_upper_upper() && pair.metric_delta < -0.050 {
        let extra_width = (target_gap - 0.242).max(0.0);
        let floor = -(0.105 - extra_width * 0.20).clamp(0.098, 0.105);
        pair.delta = pair.delta.max(floor);
    }
}
