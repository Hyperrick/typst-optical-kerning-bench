use super::*;

#[test]
fn detects_top_left_overhang() {
    let outline = glyph_from_rects(&[(0.00, 0.60, 0.62, 0.72), (0.26, 0.00, 0.36, 0.60)]);
    let config = ProfileConfig::default();
    assert!(top_left_overhang(&outline, config) > 0.20);
}

#[test]
fn ignores_plain_left_edge_as_overhang() {
    let outline = glyph_from_rects(&[(0.00, 0.00, 0.42, 0.72)]);
    let config = ProfileConfig::default();
    assert_eq!(top_left_overhang(&outline, config), 0.0);
}

#[test]
fn detects_round_side_features() {
    let outline = glyph_from_polygon(&[
        (0.22, 0.00),
        (0.04, 0.18),
        (0.00, 0.36),
        (0.04, 0.54),
        (0.22, 0.72),
        (0.48, 0.72),
        (0.66, 0.54),
        (0.70, 0.36),
        (0.66, 0.18),
        (0.48, 0.00),
    ]);
    let left = SideFeatures::from_outline(&outline, Side::Left);
    let right = SideFeatures::from_outline(&outline, Side::Right);

    assert!(left.roundness > 0.10);
    assert!(right.roundness > 0.10);
    assert!(left.stemness < 0.45);
    assert!(right.stemness < 0.45);
}

#[test]
fn detects_stem_side_features() {
    let outline = glyph_from_rects(&[(0.20, 0.00, 0.32, 0.72)]);
    let left = SideFeatures::from_outline(&outline, Side::Left);
    let right = SideFeatures::from_outline(&outline, Side::Right);

    assert!(left.stemness > 0.90);
    assert!(right.stemness > 0.90);
    assert!(left.roundness < 0.01);
    assert!(right.roundness < 0.01);
}
