use super::*;
use pen_core::{Command, Crop, Element, Kind, Props, WorldPt, WorldRect};
#[test]
fn decoded_cache_has_entry_and_byte_caps_and_releases_removed_images() {
    let mut doc = Document::new();
    let mut commands = Vec::new();
    for n in 0..70 {
        // PNG readers ignore trailing bytes; distinct encoded content exercises the cap.
        let mut bytes =
            include_bytes!("../../../crates/pen-core/tests/fixtures/landscape.png").to_vec();
        bytes.push(n);
        let file = BinaryFile::from_bytes(bytes).unwrap();
        let file_id = file.id();
        let id = doc.alloc_id();
        commands.push(Command::File {
            id: file_id.clone(),
            file: Some(file),
        });
        commands.push(Command::Add {
            element: Element::new(
                id,
                Kind::Image {
                    file_id,
                    crop: Crop::default(),
                    bounds: WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(360.0, 240.0)),
                },
                Props::default(),
            ),
            at: n as usize,
        });
    }
    doc.apply(Command::Group(commands)).unwrap();
    let mut cache = Cache::default();
    cache.update(&doc);
    assert_eq!(cache.entries.len(), MAX_ENTRIES);
    let bytes: u64 = cache
        .entries
        .values()
        .map(|p| {
            let (w, h) = p.size();
            u64::from(w) * u64::from(h) * 4
        })
        .sum();
    assert!(bytes <= MAX_BYTES);
    let commands = doc
        .elements()
        .iter()
        .map(|e| Command::Remove { id: e.id() })
        .collect();
    doc.apply(Command::Group(commands)).unwrap();
    cache.update(&doc);
    assert!(cache.entries.is_empty());
}
