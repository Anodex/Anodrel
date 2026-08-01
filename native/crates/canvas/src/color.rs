//! Straight-alpha colour values and the blending rules used by the rasterizer.

/// A straight-alpha (non-premultiplied) 8-bit colour.
///
/// Channels are stored in RGBA order. Straight alpha is deliberate: brand
/// tokens stay readable as literals and the rasterizer premultiplies only at
/// the moment it composites.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Opacity, where `0` is fully transparent and `255` fully opaque.
    pub alpha: u8,
}

impl Color {
    /// Fully transparent black. Useful as a gradient terminator.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0, 0, 0);

    /// Builds an opaque colour.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }

    /// Builds a colour with explicit opacity.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Builds an opaque colour from a `0xRRGGBB` literal.
    ///
    /// Brand tokens are written this way so they can be compared directly with
    /// the design source.
    #[must_use]
    pub const fn hex(value: u32) -> Self {
        Self::rgb(
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        )
    }

    /// Returns the same colour at a new opacity.
    #[must_use]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { alpha, ..self }
    }

    /// Returns the same colour with its opacity scaled by `factor`.
    ///
    /// `factor` is clamped to `0.0..=1.0`. This is how reveal animations fade
    /// brand tokens without redefining them.
    #[must_use]
    pub fn scale_alpha(self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        self.with_alpha((f32::from(self.alpha) * factor).round() as u8)
    }

    /// Linearly interpolates every channel, including alpha.
    #[must_use]
    pub fn lerp(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self::rgba(
            lerp_channel(self.red, other.red, amount),
            lerp_channel(self.green, other.green, amount),
            lerp_channel(self.blue, other.blue, amount),
            lerp_channel(self.alpha, other.alpha, amount),
        )
    }

    /// Mixes `amount` of white into the colour, preserving alpha.
    #[must_use]
    pub fn lighten(self, amount: f32) -> Self {
        self.lerp(Self::WHITE.with_alpha(self.alpha), amount)
    }

    /// Mixes `amount` of black into the colour, preserving alpha.
    #[must_use]
    pub fn darken(self, amount: f32) -> Self {
        self.lerp(Self::BLACK.with_alpha(self.alpha), amount)
    }

    /// Composites `self` over an opaque `backdrop` using source-over.
    ///
    /// The result is always opaque, which is what text colours need: GDI draws
    /// glyphs without alpha, so a faded label is resolved against the exact
    /// pixels the rasterizer already produced.
    #[must_use]
    pub fn over(self, backdrop: Self) -> Self {
        let alpha = f32::from(self.alpha) / 255.0;
        Self::rgb(
            blend_channel(backdrop.red, self.red, alpha),
            blend_channel(backdrop.green, self.green, alpha),
            blend_channel(backdrop.blue, self.blue, alpha),
        )
    }

    /// Packs the colour as `0xAARRGGBB`.
    ///
    /// On little-endian targets the bytes land as `B, G, R, A`, which is the
    /// layout a 32-bit `BI_RGB` device-independent bitmap expects.
    #[must_use]
    pub const fn to_argb(self) -> u32 {
        ((self.alpha as u32) << 24)
            | ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
    }

    /// Unpacks a `0xAARRGGBB` value.
    #[must_use]
    pub const fn from_argb(value: u32) -> Self {
        Self::rgba(
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
            ((value >> 24) & 0xFF) as u8,
        )
    }
}

fn lerp_channel(from: u8, to: u8, amount: f32) -> u8 {
    let from = f32::from(from);
    (from + (f32::from(to) - from) * amount)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn blend_channel(backdrop: u8, source: u8, alpha: f32) -> u8 {
    let backdrop = f32::from(backdrop);
    (backdrop + (f32::from(source) - backdrop) * alpha)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn hex_matches_channel_construction() {
        assert_eq!(Color::hex(0xA855F7), Color::rgb(0xA8, 0x55, 0xF7));
    }

    #[test]
    fn argb_round_trips() {
        let color = Color::rgba(18, 52, 86, 120);
        assert_eq!(Color::from_argb(color.to_argb()), color);
    }

    #[test]
    fn transparent_source_leaves_the_backdrop_untouched() {
        let backdrop = Color::rgb(10, 20, 30);
        assert_eq!(Color::WHITE.with_alpha(0).over(backdrop), backdrop);
    }

    #[test]
    fn opaque_source_replaces_the_backdrop() {
        let backdrop = Color::rgb(10, 20, 30);
        assert_eq!(Color::WHITE.over(backdrop), Color::WHITE);
    }

    #[test]
    fn lighten_and_darken_move_toward_the_extremes() {
        let base = Color::rgb(100, 100, 100);
        assert!(base.lighten(0.5).red > base.red);
        assert!(base.darken(0.5).red < base.red);
        assert_eq!(base.lighten(0.5).alpha, base.alpha);
    }

    #[test]
    fn scale_alpha_is_clamped() {
        let base = Color::rgba(1, 2, 3, 200);
        assert_eq!(base.scale_alpha(0.0).alpha, 0);
        assert_eq!(base.scale_alpha(2.0).alpha, 200);
        assert_eq!(base.scale_alpha(0.5).alpha, 100);
    }
}
