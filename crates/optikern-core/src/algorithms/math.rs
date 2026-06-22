pub(super) fn normalized_delta(value: f32) -> f32 {
    let clamped = clamp_delta(value);
    if clamped.abs() < dead_zone() {
        0.0
    } else {
        clamped
    }
}

pub(super) fn dead_zone() -> f32 {
    0.006
}

pub(super) fn clamp_delta(value: f32) -> f32 {
    value.clamp(-0.16, 0.16)
}

pub(super) fn percentile(values: &[f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = ((values.len() - 1) as f32 * p.clamp(0.0, 1.0)).round() as usize;
    values[idx]
}
