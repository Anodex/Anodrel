//! The Anodrel Startup Lab surface.
//!
//! Everything on screen is composed into an Anodrel canvas and presented in one
//! blit. The layout is resolution-independent: it is authored against a base
//! size and scaled, so the same code serves a 100% and a 200% display.
//!
//! The screen states only what the host actually verified. Cards report checks
//! that ran during startup; action tiles that are not yet backed by a documented
//! host operation are drawn in a `planned` state rather than being presented as
//! working.

use std::cell::RefCell;

use anodrel_brand::{Icon, mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Color, Mask, Paint, Path, Point, Rect, Stop, point};

use super::text::{Align, TextSpec};
mod actions;
pub(super) mod ambient;
mod animation;
mod backdrop;
mod cards;
mod footer;
mod header;
mod hero;
mod model;

use super::{StartupLab, text};
pub(super) use actions::draw_actions;
pub(super) use ambient::{ambient_region, draw_ambient, invalidate_caches};
use ambient::{draw_ambient_layers, draw_settled};
pub(super) use animation::draw;
pub(super) use backdrop::draw_backdrop;
pub(super) use cards::draw_cards;
#[cfg(test)]
pub(super) use cards::{card_badge, card_status_top};
pub(super) use footer::draw_footer;
pub(super) use header::draw_header;
pub(super) use hero::{draw_hero_details, draw_hero_mark};
pub(super) use model::*;

#[cfg(test)]
mod tests;
