use super::*;

#[test]
fn delta_plan_lets_safety_cap_win_when_bounds_conflict() {
    let mut plan = DeltaPlan::new(-0.020);
    plan.require_at_least(0.040);
    plan.limit_to_at_most(0.0);

    assert_eq!(plan.finish(), 0.0);
}

#[test]
fn delta_plan_keeps_sequential_tightening_additive() {
    let mut plan = DeltaPlan::new(-0.019_531_25);
    plan.tighten_to(plan.desired_delta() - 0.018);
    plan.tighten_to(plan.desired_delta() - 0.012_196_51);

    assert!((plan.finish() + 0.049_727_76).abs() < 0.000_01);
}
