//! The Anodrel design system as code.
//!
//! This crate owns the platform's visual identity: its colour tokens, the
//! geometry of the `A` mark, and the icon set. It draws through
//! [`anodrel_canvas`] and depends on no operating-system API, so a future macOS
//! or Linux host renders the identical identity from the identical source.
//!
//! # How the mark is carried
//!
//! The mark is the **authored artwork**, committed pre-decoded as raw pixels
//! (`assets/mark-512.bgra`). Storing it decoded means the platform displays its
//! own logo exactly, while still shipping no image decoder and taking no
//! dependency. A reconstruction, however careful, is a different mark.
//!
//! Geometry for the same mark is kept alongside it and used below
//! [`mark::RASTER_MIN_EDGE`], where reducing the raster would smear its
//! chamfers but a vector stays crisp. Both occupy identical bounds, so nothing
//! shifts when the renderer crosses the threshold. Decision 0015 records the
//! split; `assets/README.md` records the asset's provenance.
//!
//! # Layout
//!
//! - [`palette`] — every colour a first-party surface may use.
//! - [`mark`] — the authored mark, its geometry fallback, and its rendering.
//! - [`icon`] — line glyphs for status cards and action tiles.
//!
//! # Example
//!
//! ```
//! use anodrel_brand::{mark, mark::MarkStyle, palette};
//! use anodrel_canvas::{Canvas, Rect};
//!
//! let mut canvas = Canvas::new(400, 400);
//! canvas.clear(palette::BACKDROP);
//! mark::draw(&mut canvas, Rect::new(80.0, 60.0, 320.0, 300.0), MarkStyle::hero());
//!
//! // The apex sits on the centre line, so the mark is on the surface.
//! assert_ne!(canvas.pixel(200, 120), palette::BACKDROP);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod icon;
pub mod mark;
pub mod palette;

pub use icon::Icon;
pub use mark::{MarkStyle, Piece};
