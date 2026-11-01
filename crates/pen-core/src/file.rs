//! Versioned, validated scene interchange. Version zero migrates by supplying defaults.
use crate::{Command, Document, Element, Kind, Props, WorldPt};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub settings: crate::Settings,
    pub scroll: WorldPt,
    pub zoom: f32,
    #[serde(default)]
    pub style: Props,
    #[serde(default)]
    pub font: crate::Font,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default)]
    pub text_align: crate::TextAlign,
}

fn default_font_size() -> f32 {
    20.0
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: crate::Settings::default(),
            scroll: WorldPt::ORIGIN,
            zoom: 1.0,
            style: Props::default(),
            font: crate::Font::default(),
            font_size: 20.0,
            text_align: crate::TextAlign::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneFile {
    #[serde(skip)]
    pub dropped_bindings: usize,
    pub format: String,
    pub version: u32,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub app_state: AppState,
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("This scene is truncated")]
    Truncated,
    #[error("Invalid scene JSON: {0}")]
    Json(serde_json::Error),
    #[error("This is not a drusty-pen scene")]
    Format,
    #[error("Scene version {0} needs a newer drusty-pen")]
    NewerVersion(u32),
    #[error("Invalid scene data: {0}")]
    Invalid(String),
}

impl SceneFile {
    #[must_use]
    pub fn new(doc: &Document, app_state: AppState) -> Self {
        Self {
            dropped_bindings: 0,
            format: "drusty-pen".into(),
            version: VERSION,
            elements: doc.elements().to_vec(),
            app_state,
        }
    }

    pub fn document(&self) -> Result<Document, FileError> {
        self.validate()?;
        let mut doc = Document::new();
        let commands = self
            .elements
            .iter()
            .enumerate()
            .map(|(at, e)| Command::Add {
                element: e.clone(),
                at,
            })
            .collect();
        doc.apply(Command::Group(commands))
            .map_err(|e| FileError::Invalid(e.to_string()))?;
        Ok(doc)
    }

    fn validate(&self) -> Result<(), FileError> {
        if self.format != "drusty-pen" {
            return Err(FileError::Format);
        }
        if self.version > VERSION {
            return Err(FileError::NewerVersion(self.version));
        }
        let state = &self.app_state;
        if !state.scroll.is_finite()
            || !state.zoom.is_finite()
            || !(0.1..=30.0).contains(&state.zoom)
            || !valid_props(&state.style)
            || !state.font_size.is_finite()
            || !(1.0..=1000.0).contains(&state.font_size)
        {
            return Err(FileError::Invalid("invalid camera or defaults".into()));
        }
        for e in &self.elements {
            let bounds = e.kind.bounds();
            if e.id().get() == 0
                || e.id().get() == u64::MAX
                || !e.z_key.valid()
                || !e.angle.is_finite()
                || !bounds.min().is_finite()
                || !bounds.max().is_finite()
                || !bounds.width().is_finite()
                || !bounds.height().is_finite()
                || bounds.width() < 0.0
                || bounds.height() < 0.0
                || !valid_props(&e.props)
            {
                return Err(FileError::Invalid(
                    "invalid element geometry or style".into(),
                ));
            }
            if let Some(path) = e.kind.path()
                && (!path.origin.is_finite()
                    || path.points.is_empty()
                    || path.points.iter().any(|p| {
                        !p.offset.is_finite()
                            || !p.pressure.is_finite()
                            || !(0.0..=1.0).contains(&p.pressure)
                    })
                    || path.world_points().any(|p| !p.is_finite()))
            {
                return Err(FileError::Invalid("invalid path".into()));
            }
            if let Kind::Text { size, .. } = e.kind
                && (!size.is_finite() || !(1.0..=1000.0).contains(&size))
            {
                return Err(FileError::Invalid("invalid font size".into()));
            }
        }
        Ok(())
    }
}

fn valid_props(props: &Props) -> bool {
    props.opacity <= 100
        && props.stroke_width.is_finite()
        && (0.0..=1000.0).contains(&props.stroke_width)
}

pub fn load(bytes: &[u8]) -> Result<SceneFile, FileError> {
    // Check the envelope before decoding kinds, so future element kinds still report
    // a useful version error instead of a misleading malformed-JSON error.
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(json_error)?;
    if value.get("format").and_then(serde_json::Value::as_str) != Some("drusty-pen") {
        return Err(FileError::Format);
    }
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| FileError::Invalid("missing version".into()))?;
    if version > u64::from(VERSION) {
        return Err(FileError::NewerVersion(
            version.min(u64::from(u32::MAX)) as u32
        ));
    }
    let mut scene: SceneFile = serde_json::from_value(value).map_err(json_error)?;
    scene.version = VERSION;
    // A dangling text binding is damage, not a reason to lose the scene: the text stays,
    // unbound, where it was last drawn.
    scene.dropped_bindings =
        crate::bound::repair(&mut scene.elements) + crate::binding::repair(&mut scene.elements);
    scene.document()?;
    Ok(scene)
}

pub fn save(scene: &SceneFile) -> Result<Vec<u8>, FileError> {
    scene.document()?;
    serde_json::to_vec_pretty(scene).map_err(json_error)
}

fn json_error(error: serde_json::Error) -> FileError {
    if error.is_eof() {
        FileError::Truncated
    } else {
        FileError::Json(error)
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
