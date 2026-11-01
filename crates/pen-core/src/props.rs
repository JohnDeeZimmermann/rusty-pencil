use crate::{Edges, Roughness, Srgba};
use serde::{Deserialize, Serialize};

/// How an element is painted. Defaults match Excalidraw's.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(default)]
pub struct Props {
    pub stroke: Srgba,
    /// `None` is a transparent background.
    pub background: Option<Srgba>,
    /// In world points, so it scales with zoom.
    pub stroke_width: f32,
    pub edges: Edges,
    pub fill: FillStyle,
    pub stroke_style: StrokeStyle,
    /// Straight alpha multiplier, 0–100 percent.
    pub opacity: u8,
    pub arrow_type: ArrowType,
    pub vertical_align: VerticalAlign,
    pub start_head: Arrowhead,
    pub end_head: Arrowhead,
    #[serde(default)]
    pub roughness: Roughness,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            stroke: Srgba::INK,
            background: None,
            stroke_width: 2.0,
            edges: Edges::Round,
            fill: FillStyle::Solid,
            stroke_style: StrokeStyle::Solid,
            opacity: 100,
            arrow_type: ArrowType::Sharp,
            vertical_align: VerticalAlign::Middle,
            start_head: Arrowhead::None,
            end_head: Arrowhead::Arrow,
            roughness: Roughness::Artist,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillStyle {
    Hachure,
    CrossHatch,
    #[default]
    Solid,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arrowhead {
    #[default]
    None,
    Arrow,
    Bar,
    Dot,
    Triangle,
    TriangleOutline,
    Circle,
    CircleOutline,
    Diamond,
    DiamondOutline,
}

/// A field edit preserves every other field, including mixed values in a selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Property {
    Stroke(Srgba),
    Background(Option<Srgba>),
    Fill(FillStyle),
    Width(f32),
    StrokeStyle(StrokeStyle),
    Edges(Edges),
    Opacity(u8),
    ArrowType(ArrowType),
    VerticalAlign(VerticalAlign),
    StartHead(Arrowhead),
    EndHead(Arrowhead),
}
impl Property {
    pub fn apply(self, props: &mut Props) {
        match self {
            Self::Stroke(v) => props.stroke = v,
            Self::Background(v) => props.background = v,
            Self::Fill(v) => props.fill = v,
            Self::Width(v) if v.is_finite() && (0.0..=1000.0).contains(&v) => {
                props.stroke_width = v;
            }
            Self::Width(_) => {}
            Self::StrokeStyle(v) => props.stroke_style = v,
            Self::Edges(v) => props.edges = v,
            Self::Opacity(v) => props.opacity = v.min(100),
            Self::ArrowType(v) => props.arrow_type = v,
            Self::VerticalAlign(v) => props.vertical_align = v,
            Self::StartHead(v) => props.start_head = v,
            Self::EndHead(v) => props.end_head = v,
        }
    }
}

/// Shared inspector value; `Mixed` never substitutes a default for selected data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<T> {
    Uniform(T),
    Mixed,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowType {
    #[default]
    Sharp,
    Curved,
    Elbow,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}
