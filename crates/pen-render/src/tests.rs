#[test]
fn links_against_core_and_editor() {
    let camera = pen_editor::Camera::default();
    assert_eq!(camera.scroll(), pen_core::WorldPt::ORIGIN);
}
