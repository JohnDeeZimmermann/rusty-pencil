//! Excalidraw JSON import. Unsupported and deleted elements do not prevent recovery.
use crate::{
    file::{AppState, FileError, SceneFile},
    *,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::collections::BTreeMap;

fn number(v: &Value, key: &str, default: f32) -> f32 {
    v[key].as_f64().map_or(default, |n| n as f32)
}
fn color(v: &Value) -> Option<Srgba> {
    let s = v.as_str()?.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    u32::from_str_radix(s, 16).ok().map(Srgba::rgb)
}

pub fn excalidraw(value: &Value) -> Result<SceneFile, FileError> {
    let mut doc = Document::new();
    let mut files = BTreeMap::new();
    if let Some(source) = value["files"].as_object() {
        for (key, v) in source {
            if let Some(url) = v["dataURL"].as_str()
                && let Some((header, data)) = url.split_once(",")
            {
                let mime = header
                    .strip_prefix("data:")
                    .and_then(|s| s.strip_suffix(";base64"));
                if let (Some(mime), Ok(data)) = (mime, STANDARD.decode(data))
                    && let Some(file) = BinaryFile::from_bytes(data).filter(|f| f.mime == mime)
                {
                    let id = file.id();
                    files.insert(key.clone(), id.clone());
                    doc.apply(Command::File {
                        id,
                        file: Some(file),
                    })
                    .map_err(|e| FileError::Invalid(e.to_string()))?;
                }
            }
        }
    }
    let elements = value["elements"]
        .as_array()
        .ok_or_else(|| FileError::Invalid("missing elements".into()))?;
    let mut skipped = 0;
    let mut sources = BTreeMap::new();
    let mut groups = BTreeMap::new();
    for v in elements {
        if v["isDeleted"].as_bool() == Some(true) {
            continue;
        }
        let p = WorldPt::new(number(v, "x", 0.0), number(v, "y", 0.0));
        let bounds = WorldRect::from_corners(
            p,
            WorldPt::new(
                p.x + number(v, "width", 1.0),
                p.y + number(v, "height", 1.0),
            ),
        );
        let mut props = Props {
            stroke: color(&v["strokeColor"]).unwrap_or(Props::default().stroke),
            background: color(&v["backgroundColor"]),
            stroke_width: number(v, "strokeWidth", 2.0),
            opacity: number(v, "opacity", 100.0).clamp(0.0, 100.0) as u8,
            ..Props::default()
        };
        props.fill = match v["fillStyle"].as_str() {
            Some("solid") => FillStyle::Solid,
            Some("cross-hatch") => FillStyle::CrossHatch,
            _ => FillStyle::Hachure,
        };
        props.stroke_style = match v["strokeStyle"].as_str() {
            Some("dashed") => StrokeStyle::Dashed,
            Some("dotted") => StrokeStyle::Dotted,
            _ => StrokeStyle::Solid,
        };
        props.start_head = head(v["startArrowhead"].as_str());
        props.end_head = if v.get("endArrowhead").is_some() {
            head(v["endArrowhead"].as_str())
        } else if v["type"].as_str() == Some("arrow") {
            Arrowhead::Arrow
        } else {
            Arrowhead::None
        };
        props.arrow_type = if v["elbowed"].as_bool() == Some(true) {
            ArrowType::Elbow
        } else {
            ArrowType::Sharp
        };
        props.vertical_align = match v["verticalAlign"].as_str() {
            Some("top") => VerticalAlign::Top,
            Some("bottom") => VerticalAlign::Bottom,
            _ => VerticalAlign::Middle,
        };
        props.edges = if v["roundness"].is_object() {
            Edges::Round
        } else {
            Edges::Sharp
        };
        props.roughness = match number(v, "roughness", 1.0) as u8 {
            0 => Roughness::Architect,
            2.. => Roughness::Cartoonist,
            _ => Roughness::Artist,
        };
        let kind = match v["type"].as_str() {
            Some("rectangle") => Kind::Rect(bounds),
            Some("ellipse") => Kind::Ellipse(bounds),
            Some("diamond") => Kind::Diamond(bounds),
            Some("text") => Kind::Text {
                content: v["text"].as_str().unwrap_or("").into(),
                font: match v["fontFamily"].as_u64() {
                    Some(2) => Font::Normal,
                    Some(3) => Font::Code,
                    _ => Font::HandDrawn,
                },
                size: number(v, "fontSize", 20.0),
                align: match v["textAlign"].as_str() {
                    Some("center") => TextAlign::Centre,
                    Some("right") => TextAlign::Right,
                    _ => TextAlign::Left,
                },
                bounds,
                container: None,
            },
            Some(t @ ("line" | "arrow" | "freedraw")) => {
                let points: Vec<_> = v["points"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|a| {
                        WorldPt::new(
                            p.x + a[0].as_f64().unwrap_or(0.0) as f32,
                            p.y + a[1].as_f64().unwrap_or(0.0) as f32,
                        )
                    })
                    .collect();
                let mut path = Path::from_world(&points);
                if t == "freedraw"
                    && let Some(pressures) = v["pressures"].as_array()
                {
                    for (p, value) in path.points.iter_mut().zip(pressures) {
                        p.pressure = value.as_f64().unwrap_or(0.5).clamp(0.0, 1.0) as f32;
                    }
                }
                match t {
                    "arrow" => Kind::Arrow(path),
                    "freedraw" => Kind::Freedraw(path),
                    _ => Kind::Line(path),
                }
            }
            Some("image") => {
                let Some(id) = v["fileId"].as_str().and_then(|id| files.get(id)) else {
                    skipped += 1;
                    continue;
                };
                Kind::Image {
                    file_id: id.clone(),
                    crop: if v["crop"].is_object() {
                        let c = &v["crop"];
                        Crop {
                            x: number(c, "x", 0.0) / number(c, "naturalWidth", 1.0),
                            y: number(c, "y", 0.0) / number(c, "naturalHeight", 1.0),
                            width: number(c, "width", 1.0) / number(c, "naturalWidth", 1.0),
                            height: number(c, "height", 1.0) / number(c, "naturalHeight", 1.0),
                        }
                    } else {
                        Crop::default()
                    },
                    bounds,
                }
            }
            _ => {
                skipped += 1;
                continue;
            }
        };
        let id = doc.alloc_id();
        let mut element = Element::new(id, kind, props);
        element.angle = number(v, "angle", 0.0);
        if let Some(seed) = v["seed"].as_u64() {
            element.set_seed(seed as u32);
        }
        for g in v["groupIds"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let next = GroupId(groups.len() as u64 + 1);
            element.group_ids.push(*groups.entry(g).or_insert(next));
        }
        if let Some(source_id) = v["id"].as_str() {
            sources.insert(source_id, id);
        }
        element.locked = v["locked"].as_bool().unwrap_or(false);
        doc.apply(Command::Add {
            element,
            at: doc.elements().len(),
        })
        .map_err(|e| FileError::Invalid(e.to_string()))?;
    }
    let mut state = AppState::default();
    state.settings.canvas =
        color(&value["appState"]["viewBackgroundColor"]).unwrap_or(state.settings.canvas);
    let mut scene = SceneFile::new(&doc, state);
    scene.skipped_elements = skipped;
    for v in elements {
        let Some(id) = v["id"].as_str().and_then(|id| sources.get(id)) else {
            continue;
        };
        let Some(out) = scene.elements.iter_mut().find(|e| e.id() == *id) else {
            continue;
        };
        if let Kind::Text { container, .. } = &mut out.kind {
            *container = v["containerId"]
                .as_str()
                .and_then(|id| sources.get(id))
                .and_then(|id| doc.get(*id))
                .and_then(|target| {
                    if binding::is_shape(target) {
                        Some(Container::Shape(target.id()))
                    } else if target.kind.is_linear() {
                        Some(Container::Path(target.id()))
                    } else {
                        None
                    }
                });
        }
        if let Kind::Arrow(path) = &out.kind {
            for (end, key) in ["startBinding", "endBinding"].iter().enumerate() {
                let Some(shape) = v[key]["elementId"]
                    .as_str()
                    .and_then(|id| sources.get(id))
                    .and_then(|id| doc.get(*id))
                    .filter(|e| binding::is_shape(e))
                else {
                    continue;
                };
                let index = if end == 0 {
                    0
                } else {
                    path.points.len().saturating_sub(1)
                };
                if let Some(p) = path.world_point(index) {
                    let p = rotate(p, shape.kind.bounds().centre(), -shape.angle);
                    let c = shape.kind.bounds().centre();
                    out.bindings[end] = Some(binding::Binding {
                        element: shape.id(),
                        focus: (p.y - c.y).atan2(p.x - c.x) / std::f32::consts::PI,
                        gap: number(&v[key], "gap", 8.0).clamp(0.0, 1000.0),
                    });
                }
            }
        }
    }
    scene.document()?;
    Ok(scene)
}

fn head(value: Option<&str>) -> Arrowhead {
    match value {
        Some("arrow") => Arrowhead::Arrow,
        Some("bar") => Arrowhead::Bar,
        Some("dot") => Arrowhead::Dot,
        Some("triangle") => Arrowhead::Triangle,
        Some("triangle_outline") => Arrowhead::TriangleOutline,
        Some("circle") => Arrowhead::Circle,
        Some("circle_outline") => Arrowhead::CircleOutline,
        Some("diamond") => Arrowhead::Diamond,
        Some("diamond_outline") => Arrowhead::DiamondOutline,
        _ => Arrowhead::None,
    }
}
