//! The Anodrel colour tokens.
//!
//! Every colour a first-party surface may use is named here. Surfaces reference
//! tokens rather than literals so the identity can be adjusted in one place and
//! so a review can tell brand colour from arbitrary colour.

use anodrel_canvas::Color;

/// The violet end of the mark's ramp, used for the leftmost geometry.
pub const VIOLET_LIGHT: Color = Color::hex(0xC77DFF);
/// The core brand violet.
pub const VIOLET: Color = Color::hex(0xA855F7);
/// Deep violet, used where the mark falls into shadow.
pub const VIOLET_DEEP: Color = Color::hex(0x7C3AED);
/// The indigo midpoint where violet hands off to blue.
pub const INDIGO: Color = Color::hex(0x6366F1);
/// The core brand blue.
pub const BLUE: Color = Color::hex(0x3B82F6);
/// Light blue, used for the mark's right-hand highlights.
pub const BLUE_LIGHT: Color = Color::hex(0x60A5FA);
/// The brightest blue in the ramp, at the mark's far right edge.
pub const SKY: Color = Color::hex(0x38BDF8);

/// The deepest background tone.
pub const BACKDROP: Color = Color::hex(0x05070F);
/// A lifted background tone used for the hero's centre.
pub const BACKDROP_LIFT: Color = Color::hex(0x0D1326);
/// The chrome band behind the wordmark and the status strip.
pub const CHROME: Color = Color::hex(0x080B16);
/// Card and panel fill.
pub const PANEL: Color = Color::hex(0x111729);
/// A slightly raised panel fill, used for hovered surfaces.
pub const PANEL_RAISED: Color = Color::hex(0x18203A);
/// Hairline borders around panels and cards.
pub const PANEL_EDGE: Color = Color::hex(0x222B45);

/// Primary text.
pub const INK: Color = Color::hex(0xF2F5FF);
/// Secondary text.
pub const INK_SOFT: Color = Color::hex(0xA9B4D0);
/// Tertiary text and inactive glyphs.
pub const INK_MUTED: Color = Color::hex(0x6E7A99);

/// The positive status signal.
pub const READY: Color = Color::hex(0x4ADE80);
/// The signal for a surface that is designed but not yet linked to a capability.
pub const PLANNED: Color = Color::hex(0x64748B);

/// Accent for the protocol core.
pub const ACCENT_CORE: Color = Color::hex(0x8B7CF6);
/// Accent for package verification.
pub const ACCENT_PACKAGE: Color = Color::hex(0x2DD4A7);
/// Accent for the private transport.
pub const ACCENT_IPC: Color = Color::hex(0xC084FC);
/// Accent for the native shell.
pub const ACCENT_SHELL: Color = Color::hex(0x3B9EF7);

/// The mark's left-to-right colour ramp.
///
/// Returned as positions paired with colours so callers can build either a
/// gradient across the whole mark or a matching ramp across a row of cards.
#[must_use]
pub fn mark_ramp() -> [(f32, Color); 5] {
    [
        (0.00, VIOLET_LIGHT),
        (0.26, VIOLET),
        (0.52, INDIGO),
        (0.78, BLUE),
        (1.00, SKY),
    ]
}

#[cfg(test)]
mod tests {
    use super::mark_ramp;

    #[test]
    fn the_mark_ramp_is_ordered_and_spans_the_full_axis() {
        let ramp = mark_ramp();
        assert_eq!(ramp[0].0, 0.0);
        assert_eq!(ramp[ramp.len() - 1].0, 1.0);
        for pair in ramp.windows(2) {
            assert!(pair[0].0 < pair[1].0, "ramp positions must ascend");
        }
    }

    #[test]
    fn the_ramp_travels_from_violet_to_blue() {
        let ramp = mark_ramp();
        let first = ramp[0].1;
        let last = ramp[ramp.len() - 1].1;
        assert!(first.red > last.red, "the ramp should lose red");
        assert!(
            first.blue >= last.blue - 8,
            "the ramp should stay blue-weighted"
        );
        assert!(
            last.green > first.green,
            "the ramp should gain green toward sky"
        );
    }
}
