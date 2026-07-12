use crate::{DEAD_ZONE, GlyphClass, PairEvidence, normalize, preservation::preserve_metric_prior};

pub fn nearest_contour(evidence: PairEvidence) -> f32 {
    evidence.nearest_delta
}

pub fn fallback_only(evidence: PairEvidence) -> f32 {
    if evidence.monospaced || evidence.metric_delta.abs() >= DEAD_ZONE {
        evidence.metric_delta
    } else {
        evidence.optical_delta
    }
}

pub fn compact_guarded(evidence: PairEvidence) -> f32 {
    if evidence.monospaced {
        return evidence.metric_delta;
    }

    let mut delta = metric_prior(evidence);
    delta = protect_local_geometry(evidence, delta);
    delta = apply_shape_targets(evidence, delta);
    preserve_metric_prior(evidence.metric_delta, normalize(delta))
}

fn metric_prior(evidence: PairEvidence) -> f32 {
    let metric = evidence.metric_delta;
    let optical = evidence.optical_delta;
    if metric.abs() < DEAD_ZONE {
        return match (evidence.left, evidence.right) {
            (GlyphClass::Upper, GlyphClass::Digit) => optical.clamp(-0.055, 0.030),
            (GlyphClass::Upper, GlyphClass::Upper) => optical.clamp(-0.070, 0.030),
            (GlyphClass::Digit, GlyphClass::Punctuation)
            | (GlyphClass::Punctuation, GlyphClass::Digit) => optical.clamp(-0.035, 0.035),
            (GlyphClass::Digit, GlyphClass::Digit) => optical.clamp(-0.055, 0.0),
            (GlyphClass::Lower, GlyphClass::Upper) => optical.clamp(-0.060, 0.030),
            _ => optical,
        };
    }

    if (optical - metric).abs() <= 0.045 {
        return metric;
    }
    let preserve = metric < -DEAD_ZONE
        && matches!(
            (evidence.left, evidence.right),
            (
                GlyphClass::Upper,
                GlyphClass::Lower | GlyphClass::Upper | GlyphClass::Punctuation
            )
        );
    metric + if preserve { 0.25 } else { 0.80 } * (optical - metric)
}

fn protect_local_geometry(evidence: PairEvidence, mut delta: f32) -> f32 {
    if evidence.min_gap <= 0.0
        && evidence.nearest_delta > evidence.nearest_guard()
        && !evidence.has_digit()
        && !evidence.has_punctuation()
    {
        let penetration = (-evidence.min_gap).max(0.0);
        return delta.max(
            (evidence.nearest_delta * 0.78 + penetration * 0.22)
                .clamp(evidence.nearest_guard(), 0.055),
        );
    }

    if delta < 0.0
        && (evidence.aperture_risk() || evidence.nearest_delta > evidence.nearest_guard())
        && !matches!(
            (evidence.left, evidence.right),
            (GlyphClass::Digit | GlyphClass::Upper, GlyphClass::Digit)
                | (
                    GlyphClass::Upper,
                    GlyphClass::Lower | GlyphClass::Punctuation
                )
        )
    {
        delta = delta.max(if evidence.metric_delta.abs() >= DEAD_ZONE {
            evidence.metric_delta
        } else {
            0.0
        });
    }

    if evidence.is(GlyphClass::Upper, GlyphClass::Lower)
        && evidence.metric_delta.abs() < DEAD_ZONE
        && evidence.right_side.is_round()
        && evidence.min_gap <= (evidence.target_gap * 0.42).clamp(0.070, 0.120)
        && (evidence.aperture_risk() || evidence.nearest_delta > evidence.nearest_guard())
    {
        delta = delta.max(-(evidence.gap_mad * 1.05).clamp(0.045, 0.065));
    }

    delta
}

fn apply_shape_targets(evidence: PairEvidence, mut delta: f32) -> f32 {
    if evidence.is(GlyphClass::Upper, GlyphClass::Lower)
        && evidence.metric_delta < -DEAD_ZONE
        && evidence.right_side.roundness > 0.040
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.robust_gap > evidence.spread_upper() + 0.012
    {
        delta = delta.min(evidence.metric_delta);
    }

    let wide_serif = evidence.target_gap >= 0.240 && evidence.x_to_cap() < 0.72;
    if evidence.target_gap >= 0.255
        && evidence.x_to_cap() < 0.72
        && evidence.is(GlyphClass::Upper, GlyphClass::Upper)
        && evidence.metric_delta <= DEAD_ZONE
        && (evidence.left_side.is_diagonal() || evidence.right_side.is_diagonal())
        && evidence.min_gap > -0.020
        && evidence.robust_gap > evidence.spread_upper()
    {
        delta = delta.min(0.0);
    }

    if evidence.is(GlyphClass::Lower, GlyphClass::Upper)
        && evidence.metric_delta.abs() < DEAD_ZONE
        && evidence.optical_delta < -DEAD_ZONE
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.58).clamp(0.10, 0.18)
        && !evidence.aperture_risk()
        && evidence.right_top_left_overhang > 0.10
    {
        let overhang = evidence.right_top_left_overhang;
        let shape = ((overhang - 0.10) * 0.24).clamp(0.0, 0.040);
        let excess =
            ((evidence.robust_gap - evidence.spread_upper()).max(0.0) * 0.40).clamp(0.0, 0.030);
        let round = if evidence.left_side.is_round() && overhang > 0.18 {
            (((evidence.left_side.roundness - 0.030) * 0.70).clamp(0.0, 0.024)
                + ((overhang - 0.18) * 0.16).clamp(0.0, 0.020))
            .clamp(0.0, 0.034)
        } else {
            0.0
        };
        let serif_excess = (evidence.robust_gap - evidence.spread_upper()).max(0.0);
        let serif = if evidence.target_gap >= 0.220
            && evidence.x_to_cap() < 0.72
            && serif_excess > 0.030
            && evidence.left_side.is_round()
            && overhang > 0.18
        {
            (serif_excess * 0.30).clamp(0.0, 0.020)
        } else {
            0.0
        };
        let floor = if serif > 0.0 {
            -0.140
        } else if evidence.left_side.is_round() && overhang > 0.18 {
            -0.120
        } else {
            -0.095
        };
        delta = delta.min((delta - shape - excess - round - serif).max(floor));
    }

    if evidence.is(GlyphClass::Upper, GlyphClass::Punctuation)
        && evidence.metric_delta < -DEAD_ZONE
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.65).clamp(0.12, 0.18)
    {
        let amount = (evidence.gap_mad * 0.46).clamp(0.018, 0.035);
        delta = delta.min((evidence.metric_delta.min(delta) - amount).max(-0.120));
    }

    if evidence.is(GlyphClass::Digit, GlyphClass::Digit)
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.24).clamp(0.045, 0.075)
    {
        let target = if (evidence.left_side.is_stem() && evidence.right_side.is_round())
            || (evidence.left_side.is_round() && evidence.right_side.is_stem())
        {
            -0.040
        } else if evidence.left_side.is_round() && evidence.right_side.is_round() {
            -0.024
        } else {
            0.0
        };
        delta = delta.min(target);
    }

    if wide_serif
        && matches!(
            (evidence.left, evidence.right),
            (GlyphClass::Upper, GlyphClass::Lower) | (GlyphClass::Lower, GlyphClass::Upper)
        )
        && evidence.metric_delta <= -DEAD_ZONE
        && delta >= -0.135
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.48).clamp(0.11, 0.16)
        && !evidence.aperture_risk()
        && (evidence.left_side.is_round()
            || evidence.right_side.is_round()
            || evidence.right_top_left_overhang > 0.10)
    {
        let gap_bonus =
            ((evidence.robust_gap - evidence.spread_upper()).max(0.0) * 0.16).clamp(0.0, 0.014);
        delta = delta.min((delta.min(evidence.metric_delta) - 0.018 - gap_bonus).max(-0.140));
    }

    if wide_serif
        && evidence.is(GlyphClass::Upper, GlyphClass::Upper)
        && (evidence.left_side.is_diagonal() || evidence.right_side.is_diagonal())
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.48).clamp(0.11, 0.16)
        && !evidence.aperture_risk()
        && evidence.metric_delta >= -0.105
    {
        delta = delta.min((delta.min(evidence.metric_delta.min(0.0)) - 0.022).max(-0.125));
    }

    let sans = evidence.target_gap <= 0.235 && evidence.x_to_cap() >= 0.72;
    if sans
        && matches!(
            (evidence.left, evidence.right),
            (GlyphClass::Lower | GlyphClass::Upper, GlyphClass::Lower)
        )
        && evidence.nearest_delta <= evidence.nearest_guard()
        && evidence.min_gap > (evidence.target_gap * 0.36).clamp(0.070, 0.100)
        && !evidence.aperture_risk()
        && !(evidence.is(GlyphClass::Lower, GlyphClass::Lower)
            && evidence.metric_delta.abs() >= 0.025)
    {
        let amount = if evidence.target_gap <= 0.210 {
            0.008
        } else {
            0.018
        };
        delta = (delta - amount).max(-0.105);
    }

    let is_plain_letter_pair = !evidence.has_digit() && !evidence.has_punctuation();
    let preserves_metricless_tightening =
        delta < -DEAD_ZONE && evidence.metric_delta.abs() < DEAD_ZONE;
    if is_plain_letter_pair
        && delta.abs() < 0.045
        && !preserves_metricless_tightening
        && evidence.nearest_delta <= evidence.nearest_guard()
        && !evidence.aperture_risk()
        && evidence.min_gap > (evidence.target_gap * 0.22).clamp(0.045, 0.065)
    {
        delta -= (evidence.gap_mad * 0.25).clamp(0.008, 0.016);
    }

    delta
}
