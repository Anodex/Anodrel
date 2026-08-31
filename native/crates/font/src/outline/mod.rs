//! Bounded TrueType simple-outline extraction.

mod curves;
mod error;
mod path;
mod simple;
mod source;
mod types;

pub use error::GlyphOutlineError;
pub use path::{GlyphPath, GlyphPathPoint, GlyphSegment};
pub use types::{GlyphBounds, GlyphOutline, GlyphPoint};

pub(crate) use source::OutlineSource;
