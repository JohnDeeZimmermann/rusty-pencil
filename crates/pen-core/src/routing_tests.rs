use crate::{Path, WorldPt, WorldRect, routing};
fn p(x: f32, y: f32) -> WorldPt {
    WorldPt::new(x, y)
}
#[test]
fn grid_routes_around_endpoint_boxes_including_when_they_face_away() {
    let obstacles = [
        WorldRect::from_corners(p(100.0, 100.0), p(300.0, 200.0)),
        WorldRect::from_corners(p(450.0, 250.0), p(650.0, 450.0)),
    ];
    for (a, b) in [
        (p(300.0, 150.0), p(450.0, 350.0)),
        (p(100.0, 150.0), p(650.0, 350.0)),
        (p(200.0, 100.0), p(550.0, 250.0)),
    ] {
        let points: Vec<_> = routing::elbow(a, b, &obstacles).world_points().collect();
        assert_eq!(points.first(), Some(&a));
        assert_eq!(points.last(), Some(&b));
        for s in points.windows(2) {
            assert!((s[0].x - s[1].x).abs() < 0.001 || (s[0].y - s[1].y).abs() < 0.001);
            for i in 1..100 {
                let t = i as f32 / 100.0;
                let sample = p(
                    s[0].x + (s[1].x - s[0].x) * t,
                    s[0].y + (s[1].y - s[0].y) * t,
                );
                assert!(
                    obstacles.iter().all(|r| !(sample.x > r.min().x
                        && sample.x < r.max().x
                        && sample.y > r.min().y
                        && sample.y < r.max().y)),
                    "crossing at {sample:?}: {points:?}"
                );
            }
        }
    }
}
#[test]
fn catmull_rom_passes_through_vertices_and_keeps_endpoints_and_tangents() {
    let path = Path::from_world(&[p(0.0, 0.0), p(100.0, 80.0), p(200.0, 0.0), p(300.0, 20.0)]);
    let curved = routing::curved(&path);
    let points: Vec<_> = curved.world_points().collect();
    for vertex in path.world_points() {
        assert!(points.contains(&vertex));
    }
    assert!(points.len() > 20);
    assert!(points.iter().all(|p| p.is_finite()));
    let last = points.len() - 1;
    assert!(points[last].x > points[last - 1].x);
    assert_eq!(
        routing::curved(&Path::from_world(&[p(0.0, 0.0)]))
            .points
            .len(),
        1
    );
}
