//! Standalone SVG from world-space paint, and CPU PNG from that same SVG.
use crate::{DrawList, Prim};
use base64::{Engine, engine::general_purpose::STANDARD};
use pen_core::{BinaryFile, FileId, Srgba, WorldPt, WorldRect};
use std::{collections::BTreeMap, fmt::Write};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub scale: u8,
    pub background: Option<Srgba>,
    pub dark: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            scale: 1,
            background: Some(Srgba::rgb(0xf8f9f7)),
            dark: false,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Nothing to export")]
    Empty,
    #[error("Export exceeds 64 megapixels, or scale is not 1, 2 or 3")]
    Size,
    #[error("Invalid SVG: {0}")]
    Svg(String),
    #[error("PNG encoding failed: {0}")]
    Png(String),
    #[error("Missing embedded image")]
    Image,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn ink(c: Srgba, dark: bool) -> String {
    let c = if dark { c.dark() } else { c };
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}
fn bounds(prim: &Prim) -> Option<WorldRect> {
    match prim {
        Prim::Clip { bounds, .. } => Some(*bounds),
        Prim::Image { bounds, angle, .. } | Prim::Text { bounds, angle, .. } => [
            bounds.min(),
            WorldPt::new(bounds.max().x, bounds.min().y),
            bounds.max(),
            WorldPt::new(bounds.min().x, bounds.max().y),
        ]
        .into_iter()
        .map(|p| pen_core::rotate(p, bounds.centre(), *angle))
        .map(|p| WorldRect::from_corners(p, p))
        .reduce(WorldRect::union),
        Prim::Fill { points, .. } | Prim::Stroke { points, .. } => {
            let b = points
                .iter()
                .map(|p| WorldRect::from_corners(*p, *p))
                .reduce(WorldRect::union)?;
            let pad = if let Prim::Stroke { width, .. } = prim {
                width / 2.0
            } else {
                0.0
            };
            Some(WorldRect::from_centre(
                b.centre(),
                b.width() / 2.0 + pad,
                b.height() / 2.0 + pad,
            ))
        }
    }
}

fn svg_source(
    list: &DrawList,
    files: &BTreeMap<FileId, BinaryFile>,
    options: Options,
) -> Result<String, ExportError> {
    let b = list
        .prims()
        .iter()
        .filter_map(bounds)
        .reduce(WorldRect::union)
        .ok_or(ExportError::Empty)?;
    let padding = if matches!(list.prims(), [Prim::Clip { .. }]) {
        0.0
    } else {
        16.0
    };
    let x = b.min().x - padding;
    let y = b.min().y - padding;
    let w = (b.width() + padding * 2.0).ceil();
    let h = (b.height() + padding * 2.0).ceil();
    let scale = f32::from(options.scale);
    if !(1..=3).contains(&options.scale)
        || !w.is_finite()
        || !h.is_finite()
        || w * h * scale * scale > 64.0 * 1024.0 * 1024.0
    {
        return Err(ExportError::Size);
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{x} {y} {w} {h}\">\n",
        w * scale,
        h * scale
    );
    if let Some(c) = options.background {
        let _ = writeln!(
            out,
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" fill-opacity=\"{}\"/>",
            ink(c, options.dark),
            f32::from(c.a) / 255.0
        );
    }
    for (i, prim) in list.prims().iter().enumerate() {
        match prim {
            Prim::Clip { bounds: b, prims } => {
                let inner = DrawList {
                    prims: prims.clone(),
                };
                if !inner.is_empty() {
                    let source = svg_source(
                        &inner,
                        files,
                        Options {
                            background: None,
                            ..options
                        },
                    )?;
                    let body = source
                        .split_once('>')
                        .map_or("", |(_, body)| body)
                        .trim_end_matches("</svg>\n");
                    let _ = writeln!(
                        out,
                        "<defs><clipPath id=\"frame-{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#frame-{i})\">{body}</g>",
                        b.min().x,
                        b.min().y,
                        b.width(),
                        b.height()
                    );
                }
            }
            Prim::Fill { points, color } | Prim::Stroke { points, color, .. } => {
                let points = points
                    .iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let color_hex = ink(*color, options.dark);
                let alpha = f32::from(color.a) / 255.0;
                if let Prim::Stroke { width, closed, .. } = prim {
                    let tag = if *closed { "polygon" } else { "polyline" };
                    let _ = writeln!(
                        out,
                        "<{tag} points=\"{points}\" fill=\"none\" stroke=\"{color_hex}\" stroke-opacity=\"{alpha}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "<polygon points=\"{points}\" fill=\"{color_hex}\" fill-opacity=\"{alpha}\"/>"
                    );
                }
            }
            Prim::Image {
                file_id,
                crop,
                bounds: b,
                angle,
                opacity,
            } => {
                let file = files.get(file_id).ok_or(ExportError::Image)?;
                let c = b.centre();
                let _ = writeln!(
                    out,
                    "<g transform=\"rotate({} {} {})\" opacity=\"{}\"><svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" preserveAspectRatio=\"none\" overflow=\"hidden\"><image width=\"{}\" height=\"{}\" href=\"data:{};base64,{}\"/></svg></g>",
                    angle.to_degrees(),
                    c.x,
                    c.y,
                    f32::from(*opacity) / 100.0,
                    b.min().x,
                    b.min().y,
                    b.width(),
                    b.height(),
                    crop.x * file.width as f32,
                    crop.y * file.height as f32,
                    crop.width * file.width as f32,
                    crop.height * file.height as f32,
                    file.width,
                    file.height,
                    file.mime,
                    STANDARD.encode(&file.data)
                );
            }
            Prim::Text {
                content,
                font,
                size,
                bounds: b,
                color,
                angle,
                align,
                fit,
                vertical_align,
            } => {
                let family = match font {
                    pen_core::Font::HandDrawn => "Patrick Hand",
                    pen_core::Font::Normal => "Inter",
                    pen_core::Font::Code => "Roboto Mono",
                };
                let (anchor, x) = match align {
                    pen_core::TextAlign::Left => ("start", b.min().x),
                    pen_core::TextAlign::Centre => ("middle", b.centre().x),
                    pen_core::TextAlign::Right => ("end", b.max().x),
                };
                let c = b.centre();
                let _ = writeln!(
                    out,
                    "<g transform=\"rotate({} {} {})\">",
                    angle.to_degrees(),
                    c.x,
                    c.y
                );
                if *fit == crate::TextFit::Label
                    && let Some(bg) = options.background
                {
                    let _ = writeln!(
                        out,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\"/>",
                        b.min().x - 4.0,
                        b.min().y - 4.0,
                        b.width() + 8.0,
                        b.height() + 8.0,
                        ink(bg, options.dark)
                    );
                }
                let lines = wrap(
                    content,
                    *font,
                    *size,
                    b.width(),
                    *fit == crate::TextFit::Wrap,
                );
                let height = lines.len() as f32 * size * 1.25;
                let offset = if *fit == crate::TextFit::Wrap {
                    match vertical_align {
                        pen_core::VerticalAlign::Top => 0.0,
                        pen_core::VerticalAlign::Middle => (b.height() - height) / 2.0,
                        pen_core::VerticalAlign::Bottom => b.height() - height,
                    }
                } else {
                    0.0
                };
                let _ = writeln!(
                    out,
                    "<text id=\"text-{i}\" font-family=\"{family}\" font-size=\"{size}\" text-anchor=\"{anchor}\" fill=\"{}\" fill-opacity=\"{}\" xml:space=\"preserve\">",
                    ink(*color, options.dark),
                    f32::from(color.a) / 255.0
                );
                for (line, s) in lines.iter().enumerate() {
                    let _ = writeln!(
                        out,
                        "<tspan x=\"{x}\" y=\"{}\">{}</tspan>",
                        b.min().y + offset + size + line as f32 * size * 1.25,
                        esc(s)
                    );
                }
                out.push_str("</text></g>\n");
            }
        }
    }
    out.push_str("</svg>\n");
    Ok(out)
}
fn font_bytes(font: pen_core::Font) -> &'static [u8] {
    match font {
        pen_core::Font::Normal => include_bytes!("../../../assets/fonts/Inter.ttf"),
        pen_core::Font::HandDrawn => include_bytes!("../../../assets/fonts/PatrickHand.ttf"),
        pen_core::Font::Code => include_bytes!("../../../assets/fonts/RobotoMono.ttf"),
    }
}
fn wrap(content: &str, font: pen_core::Font, size: f32, width: f32, enabled: bool) -> Vec<String> {
    let face = rustybuzz::Face::from_slice(font_bytes(font), 0);
    let measure = |s: &str| {
        let Some(face) = &face else {
            return 0.0;
        };
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(s);
        buffer.guess_segment_properties();
        let glyphs = rustybuzz::shape(face, &[], buffer);
        glyphs
            .glyph_positions()
            .iter()
            .map(|p| p.x_advance as f32)
            .sum::<f32>()
            * size
            / face.units_per_em() as f32
    };
    let mut lines = Vec::new();
    for line in content.split('\n') {
        if !enabled {
            lines.push(line.into());
            continue;
        }
        let mut current = String::new();
        for word in line.split_inclusive(' ') {
            if !current.is_empty() && measure(format!("{current}{word}").trim_end()) > width {
                lines.push(std::mem::take(&mut current));
            }
            for c in word.chars() {
                let mut next = current.clone();
                next.push(c);
                if !current.is_empty() && !c.is_whitespace() && measure(&next) > width {
                    lines.push(std::mem::take(&mut current));
                }
                current.push(c);
            }
        }
        lines.push(current);
    }
    lines
}
fn tree(source: &str) -> Result<resvg::usvg::Tree, ExportError> {
    let mut opt = resvg::usvg::Options::default();
    for font in [
        pen_core::Font::Normal,
        pen_core::Font::HandDrawn,
        pen_core::Font::Code,
    ] {
        opt.fontdb_mut().load_font_data(font_bytes(font).to_vec());
    }
    resvg::usvg::Tree::from_str(source, &opt).map_err(|e| ExportError::Svg(e.to_string()))
}
/// Text is converted to paths using bundled fonts, so SVG appearance is portable.
pub fn svg(
    list: &DrawList,
    files: &BTreeMap<FileId, BinaryFile>,
    options: Options,
) -> Result<String, ExportError> {
    let source = svg_source(list, files, options)?;
    Ok(tree(&source)?.to_string(&resvg::usvg::WriteOptions::default()))
}

pub fn png(
    list: &DrawList,
    files: &BTreeMap<FileId, BinaryFile>,
    options: Options,
) -> Result<Vec<u8>, ExportError> {
    let source = svg_source(list, files, options)?;
    let tree = tree(&source)?;
    let size = tree.size().to_int_size();
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or(ExportError::Size)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| ExportError::Png(e.to_string()))
}
