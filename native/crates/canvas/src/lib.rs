//! Anodrel's software rasterizer.
//!
//! `anodrel-canvas` draws anti-aliased vector graphics into a plain 32-bit
//! pixel buffer. It has no operating-system dependency, no third-party crate,
//! and no unsafe code, so the same drawing produces the same pixels under a
//! unit test as it does behind a native window. Presenting the buffer is the
//! host's job — on Windows that is one device-independent bitmap blit.
//!
//! # Why this exists
//!
//! The platform's brand surfaces need gradients, soft glows, and chamfered
//! edges. Windows GDI cannot draw those, and adopting a graphics library would
//! break the rule that shipped runtime code is implemented in Anodrel (Decision 0005).
//! A focused rasterizer is a few hundred lines and keeps that rule intact.
//!
//! # Model
//!
//! - [`Path`] is one or more closed polygonal contours. Curves are flattened by
//!   the builders, so the rasterizer has exactly one primitive to handle.
//! - [`Paint`] is a pure function from a point to a [`Color`]: a flat fill or a
//!   linear or radial gradient.
//! - [`Canvas`] composites paths through paints, with coverage anti-aliasing.
//! - [`Mask`] is a standalone coverage buffer that can be blurred, which is how
//!   glows and shadows are built.
//! - [`Bevel`] shades a path's edges from one light direction.
//!
//! Contours combine under the non-zero winding rule, so a hole is a contour
//! wound against the one enclosing it.
//!
//! # Coordinates
//!
//! Canvas space is in pixels with `y` increasing downward, and pixel centres
//! sit at half-integer coordinates. A rectangle from `2.0` to `6.0` therefore
//! covers exactly the four pixels `2..=5`.
//!
//! # Example
//!
//! ```
//! use anodrel_canvas::{Bevel, Canvas, Color, Paint, Path, Rect, point};
//!
//! let mut canvas = Canvas::new(320, 200);
//! canvas.clear(Color::hex(0x0A0E1A));
//!
//! // A violet-to-blue plate, lit from the upper left.
//! let plate = Path::rounded_rect(Rect::new(40.0, 40.0, 280.0, 160.0), 18.0);
//! let face = Paint::horizontal(40.0, 280.0, Color::hex(0xA855F7), Color::hex(0x3B82F6));
//! canvas.draw_glow(&plate, 24.0, 2, &Paint::solid(Color::hex(0x7C3AED).with_alpha(90)));
//! canvas.fill_beveled(&plate, &face, &Bevel::top_left(5.0));
//!
//! // The buffer is ready to hand to a platform blit.
//! assert_eq!(canvas.pixels().len(), 320 * 200);
//! assert_ne!(canvas.pixel(160, 100), Color::hex(0x0A0E1A));
//! # let _ = point(0.0, 0.0);
//! ```
//!
//! # Cost
//!
//! Filling is proportional to the area a shape actually covers, not to the size
//! of the canvas, so a screen made of many small pieces stays cheap. Blur is
//! linear in the masked area and independent of radius. Nothing is cached
//! between calls; a surface is expected to be drawn once per paint message.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bevel;
mod color;
mod geometry;
mod image;
mod paint;
mod path;
mod surface;

pub use bevel::Bevel;
pub use color::Color;
pub use geometry::{Point, Rect, point};
pub use image::Image;
pub use paint::{Paint, Stop, stop};
pub use path::{BevelBand, Path};
pub use surface::{Canvas, Mask};
