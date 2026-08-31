//! Bounded TrueType simple-outline extraction.

mod error;
mod simple;
mod source;
mod types;

pub use error::GlyphOutlineError;
pub use types::{GlyphBounds, GlyphOutline, GlyphPoint};

pub(crate) use source::OutlineSource;
