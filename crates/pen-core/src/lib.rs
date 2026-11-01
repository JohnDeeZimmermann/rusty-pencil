//! The document model: elements, ids, geometry, hit testing, commands and the file format.
//!
//! Knows nothing about windows, input devices or rendering. Every coordinate here is in world
//! points: y down, unbounded in every direction.
#![forbid(unsafe_code)]

mod arrowheads;
pub mod bound;
pub use arrowheads::{Head, arrowheads};
mod color;
mod document;
mod element;
pub mod file;
mod geometry;
mod hit;
mod outline;
mod path;
mod props;
mod settings;
pub use settings::Settings;
mod text;

pub use bound::Container;
pub use color::Srgba;
pub use document::{Command, DocError, Document};
pub use element::{Edges, Element, ElementId, Kind};
pub use geometry::{WorldPt, WorldRect};
pub use outline::{TOLERANCE, outline};
pub use path::{Path, PathPoint};
pub use props::{Arrowhead, FillStyle, Property, Props, StrokeStyle, Value};

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
