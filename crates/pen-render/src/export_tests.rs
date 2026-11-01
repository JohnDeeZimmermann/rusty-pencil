use crate::{DrawList, Prim, export::*};
use pen_core::{Srgba, WorldPt};
use std::collections::BTreeMap;
fn fixture() -> DrawList {
    let mut list = DrawList::new();
    list.push(Prim::Fill {
        points: vec![
            WorldPt::new(0.0, 0.0),
            WorldPt::new(40.0, 0.0),
            WorldPt::new(40.0, 20.0),
            WorldPt::new(0.0, 20.0),
        ],
        color: Srgba::rgb(0x235e4d),
    });
    list
}
#[test]
fn svg_fixture_matches_golden() {
    let output = svg(&fixture(), &BTreeMap::new(), Options::default()).unwrap();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/export.svg"),
            &output,
        )
        .unwrap();
    }
    assert_eq!(output, include_str!("../tests/fixtures/export.svg"));
}
#[test]
fn png_scales_alpha_and_dark_mode_change_real_pixels() {
    for scale in 1..=3 {
        let bytes = png(
            &fixture(),
            &BTreeMap::new(),
            Options {
                scale,
                background: None,
                dark: false,
            },
        )
        .unwrap();
        let decoded = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (72 * u32::from(scale), 52 * u32::from(scale))
        );
        assert_eq!(decoded.pixel(0, 0).unwrap().alpha(), 0);
        let center = decoded
            .pixel(36 * u32::from(scale), 26 * u32::from(scale))
            .unwrap();
        assert_eq!(
            (center.red(), center.green(), center.blue(), center.alpha()),
            (0x23, 0x5e, 0x4d, 255)
        );
    }
    let dark = png(
        &fixture(),
        &BTreeMap::new(),
        Options {
            dark: true,
            ..Options::default()
        },
    )
    .unwrap();
    let light = png(&fixture(), &BTreeMap::new(), Options::default()).unwrap();
    assert_ne!(dark, light);
    assert!(matches!(
        png(&DrawList::new(), &BTreeMap::new(), Options::default()),
        Err(ExportError::Empty)
    ));
}
#[test]
fn svg_keeps_embedded_images_cropped_and_text_as_portable_paths() {
    let scene = pen_core::file::load(include_bytes!(
        "../../pen-core/tests/fixtures/all-elements.excalidraw"
    ))
    .unwrap();
    let mut list = DrawList::new();
    for e in &scene.elements {
        crate::draw_element(e, &mut list);
    }
    let output = svg(&list, &scene.files, Options::default()).unwrap();
    assert!(output.contains("data:image/png;base64,"));
    assert!(!output.contains("<text"));
    assert!(output.contains("<path"));
    assert!(png(&list, &scene.files, Options::default()).unwrap().len() > 1000);
}

#[test]
fn cropped_image_png_keeps_source_pixels_and_ignores_dark_recoloring() {
    let file = pen_core::BinaryFile::from_bytes(
        include_bytes!("../../pen-core/tests/fixtures/landscape.png").to_vec(),
    )
    .unwrap();
    let id = file.id();
    let files = BTreeMap::from([(id.clone(), file)]);
    let mut list = DrawList::new();
    list.push(Prim::Image {
        file_id: id,
        crop: pen_core::Crop {
            x: 0.5,
            width: 0.5,
            ..Default::default()
        },
        bounds: pen_core::WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(180.0, 240.0)),
        angle: 0.0,
        opacity: 100,
    });
    let bytes = png(
        &list,
        &files,
        Options {
            dark: true,
            background: None,
            ..Default::default()
        },
    )
    .unwrap();
    let decoded = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (212, 272));
    let sun = decoded.pixel(114, 78).unwrap();
    assert_eq!(
        (sun.red(), sun.green(), sun.blue(), sun.alpha()),
        (245, 208, 128, 255)
    );
    assert_eq!(decoded.pixel(0, 0).unwrap().alpha(), 0);
}
