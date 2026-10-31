//! Turns a `pen-core` document plus the `pen-editor` overlay into a backend-neutral draw list.
//!
//! Empty until M1 adds `DrawList`. Output is in world points; the app presents it through the
//! camera without rebuilding it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;
