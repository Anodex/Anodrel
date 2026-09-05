//! Bounded single-line glyph runs from one first-party font face.
//!
//! This crate maps source-order Unicode scalars to glyph identifiers and
//! horizontal design-unit positions. It does not shape, draw, load, or cache
//! text. See `docs/TEXT_RUNS.md` for the complete boundary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod run;

pub use error::TextRunError;
pub use run::{MAX_RUN_ADVANCE_DESIGN_UNITS, MAX_RUN_GLYPHS, RunGlyph, TextRun};

#[cfg(test)]
mod tests;
