use super::*;

#[test]
fn origin_is_zero() {
    assert_eq!(WorldPt::ORIGIN, WorldPt::new(0.0, 0.0));
    assert_eq!(WorldPt::default(), WorldPt::ORIGIN);
}
