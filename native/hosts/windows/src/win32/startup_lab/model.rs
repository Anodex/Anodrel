//! Declarative Startup Lab content, geometry, and hit testing.

use super::*;

/// The size the layout is authored against, in logical pixels.
pub(in crate::win32) const BASE_WIDTH: f32 = 1240.0;
/// The height the layout is authored against, in logical pixels.
pub(in crate::win32) const BASE_HEIGHT: f32 = 900.0;

/// Total length of the reveal, in milliseconds.
///
/// This is when the host stops the animation timer, so it must be at least as
/// long as the last stage below — otherwise the final element freezes part-way
/// through its fade. `the_reveal_settles_before_its_timer_stops` holds the two
/// in agreement.
pub(in crate::win32) const REVEAL_MILLIS: u64 = 1_300;

pub(in crate::win32) const WEIGHT_REGULAR: i32 = 400;
pub(in crate::win32) const WEIGHT_MEDIUM: i32 = 500;
pub(in crate::win32) const WEIGHT_SEMIBOLD: i32 = 600;
pub(in crate::win32) const WEIGHT_BOLD: i32 = 700;

/// A status card describing one startup check.
pub(in crate::win32) struct Card {
    pub(in crate::win32) icon: Icon,
    pub(in crate::win32) accent: Color,
    pub(in crate::win32) title: &'static str,
    pub(in crate::win32) status: &'static str,
    pub(in crate::win32) detail: &'static str,
    pub(in crate::win32) badge: &'static str,
}

/// The four checks the host completes before this surface opens.
///
/// Each line corresponds to work that actually ran; see `docs/STARTUP_LAB.md`
/// for what each one does and does not claim.
pub(in crate::win32) const CARDS: [Card; 4] = [
    Card {
        icon: Icon::Core,
        accent: palette::ACCENT_CORE,
        title: "Platform Core",
        status: "platform.health: ready",
        detail: "strict JSON + policy core",
        badge: "HEALTHY",
    },
    Card {
        icon: Icon::Package,
        accent: palette::ACCENT_PACKAGE,
        title: "Verified Package",
        status: "manifest + digest: ready",
        detail: "canonical containment",
        badge: "VERIFIED",
    },
    Card {
        icon: Icon::Ipc,
        accent: palette::ACCENT_IPC,
        title: "Private IPC",
        status: "loopback self-test: ready",
        detail: "auth + health round trip",
        badge: "SECURE",
    },
    Card {
        icon: Icon::Shell,
        accent: palette::ACCENT_SHELL,
        title: "Native Shell",
        status: "native surface: ready",
        detail: "direct Win32 + software renderer",
        badge: "ACTIVE",
    },
];

/// What a click on an action tile does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::win32) enum ActionKind {
    /// Start the development product fixture.
    ///
    /// This is never a product launch. The identity it activates is a
    /// compile-time constant naming the development fixture in
    /// `docs/PRODUCT_FIXTURE.md`, and the tile is inert unless a live preflight
    /// validates that fixture's machine record and signature.
    LaunchDevelopmentFixture,
    /// Show runtime logs. Requires a logging boundary.
    OpenLogs,
    /// Open a native view of the verified package facts.
    InspectPackage,
    /// Open a native view of the runtime and renderer state.
    RuntimeDiagnostics,
}

/// An action tile in the strip below the status cards.
pub(in crate::win32) struct Action {
    /// What the tile does when it is linked.
    pub(in crate::win32) kind: ActionKind,
    pub(in crate::win32) icon: Icon,
    pub(in crate::win32) accent: Color,
    pub(in crate::win32) title: &'static str,
    pub(in crate::win32) subtitle: &'static str,
    /// `false` while the operation behind the tile is still being built.
    ///
    /// A planned tile is drawn dimmed with a marker and does not respond to a
    /// click. It is shown rather than hidden so the surface reflects the whole
    /// intended shape of the platform; see `ROADMAP.md`.
    ///
    /// A tile whose availability depends on machine state declares `false` here
    /// and is resolved at run time by [`tile_is_live`].
    pub(in crate::win32) linked: bool,
}

/// Whether a tile may be drawn as live and respond to a click.
///
/// Drawing and hit-testing both read this one function, so a tile can never be
/// made clickable by changing how it looks, and never drawn as available while
/// it is inert.
///
/// `LaunchDevelopmentFixture` is the only tile whose answer depends on machine
/// state. It requires a verification-only preflight — machine record, locked
/// digest revalidation, Authenticode, and publisher fingerprint — to have
/// succeeded before this surface opened. On a machine with no provisioned
/// record, or one whose executable or signature no longer validates, it stays
/// planned.
#[must_use]
pub(in crate::win32) fn tile_is_live(action: &Action, lab: &StartupLab) -> bool {
    match action.kind {
        ActionKind::LaunchDevelopmentFixture => lab.launch_available,
        _ => action.linked,
    }
}

/// The subtitle a tile shows for the current machine state.
///
/// A live fixture tile says what it is rather than what it proves. Reading
/// "verified" next to a launch control would invite treating a development
/// fixture as a product launch, which is exactly what it is not.
pub(in crate::win32) fn tile_subtitle(action: &Action, lab: &StartupLab) -> &'static str {
    if action.kind == ActionKind::LaunchDevelopmentFixture && lab.launch_available {
        "Development only, not a product"
    } else {
        action.subtitle
    }
}

/// The marker drawn at a tile's right edge when it has no action.
///
/// It replaces the chevron rather than joining it, so a tile reads as "no
/// action here" rather than as an action that failed.
pub(in crate::win32) const PLANNED_MARKER: &str = "PLANNED";

/// Builds the planned marker's text at a layout's scale.
///
/// Drawing and [`tile_subtitle_limit`] both build it here, so the room reserved
/// for it is measured from the same text that gets painted.
pub(in crate::win32) fn planned_marker(layout: &Layout) -> TextSpec {
    TextSpec::new(PLANNED_MARKER, layout.font(9.0), WEIGHT_SEMIBOLD)
        .tracked(layout.unit(0.6).round() as i32)
}

/// Tops of a tile's title and subtitle lines.
///
/// Returned together, and read by both drawing and
/// `the_planned_marker_clears_the_title_it_sits_below`, so the two lines and
/// anything aligned to them cannot drift apart.
pub(in crate::win32) fn tile_text_rows(layout: &Layout, slot: Rect) -> (f32, f32) {
    (
        slot.center().y - layout.unit(19.0),
        slot.center().y + layout.unit(3.0),
    )
}

/// A tile's right-hand furniture: where it is drawn, and the room it takes.
///
/// Drawing reads `anchor` to place the chevron or the marker, and
/// `every_tile_label_fits_its_slot_at_the_smallest_supported_size` reads the two
/// limits. One function answers both, so a label can never be measured against
/// room its marker is really using — which is exactly how the marker came to be
/// painted through the fixture tile's title. Nothing here wraps or ellipsizes;
/// a label that overruns is simply painted over whatever it meets.
pub(in crate::win32) struct TileMarker {
    /// Chevron centre on a live tile, marker right edge on a planned one.
    pub(in crate::win32) anchor: f32,
    /// The x the title must stay left of.
    pub(in crate::win32) title_limit: f32,
    /// The x the subtitle must stay left of.
    pub(in crate::win32) subtitle_limit: f32,
}

/// Resolves a tile's marker geometry.
///
/// A live tile's chevron is centred between the two lines of text, so both have
/// to clear it. The planned marker is a word rather than a few pixels and sits
/// on the subtitle's line, which is what leaves the title the whole slot: the
/// tile carrying the longest title is the one that stays planned until a
/// machine is provisioned.
pub(in crate::win32) fn tile_marker(layout: &Layout, slot: Rect, live: bool) -> TileMarker {
    if live {
        let limit = slot.right - layout.unit(30.0);
        TileMarker {
            anchor: slot.right - layout.unit(28.0),
            title_limit: limit,
            subtitle_limit: limit,
        }
    } else {
        let anchor = slot.right - layout.unit(16.0);
        TileMarker {
            anchor,
            title_limit: anchor,
            subtitle_limit: anchor - text::width(&planned_marker(layout)) - layout.unit(10.0),
        }
    }
}

/// The action strip, in display order.
pub(in crate::win32) const ACTIONS: [Action; 4] = [
    Action {
        kind: ActionKind::LaunchDevelopmentFixture,
        icon: Icon::Launch,
        accent: palette::ACCENT_CORE,
        // "Development Product Fixture" in full overruns this slot at the
        // smallest supported window; the two lines together carry the label,
        // and the window it opens is titled in full.
        title: "Development Fixture",
        subtitle: "Not provisioned",
        linked: false,
    },
    Action {
        kind: ActionKind::OpenLogs,
        icon: Icon::Logs,
        accent: palette::ACCENT_PACKAGE,
        title: "Open Logs",
        subtitle: "Safe host events",
        linked: true,
    },
    Action {
        kind: ActionKind::InspectPackage,
        icon: Icon::Inspect,
        accent: palette::ACCENT_IPC,
        title: "Inspect Package",
        subtitle: "Manifest, digest & limits",
        linked: true,
    },
    Action {
        kind: ActionKind::RuntimeDiagnostics,
        icon: Icon::Diagnostics,
        accent: palette::ACCENT_SHELL,
        title: "Runtime Diagnostics",
        subtitle: "Health, IPC & renderer",
        linked: true,
    },
];

/// Resolved pixel geometry for one client size.
///
/// Drawing and hit-testing both read this, so a tile can never be painted in a
/// place where it cannot be clicked.
pub(in crate::win32) struct Layout {
    pub(in crate::win32) scale: f32,
    pub(in crate::win32) width: f32,
    pub(in crate::win32) height: f32,
    pub(in crate::win32) header_height: f32,
    pub(in crate::win32) margin: f32,
    pub(in crate::win32) mark: Rect,
    pub(in crate::win32) title_baseline: f32,
    pub(in crate::win32) identity_baseline: f32,
    pub(in crate::win32) pill: Rect,
    pub(in crate::win32) cards_top: f32,
    pub(in crate::win32) card_size: (f32, f32),
    pub(in crate::win32) card_gap: f32,
    pub(in crate::win32) actions: Rect,
    pub(in crate::win32) footer_top: f32,
}

impl Layout {
    /// Resolves the layout for a client area.
    ///
    /// The scale is driven by the smaller axis so nothing overflows when the
    /// window is resized away from its designed aspect ratio.
    pub(super) fn new(width: f32, height: f32) -> Self {
        let scale = (width / BASE_WIDTH)
            .min(height / BASE_HEIGHT)
            .clamp(0.55, 3.0);
        let unit = |value: f32| value * scale;
        let center_x = width / 2.0;

        let header_height = unit(76.0);
        let margin = unit(46.0);
        // Square: the authored asset is a square crop and the geometry fills
        // the unit square, so any other aspect stretches the logo.
        let mark_side = unit(220.0);
        let (mark_width, mark_height) = (mark_side, mark_side);
        let mark_top = header_height + unit(38.0);
        let mark = Rect::from_size(
            center_x - mark_width / 2.0,
            mark_top,
            mark_width,
            mark_height,
        );

        // The title clears the mark's glow, not just its geometry: the bloom
        // extends well past the artwork, and measuring from the artwork alone
        // leaves the heading looking crowded.
        let title_baseline = mark.bottom + unit(44.0);
        let identity_baseline = title_baseline + unit(56.0);
        let pill_width = unit(132.0);
        let pill_height = unit(30.0);
        let pill = Rect::from_size(
            center_x - pill_width / 2.0,
            identity_baseline + unit(28.0),
            pill_width,
            pill_height,
        );

        let card_gap = unit(15.0);
        let cards_top = pill.bottom + unit(38.0);
        let card_width = (width - margin * 2.0 - card_gap * 3.0) / 4.0;
        // Tall enough for the status line to clear the icon badge above it;
        // `the_card_status_line_clears_its_badge` holds the two in agreement.
        let card_height = unit(158.0);

        let actions_top = cards_top + card_height + unit(22.0);
        let actions = Rect::from_size(margin, actions_top, width - margin * 2.0, unit(84.0));

        Self {
            scale,
            width,
            height,
            header_height,
            margin,
            mark,
            title_baseline,
            identity_baseline,
            pill,
            cards_top,
            card_size: (card_width, card_height),
            card_gap,
            // Anchored to the bottom rather than carried along by the flow, so
            // the strip keeps a margin comparable to the sides at any height
            // instead of drifting into the edge.
            footer_top: height - unit(80.0),
            actions,
        }
    }

    pub(in crate::win32) fn unit(&self, value: f32) -> f32 {
        value * self.scale
    }

    pub(in crate::win32) fn font(&self, value: f32) -> i32 {
        (value * self.scale).round().max(1.0) as i32
    }

    pub(in crate::win32) fn card_rect(&self, index: usize) -> Rect {
        let (card_width, card_height) = self.card_size;
        let left = self.margin + (card_width + self.card_gap) * index as f32;
        Rect::from_size(left, self.cards_top, card_width, card_height)
    }

    pub(in crate::win32) fn action_rect(&self, index: usize) -> Rect {
        let slot = self.actions.width() / ACTIONS.len() as f32;
        Rect::from_size(
            self.actions.left + slot * index as f32,
            self.actions.top,
            slot,
            self.actions.height(),
        )
    }
}

/// Returns the index of the action tile under a client-area point.
pub(in crate::win32) fn action_at(
    client_width: f32,
    client_height: f32,
    at: Point,
) -> Option<usize> {
    let layout = Layout::new(client_width, client_height);
    (0..ACTIONS.len()).find(|index| layout.action_rect(*index).contains(at))
}

/// Eased progress through one stage of the reveal.
///
/// Stages overlap deliberately: each element is still settling as the next
/// begins, which reads as one motion rather than a queue.
pub(in crate::win32) fn stage(elapsed_millis: u64, start: f32, duration: f32) -> f32 {
    let progress = ((elapsed_millis as f32 - start) / duration).clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powi(3)
}

/// Vertical offset for an element that slides up as it fades in.
pub(in crate::win32) fn rise(progress: f32, distance: f32) -> f32 {
    (1.0 - progress) * distance
}
