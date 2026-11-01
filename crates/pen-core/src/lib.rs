//! The document model: elements, ids, geometry, hit testing, commands and the file format.
//!
//! Knows nothing about windows, input devices or rendering. Every coordinate here is in world
//! points: y down, unbounded in every direction.
#![forbid(unsafe_code)]

mod arrange;
pub use arrange::{GroupId, ZKey, rotate};
mod arrowheads;
pub mod binding;
pub mod bound;
pub mod routing;
pub use arrowheads::{Head, arrowheads};
mod color;
mod document;
mod element;
pub mod frames;
mod images;
pub use images::{BinaryFile, Crop, FileId};
pub mod file;
mod geometry;
mod hit;
pub mod import;
mod outline;
mod path;
mod props;
mod settings;
mod spatial;
pub use settings::Settings;
pub use spatial::{Anchor, SpatialIndex};
mod text;

pub use bound::Container;
pub use color::Srgba;
pub use document::{Command, DocError, Document};
pub use element::{Edges, Element, ElementId, Kind, Link};
pub use geometry::{WorldPt, WorldRect};
pub use outline::{TOLERANCE, outline};
pub use path::{Path, PathPoint};
pub use props::{
    ArrowType, Arrowhead, FillStyle, Property, Props, StrokeStyle, Value, VerticalAlign,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod hit_tests;

#[cfg(test)]
mod path_tests;

pub use text::{Font, Roughness, TextAlign};

#[cfg(test)]
mod bound_tests;

#[cfg(test)]
mod routing_tests;

#[cfg(test)]
mod import_tests;

#[cfg(test)]
mod frame_tests;
