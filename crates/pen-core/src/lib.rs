//! The document model: elements, ids, geometry, hit testing, commands and the file format.
//!
//! Knows nothing about windows, input devices or rendering. Every coordinate here is in world
//! points: y down, unbounded in every direction.
#![forbid(unsafe_code)]

mod color;
mod document;
mod element;
mod geometry;

pub use color::Srgba;
pub use document::{Command, DocError, Document};
pub use element::{Edges, Element, ElementId, Kind, Props};
pub use geometry::{WorldPt, WorldRect};

#[cfg(test)]
mod tests;
