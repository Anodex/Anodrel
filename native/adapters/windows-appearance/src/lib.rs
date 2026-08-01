#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows system-appearance values for host renderers.
//!
//! This adapter exposes only the current high-contrast flag and a small fixed
//! set of system colours. It neither draws, observes application content, nor
//! changes operating-system settings.

mod raw;

/// One opaque colour in standard red, green, blue channel order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
}

/// The small system-colour set useful to a native high-contrast renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemColors {
    /// Main window surface.
    pub window: Rgb,
    /// Foreground text on the main window surface.
    pub window_text: Rgb,
    /// Button and raised-surface face.
    pub button_face: Rgb,
    /// Foreground text on a button surface.
    pub button_text: Rgb,
    /// Selected control surface.
    pub highlight: Rgb,
    /// Foreground text on the selected control surface.
    pub highlight_text: Rgb,
}

/// The current Windows accessibility appearance relevant to host rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemAppearance {
    high_contrast: bool,
    colors: SystemColors,
}

impl SystemAppearance {
    /// Reads the current direct Windows high-contrast setting and colour set.
    ///
    /// If the system cannot report high-contrast state, the adapter preserves
    /// normal rendering and still supplies the system colours. The host must
    /// never write settings or infer accessibility state from application data.
    pub fn current() -> Self {
        Self {
            high_contrast: raw::high_contrast_enabled().unwrap_or(false),
            colors: raw::system_colors(),
        }
    }

    /// Whether Windows reports high-contrast mode as enabled.
    pub const fn high_contrast(self) -> bool {
        self.high_contrast
    }

    /// Current fixed system colours.
    pub const fn colors(self) -> SystemColors {
        self.colors
    }
}
