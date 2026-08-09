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
use super::{StartupLab, text};

/// The size the layout is authored against, in logical pixels.
pub(super) const BASE_WIDTH: f32 = 1240.0;
/// The height the layout is authored against, in logical pixels.
pub(super) const BASE_HEIGHT: f32 = 900.0;

/// Total length of the reveal, in milliseconds.
///
/// This is when the host stops the animation timer, so it must be at least as
/// long as the last stage below — otherwise the final element freezes part-way
/// through its fade. `the_reveal_settles_before_its_timer_stops` holds the two
/// in agreement.
pub(super) const REVEAL_MILLIS: u64 = 1_300;

const WEIGHT_REGULAR: i32 = 400;
const WEIGHT_MEDIUM: i32 = 500;
const WEIGHT_SEMIBOLD: i32 = 600;
const WEIGHT_BOLD: i32 = 700;

/// A status card describing one startup check.
struct Card {
    icon: Icon,
    accent: Color,
    title: &'static str,
    status: &'static str,
    detail: &'static str,
    badge: &'static str,
}

/// The four checks the host completes before this surface opens.
///
/// Each line corresponds to work that actually ran; see `docs/STARTUP_LAB.md`
/// for what each one does and does not claim.
const CARDS: [Card; 4] = [
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
pub(super) enum ActionKind {
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
pub(super) struct Action {
    /// What the tile does when it is linked.
    pub(super) kind: ActionKind,
    icon: Icon,
    accent: Color,
    title: &'static str,
    subtitle: &'static str,
    /// `false` while the operation behind the tile is still being built.
    ///
    /// A planned tile is drawn dimmed with a marker and does not respond to a
    /// click. It is shown rather than hidden so the surface reflects the whole
    /// intended shape of the platform; see `ROADMAP.md`.
    ///
    /// A tile whose availability depends on machine state declares `false` here
    /// and is resolved at run time by [`tile_is_live`].
    pub(super) linked: bool,
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
pub(super) fn tile_is_live(action: &Action, lab: &StartupLab) -> bool {
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
fn tile_subtitle(action: &Action, lab: &StartupLab) -> &'static str {
    if action.kind == ActionKind::LaunchDevelopmentFixture && lab.launch_available {
        "Development only, not a product"
    } else {
        action.subtitle
    }
}

/// The action strip, in display order.
pub(super) const ACTIONS: [Action; 4] = [
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
pub(super) struct Layout {
    scale: f32,
    width: f32,
    height: f32,
    header_height: f32,
    margin: f32,
    mark: Rect,
    title_baseline: f32,
    identity_baseline: f32,
    pill: Rect,
    cards_top: f32,
    card_size: (f32, f32),
    card_gap: f32,
    actions: Rect,
    footer_top: f32,
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

    fn unit(&self, value: f32) -> f32 {
        value * self.scale
    }

    fn font(&self, value: f32) -> i32 {
        (value * self.scale).round().max(1.0) as i32
    }

    fn card_rect(&self, index: usize) -> Rect {
        let (card_width, card_height) = self.card_size;
        let left = self.margin + (card_width + self.card_gap) * index as f32;
        Rect::from_size(left, self.cards_top, card_width, card_height)
    }

    fn action_rect(&self, index: usize) -> Rect {
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
pub(super) fn action_at(client_width: f32, client_height: f32, at: Point) -> Option<usize> {
    let layout = Layout::new(client_width, client_height);
    (0..ACTIONS.len()).find(|index| layout.action_rect(*index).contains(at))
}

/// Eased progress through one stage of the reveal.
///
/// Stages overlap deliberately: each element is still settling as the next
/// begins, which reads as one motion rather than a queue.
fn stage(elapsed_millis: u64, start: f32, duration: f32) -> f32 {
    let progress = ((elapsed_millis as f32 - start) / duration).clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powi(3)
}

/// Vertical offset for an element that slides up as it fades in.
fn rise(progress: f32, distance: f32) -> f32 {
    (1.0 - progress) * distance
}

/// Draws the whole surface.
pub(super) fn draw(canvas: &mut Canvas, lab: &StartupLab, elapsed_millis: u64) {
    let layout = Layout::new(canvas.width() as f32, canvas.height() as f32);

    // Once the reveal has settled, retain the invariant parts of the surface.
    // A full repaint and a partial ambient repaint then start from the exact
    // same base pixels, which keeps the optimization visually invisible.
    if elapsed_millis >= REVEAL_MILLIS && draw_settled(canvas, lab, &layout, elapsed_millis) {
        return;
    }

    draw_backdrop(canvas, &layout);
    draw_header(canvas, &layout, stage(elapsed_millis, 0.0, 340.0));
    draw_hero_mark(canvas, &layout, elapsed_millis);
    draw_hero_details(canvas, &layout, lab, elapsed_millis);
    draw_cards(canvas, &layout, elapsed_millis);
    draw_actions(canvas, &layout, lab, elapsed_millis);
    draw_footer(canvas, &layout, lab, stage(elapsed_millis, 820.0, 360.0));
}

/// Length of one ambient cycle, in milliseconds.
///
/// The mark breathes across the whole cycle; the highlight crosses it once near
/// the start and is then absent, so the motion reads as light occasionally
/// catching a solid object rather than as a loop.
pub(super) const AMBIENT_CYCLE_MILLIS: u64 = 7_200;

/// Fraction of a cycle the highlight takes to cross.
const SWEEP_FRACTION: f32 = 0.22;

thread_local! {
    /// The composed backdrop, keyed by client size.
    ///
    /// The backdrop is a full-surface radial gradient — around a million paint
    /// samples — and it is identical on every frame of the reveal. Computing it
    /// once per size turns the per-frame cost into a copy, which is the
    /// difference between a smooth reveal and a slideshow.
    static BACKDROP: RefCell<Option<Canvas>> = const { RefCell::new(None) };

    /// The settled hero, split into the layers ambient motion needs.
    static HERO: RefCell<Option<Hero>> = const { RefCell::new(None) };

    /// The settled base surface with the animated mark and its foreground
    /// details left out.
    ///
    /// Ambient motion restores this base, then redraws the mark and the title,
    /// identity, and validation pill above it. Keeping the foreground details
    /// out of the cache preserves their original draw order over the glow.
    static STATIC: RefCell<Option<StaticLayer>> = const { RefCell::new(None) };
}

/// Inputs that alter the cached settled base.
#[derive(PartialEq, Eq)]
struct StaticKey {
    width: u32,
    height: u32,
    display_name: String,
    application_id: String,
    startup_millis: u64,
    working_set_bytes: u64,
    hovered: Option<usize>,
}

impl StaticKey {
    fn from_surface(canvas: &Canvas, lab: &StartupLab) -> Self {
        Self {
            width: canvas.width(),
            height: canvas.height(),
            display_name: lab.package.display_name.clone(),
            application_id: lab.package.application_id.clone(),
            startup_millis: lab.startup_millis,
            working_set_bytes: lab.working_set_bytes,
            hovered: lab.hovered,
        }
    }
}

/// The settled base surface and the values it was built from.
struct StaticLayer {
    key: StaticKey,
    base: Canvas,
}

/// The hero mark pre-composed into independently animatable layers.
///
/// Ambient motion changes only two things: how brightly the glow burns and
/// where a highlight sits on the mark. Rendering the glow and the body once and
/// then compositing them costs a blend per pixel instead of a blur and a
/// resample, which is what makes continuous motion affordable at all.
struct Hero {
    /// Client-space bounds of the mark itself.
    bounds: Rect,
    /// Region the layers occupy, including the glow's reach.
    region: Rect,
    glow: Canvas,
    body: Canvas,
    /// The mark's own alpha, for confining the highlight to the artwork.
    mask: Mask,
}

impl Hero {
    fn build(bounds: Rect, layout: &Layout) -> Option<Self> {
        let style = MarkStyle::hero();
        let padding = (bounds.width() * style.glow_ratio * 1.6).ceil();
        // Stop the region at the header. Restoring the backdrop across the
        // chrome band would erase the wordmark, and the glow reaching that far
        // is faint enough that leaving it static shows no seam.
        let ceiling = Rect::new(0.0, layout.header_height, layout.width, layout.height);
        let region = bounds.inflate(padding).intersect(ceiling);
        if region.is_empty() {
            return None;
        }
        let left = region.left.floor();
        let top = region.top.floor();
        let width = (region.right.ceil() - left) as u32;
        let height = (region.bottom.ceil() - top) as u32;
        let local = bounds.translate(-left, -top);

        let mut glow = Canvas::new(width, height);
        mark::draw_glow_layer(&mut glow, local, style);
        let mut body = Canvas::new(width, height);
        mark::draw_body_layer(&mut body, local, style);
        let mask = mark::coverage_mask(bounds)?;

        Some(Self {
            bounds,
            region: Rect::new(left, top, left + width as f32, top + height as f32),
            glow,
            body,
            mask,
        })
    }
}

/// Returns the region ambient motion repaints, or `None` if it is unavailable.
///
/// Building the layers is the expensive part and happens here, on the first
/// call for a given size.
pub(super) fn ambient_region(width: f32, height: f32) -> Option<Rect> {
    let layout = Layout::new(width, height);
    HERO.with(|cache| {
        let mut slot = cache.borrow_mut();
        if slot.as_ref().is_none_or(|hero| hero.bounds != layout.mark) {
            *slot = Hero::build(layout.mark, &layout);
        }
        slot.as_ref()
            .map(|hero| hero.region.union(hero_foreground_region(&layout)))
    })
}

/// The static copy redrawn above the animated mark.
///
/// The title, identity, and validation pill are all translucent at some part
/// of their draw. Their complete bounds therefore have to be restored before
/// they are painted again; otherwise a partial frame would compound their
/// alpha just below the glow. The wide rectangle is intentional: a validated
/// display name may have any measured width, but the extra restored pixels are
/// still a small horizontal band rather than a full frame.
fn hero_foreground_region(layout: &Layout) -> Rect {
    Rect::new(
        0.0,
        layout.title_baseline - layout.unit(16.0),
        layout.width,
        layout.pill.bottom + layout.unit(4.0),
    )
}

/// Repaints only the hero region for one ambient frame.
///
/// Returns `false` when the cached base or mark layers are unavailable, in
/// which case the caller must fall back to a full compose.
pub(super) fn draw_ambient(canvas: &mut Canvas, lab: &StartupLab, elapsed_millis: u64) -> bool {
    let layout = Layout::new(canvas.width() as f32, canvas.height() as f32);
    let Some(region) = ambient_region(layout.width, layout.height) else {
        return false;
    };
    let restored = STATIC.with(|static_layer| {
        static_layer
            .borrow()
            .as_ref()
            .is_some_and(|cached| canvas.copy_region_from(&cached.base, region))
    });
    if !restored || !draw_ambient_layers(canvas, elapsed_millis) {
        return false;
    }
    draw_hero_details(canvas, &layout, lab, elapsed_millis);
    true
}

/// Draws the dynamic mark layers over pixels that are already restored.
fn draw_ambient_layers(canvas: &mut Canvas, elapsed_millis: u64) -> bool {
    let phase = (elapsed_millis % AMBIENT_CYCLE_MILLIS) as f32 / AMBIENT_CYCLE_MILLIS as f32;

    HERO.with(|hero_cache| {
        let hero_ref = hero_cache.borrow();
        let Some(hero) = hero_ref.as_ref() else {
            return false;
        };

        let left = hero.region.left as i32;
        let top = hero.region.top as i32;

        // Breathing: the glow layer is rendered at its brightest and then
        // modulated down, because a layer cannot be composited above full
        // opacity. The swing has to be wide enough to register at a glance —
        // a few percent is arithmetic nobody sees.
        let breath = 0.62 + 0.38 * (phase * std::f32::consts::TAU).sin().mul_add(0.5, 0.5);
        canvas.draw_canvas(&hero.glow, left, top, breath);
        canvas.draw_canvas(&hero.body, left, top, 1.0);

        if phase < SWEEP_FRACTION {
            let progress = phase / SWEEP_FRACTION;
            // Fade the band in and out across its pass so it never appears or
            // vanishes at an edge.
            let strength = (progress * std::f32::consts::PI).sin() * 0.62;
            canvas.fill_mask(
                &hero.mask,
                &mark::sweep_paint(hero.bounds, progress, strength),
            );
        }
        true
    })
}

/// Discards cached layers, so the next frame rebuilds them.
pub(super) fn invalidate_caches() {
    BACKDROP.with(|cache| *cache.borrow_mut() = None);
    HERO.with(|cache| *cache.borrow_mut() = None);
    STATIC.with(|cache| *cache.borrow_mut() = None);
}

/// Composes every settled element that sits below or outside the animated mark.
///
/// The title, identity, and validation pill deliberately stay out of this
/// canvas: they are foreground details over the glow and must be drawn after
/// the mark on both full and partial frames.
fn build_static_base(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab) {
    draw_backdrop(canvas, layout);
    draw_header(canvas, layout, 1.0);
    draw_cards(canvas, layout, REVEAL_MILLIS);
    draw_actions(canvas, layout, lab, REVEAL_MILLIS);
    draw_footer(canvas, layout, lab, 1.0);
}

/// Ensures a settled base is available for this exact diagnostic surface.
fn ensure_static_base(canvas: &Canvas, layout: &Layout, lab: &StartupLab) -> bool {
    let key = StaticKey::from_surface(canvas, lab);
    let current = STATIC.with(|static_layer| {
        static_layer
            .borrow()
            .as_ref()
            .is_some_and(|cached| cached.key == key)
    });
    if current {
        return true;
    }

    let mut base = Canvas::new(canvas.width(), canvas.height());
    build_static_base(&mut base, layout, lab);
    STATIC.with(|static_layer| {
        *static_layer.borrow_mut() = Some(StaticLayer { key, base });
    });
    true
}

/// Restores the settled base, then adds the mark and its foreground details.
fn draw_settled(
    canvas: &mut Canvas,
    lab: &StartupLab,
    layout: &Layout,
    elapsed_millis: u64,
) -> bool {
    if !ensure_static_base(canvas, layout, lab) {
        return false;
    }
    let restored = STATIC.with(|static_layer| {
        static_layer
            .borrow()
            .as_ref()
            .is_some_and(|cached| canvas.copy_from(&cached.base))
    });
    if !restored
        || ambient_region(layout.width, layout.height).is_none()
        || !draw_ambient_layers(canvas, elapsed_millis)
    {
        return false;
    }
    draw_hero_details(canvas, layout, lab, elapsed_millis);
    true
}

fn draw_backdrop(canvas: &mut Canvas, layout: &Layout) {
    let reused = BACKDROP.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .is_some_and(|cached| canvas.copy_from(cached))
    });
    if reused {
        return;
    }
    compose_backdrop(canvas, layout);
    let mut snapshot = Canvas::new(canvas.width(), canvas.height());
    if snapshot.copy_from(canvas) {
        BACKDROP.with(|cache| *cache.borrow_mut() = Some(snapshot));
    }
}

fn compose_backdrop(canvas: &mut Canvas, layout: &Layout) {
    canvas.clear(palette::BACKDROP);

    // A wide, low bloom behind the hero lifts the centre away from the frame.
    canvas.fill_rect(
        canvas.bounds(),
        &Paint::radial(
            point(
                layout.width / 2.0,
                layout.mark.center().y + layout.unit(30.0),
            ),
            layout.width * 0.62,
            vec![
                Stop::new(0.0, palette::BACKDROP_LIFT.with_alpha(235)),
                Stop::new(0.45, palette::BACKDROP_LIFT.with_alpha(90)),
                Stop::new(1.0, Color::TRANSPARENT),
            ],
        ),
    );

    // Faceted planes echoing the mark's geometry. They are kept to the outer
    // corners and to a few units of alpha: at this weight they register as
    // depth, while a straight edge crossing the composition would read as a
    // rendering artefact.
    let facets = [
        [point(0.0, 0.0), point(0.20, 0.0), point(0.0, 0.34)],
        [point(1.0, 0.0), point(0.82, 0.0), point(1.0, 0.30)],
        [point(0.0, 1.0), point(0.0, 0.74), point(0.17, 1.0)],
        [point(1.0, 1.0), point(1.0, 0.78), point(0.86, 1.0)],
    ];
    for triangle in facets {
        canvas.fill_path(
            &Path::polygon(
                triangle.map(|vertex| point(vertex.x * layout.width, vertex.y * layout.height)),
            ),
            &Paint::solid(palette::INDIGO.with_alpha(5)),
        );
    }
}

fn draw_header(canvas: &mut Canvas, layout: &Layout, progress: f32) {
    if progress <= 0.0 {
        return;
    }
    canvas.fill_rect(
        Rect::new(0.0, 0.0, layout.width, layout.header_height),
        &Paint::solid(palette::CHROME.with_alpha(215)),
    );
    canvas.fill_rect(
        Rect::new(
            0.0,
            layout.header_height - layout.unit(1.0),
            layout.width,
            layout.header_height,
        ),
        &Paint::solid(palette::PANEL_EDGE.with_alpha(160)),
    );

    let baseline = layout.header_height / 2.0;
    let wordmark = TextSpec::new("ANODREL", layout.font(30.0), WEIGHT_BOLD)
        .tracked(layout.unit(2.5).round() as i32);
    let wordmark_width = text::width(&wordmark);
    let wordmark_left = layout.margin;
    let wordmark_top = baseline - text::line_height(&wordmark) / 2.0;

    // The wordmark carries the mark's own ramp, so identity reads the same in
    // type as it does in geometry.
    text::draw(
        canvas,
        &wordmark,
        point(wordmark_left, wordmark_top),
        Align::Left,
        &Paint::linear(
            point(wordmark_left, 0.0),
            point(wordmark_left + wordmark_width, 0.0),
            palette::mark_ramp()
                .map(|(position, color)| Stop::new(position, color.scale_alpha(progress)))
                .to_vec(),
        ),
    );

    let divider_x = wordmark_left + wordmark_width + layout.unit(28.0);
    canvas.fill_rect(
        Rect::new(
            divider_x,
            baseline - layout.unit(13.0),
            divider_x + layout.unit(1.0),
            baseline + layout.unit(13.0),
        ),
        &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
    );

    let tagline = TextSpec::new(
        "Native Application Platform",
        layout.font(15.0),
        WEIGHT_REGULAR,
    );
    text::draw(
        canvas,
        &tagline,
        point(
            divider_x + layout.unit(26.0),
            baseline - text::line_height(&tagline) / 2.0,
        ),
        Align::Left,
        &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
    );

    let context = TextSpec::new("Windows Foundation", layout.font(15.0), WEIGHT_REGULAR);
    let surface = TextSpec::new("Startup Lab", layout.font(15.0), WEIGHT_MEDIUM);
    let separator = TextSpec::new(" / ", layout.font(15.0), WEIGHT_REGULAR);
    let total = text::width(&context) + text::width(&separator) + text::width(&surface);
    let right_top = baseline - text::line_height(&context) / 2.0;
    let mut cursor = layout.width - layout.margin - total;

    canvas.fill_circle(
        point(cursor - layout.unit(18.0), baseline),
        layout.unit(4.0),
        &Paint::solid(palette::VIOLET.scale_alpha(progress)),
    );
    cursor = text::draw_run(
        canvas,
        &context,
        point(cursor, right_top),
        &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
    );
    cursor = text::draw_run(
        canvas,
        &separator,
        point(cursor, right_top),
        &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
    );
    text::draw(
        canvas,
        &surface,
        point(cursor, right_top),
        Align::Left,
        &Paint::solid(palette::BLUE_LIGHT.scale_alpha(progress)),
    );
}

/// Draws the mark below the title, identity, and validation pill.
fn draw_hero_mark(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
    let mark_progress = stage(elapsed_millis, 120.0, 660.0);
    if mark_progress >= 1.0 {
        // A full paint already has its base pixels. The partial route restores
        // them before it reaches these same layers, so both paths produce the
        // same mark without drawing foreground text twice.
        if ambient_region(layout.width, layout.height).is_none()
            || !draw_ambient_layers(canvas, elapsed_millis)
        {
            mark::draw(canvas, layout.mark, MarkStyle::hero());
        }
    } else if mark_progress > 0.0 {
        // The mark settles from slightly small, so it reads as arriving rather
        // than simply appearing.
        let scale = 0.94 + 0.06 * mark_progress;
        let bounds = Rect::centered(
            layout.mark.center(),
            layout.mark.width() * scale,
            layout.mark.height() * scale,
        );
        mark::draw(
            canvas,
            bounds,
            MarkStyle::hero().with_opacity(mark_progress),
        );
    }
}

/// Draws the settled foreground details that sit above the mark glow.
fn draw_hero_details(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, elapsed_millis: u64) {
    let center_x = layout.width / 2.0;

    let title_progress = stage(elapsed_millis, 380.0, 440.0);
    if title_progress > 0.0 {
        let title = TextSpec::new(&lab.package.display_name, layout.font(44.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &title,
            point(
                center_x,
                layout.title_baseline + rise(title_progress, layout.unit(14.0)),
            ),
            Align::Center,
            &Paint::solid(palette::INK.scale_alpha(title_progress)),
        );
    }

    let identity_progress = stage(elapsed_millis, 470.0, 440.0);
    if identity_progress > 0.0 {
        let label = TextSpec::new("Validated Application", layout.font(17.0), WEIGHT_REGULAR);
        let separator = TextSpec::new("  /  ", layout.font(17.0), WEIGHT_REGULAR);
        let identifier = TextSpec::new(
            &lab.package.application_id,
            layout.font(17.0),
            WEIGHT_MEDIUM,
        );
        let total = text::width(&label) + text::width(&separator) + text::width(&identifier);
        let top = layout.identity_baseline + rise(identity_progress, layout.unit(12.0));
        let mut cursor = center_x - total / 2.0;
        cursor = text::draw_run(
            canvas,
            &label,
            point(cursor, top),
            &Paint::solid(palette::INK_SOFT.scale_alpha(identity_progress)),
        );
        cursor = text::draw_run(
            canvas,
            &separator,
            point(cursor, top),
            &Paint::solid(palette::INK_MUTED.scale_alpha(identity_progress)),
        );
        text::draw(
            canvas,
            &identifier,
            point(cursor, top),
            Align::Left,
            &Paint::solid(palette::BLUE_LIGHT.scale_alpha(identity_progress)),
        );
    }

    let pill_progress = stage(elapsed_millis, 560.0, 420.0);
    if pill_progress > 0.0 {
        let pill = layout
            .pill
            .translate(0.0, rise(pill_progress, layout.unit(10.0)));
        let radius = pill.height() / 2.0;
        canvas.fill_rounded_rect(
            pill,
            radius,
            &Paint::solid(palette::VIOLET.with_alpha(26).scale_alpha(pill_progress)),
        );
        canvas.stroke_rounded_rect(
            pill,
            radius,
            layout.unit(1.0).max(1.0),
            &Paint::solid(palette::VIOLET.with_alpha(120).scale_alpha(pill_progress)),
        );
        let glyph = Rect::from_size(
            pill.left + layout.unit(15.0),
            pill.center().y - layout.unit(7.5),
            layout.unit(15.0),
            layout.unit(15.0),
        );
        Icon::Package.draw(
            canvas,
            glyph,
            layout.unit(1.4).max(1.0),
            &Paint::solid(palette::VIOLET_LIGHT.scale_alpha(pill_progress)),
        );
        let caption = TextSpec::new("Validated", layout.font(14.0), WEIGHT_MEDIUM);
        text::draw(
            canvas,
            &caption,
            point(
                glyph.right + layout.unit(9.0),
                pill.center().y - text::line_height(&caption) / 2.0,
            ),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(pill_progress)),
        );
    }
}

fn draw_cards(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
    for (index, card) in CARDS.iter().enumerate() {
        let progress = stage(elapsed_millis, 600.0 + index as f32 * 70.0, 460.0);
        if progress <= 0.0 {
            continue;
        }
        let rect = layout
            .card_rect(index)
            .translate(0.0, rise(progress, layout.unit(18.0)));
        let radius = layout.unit(14.0);

        canvas.fill_rounded_rect(
            rect,
            radius,
            &Paint::linear(
                point(0.0, rect.top),
                point(0.0, rect.bottom),
                vec![
                    Stop::new(0.0, palette::PANEL.with_alpha(242).scale_alpha(progress)),
                    Stop::new(1.0, palette::PANEL.with_alpha(190).scale_alpha(progress)),
                ],
            ),
        );
        canvas.stroke_rounded_rect(
            rect,
            radius,
            layout.unit(1.0).max(1.0),
            &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
        );

        let badge = card_badge(layout, rect);
        let badge_size = badge.width();
        canvas.fill_circle(
            badge.center(),
            badge_size / 2.0,
            &Paint::solid(card.accent.with_alpha(28).scale_alpha(progress)),
        );
        canvas.stroke_path(
            &Path::circle(badge.center(), badge_size / 2.0),
            layout.unit(1.0).max(1.0),
            &Paint::solid(card.accent.with_alpha(110).scale_alpha(progress)),
        );
        card.icon.draw(
            canvas,
            badge.inflate(-layout.unit(13.0)),
            layout.unit(1.7).max(1.0),
            &Paint::solid(card.accent.scale_alpha(progress)),
        );

        let text_left = badge.right + layout.unit(16.0);
        let title = TextSpec::new(card.title, layout.font(17.0), WEIGHT_MEDIUM);
        let title_top = rect.top + layout.unit(24.0);
        text::draw(
            canvas,
            &title,
            point(text_left, title_top),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(progress)),
        );
        canvas.fill_circle(
            point(
                text_left + text::width(&title) + layout.unit(11.0),
                title_top + text::line_height(&title) / 2.0,
            ),
            layout.unit(4.0),
            &Paint::solid(palette::READY.scale_alpha(progress)),
        );

        // The status and detail lines run the card's full width, so they start
        // below the badge rather than beside it. They must clear it: the badge
        // shares their left edge, so an overlap puts the circle's arc straight
        // through the text.
        let status = TextSpec::new(card.status, layout.font(17.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &status,
            point(rect.left + layout.unit(20.0), card_status_top(layout, rect)),
            Align::Left,
            &Paint::solid(card.accent.scale_alpha(progress)),
        );

        let detail = TextSpec::new(card.detail, layout.font(13.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &detail,
            point(rect.left + layout.unit(20.0), rect.top + layout.unit(102.0)),
            Align::Left,
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
        );

        draw_chip(
            canvas,
            layout,
            point(
                rect.left + layout.unit(20.0),
                rect.bottom - layout.unit(32.0),
            ),
            card.badge,
            card.accent,
            progress,
        );
    }
}

/// Inset of a card's icon badge from the card's top-left corner.
const CARD_BADGE_INSET: f32 = 20.0;
/// Diameter of a card's icon badge.
const CARD_BADGE_SIZE: f32 = 46.0;
/// Top of a card's status line, measured from the card's top edge.
const CARD_STATUS_TOP: f32 = 74.0;

/// Returns a card's icon badge circle.
fn card_badge(layout: &Layout, rect: Rect) -> Rect {
    Rect::from_size(
        rect.left + layout.unit(CARD_BADGE_INSET),
        rect.top + layout.unit(CARD_BADGE_INSET),
        layout.unit(CARD_BADGE_SIZE),
        layout.unit(CARD_BADGE_SIZE),
    )
}

/// Returns the top of a card's status line.
fn card_status_top(layout: &Layout, rect: Rect) -> f32 {
    rect.top + layout.unit(CARD_STATUS_TOP)
}

fn draw_chip(
    canvas: &mut Canvas,
    layout: &Layout,
    at: Point,
    label: &str,
    accent: Color,
    progress: f32,
) {
    let spec = TextSpec::new(label, layout.font(11.0), WEIGHT_SEMIBOLD)
        .tracked(layout.unit(0.6).round() as i32);
    let padding = layout.unit(9.0);
    let height = layout.unit(21.0);
    let rect = Rect::from_size(at.x, at.y, text::width(&spec) + padding * 2.0, height);
    canvas.fill_rounded_rect(
        rect,
        layout.unit(5.0),
        &Paint::solid(accent.with_alpha(30).scale_alpha(progress)),
    );
    text::draw(
        canvas,
        &spec,
        point(
            rect.left + padding,
            rect.center().y - text::line_height(&spec) / 2.0,
        ),
        Align::Left,
        &Paint::solid(accent.scale_alpha(progress)),
    );
}

fn draw_actions(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, elapsed_millis: u64) {
    let progress = stage(elapsed_millis, 780.0, 400.0);
    if progress <= 0.0 {
        return;
    }
    let container = layout
        .actions
        .translate(0.0, rise(progress, layout.unit(16.0)));
    let radius = layout.unit(14.0);
    canvas.fill_rounded_rect(
        container,
        radius,
        &Paint::solid(palette::PANEL.with_alpha(200).scale_alpha(progress)),
    );
    canvas.stroke_rounded_rect(
        container,
        radius,
        layout.unit(1.0).max(1.0),
        &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
    );

    for (index, action) in ACTIONS.iter().enumerate() {
        let slot = layout
            .action_rect(index)
            .translate(0.0, rise(progress, layout.unit(16.0)));
        let live = tile_is_live(action, lab);
        let hovered = lab.hovered == Some(index) && live;
        let dim = if live { 1.0 } else { 0.45 };

        if hovered {
            canvas.fill_rounded_rect(
                slot.inflate(-layout.unit(4.0)),
                layout.unit(11.0),
                &Paint::solid(action.accent.with_alpha(30).scale_alpha(progress)),
            );
            canvas.stroke_rounded_rect(
                slot.inflate(-layout.unit(4.0)),
                layout.unit(11.0),
                layout.unit(1.0).max(1.0),
                &Paint::solid(action.accent.with_alpha(130).scale_alpha(progress)),
            );
        }

        if index > 0 {
            canvas.fill_rect(
                Rect::new(
                    slot.left,
                    slot.top + layout.unit(22.0),
                    slot.left + layout.unit(1.0),
                    slot.bottom - layout.unit(22.0),
                ),
                &Paint::solid(palette::PANEL_EDGE.with_alpha(150).scale_alpha(progress)),
            );
        }

        let badge_size = layout.unit(44.0);
        let badge = Rect::from_size(
            slot.left + layout.unit(22.0),
            slot.center().y - badge_size / 2.0,
            badge_size,
            badge_size,
        );
        canvas.fill_circle(
            badge.center(),
            badge_size / 2.0,
            &Paint::solid(
                action
                    .accent
                    .with_alpha(if hovered { 44 } else { 24 })
                    .scale_alpha(progress * dim),
            ),
        );
        action.icon.draw(
            canvas,
            badge.inflate(-layout.unit(12.0)),
            layout.unit(1.7).max(1.0),
            &Paint::solid(action.accent.scale_alpha(progress * dim)),
        );

        let text_left = badge.right + layout.unit(15.0);
        let title = TextSpec::new(action.title, layout.font(16.0), WEIGHT_MEDIUM);
        text::draw(
            canvas,
            &title,
            point(text_left, slot.center().y - layout.unit(19.0)),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(progress * dim)),
        );
        let subtitle = TextSpec::new(
            tile_subtitle(action, lab),
            layout.font(12.0),
            WEIGHT_REGULAR,
        );
        text::draw(
            canvas,
            &subtitle,
            point(text_left, slot.center().y + layout.unit(3.0)),
            Align::Left,
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress * dim)),
        );

        if live {
            draw_chevron(
                canvas,
                point(slot.right - layout.unit(28.0), slot.center().y),
                layout.unit(5.0),
                layout.unit(1.6).max(1.0),
                &Paint::solid(
                    if hovered {
                        action.accent
                    } else {
                        palette::INK_MUTED
                    }
                    .scale_alpha(progress),
                ),
            );
        } else {
            // Occupies the chevron's position, so a tile reads as "no action
            // here" rather than as an action that failed.
            let marker = TextSpec::new("PLANNED", layout.font(9.0), WEIGHT_SEMIBOLD)
                .tracked(layout.unit(0.6).round() as i32);
            text::draw(
                canvas,
                &marker,
                point(
                    slot.right - layout.unit(16.0),
                    slot.center().y - text::line_height(&marker) / 2.0,
                ),
                Align::Right,
                &Paint::solid(palette::PLANNED.scale_alpha(progress)),
            );
        }
    }
}

fn draw_chevron(canvas: &mut Canvas, at: Point, size: f32, width: f32, paint: &Paint) {
    canvas.draw_polyline(
        &[
            point(at.x - size * 0.5, at.y - size),
            point(at.x + size * 0.5, at.y),
            point(at.x - size * 0.5, at.y + size),
        ],
        width,
        paint,
    );
}

fn draw_footer(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, progress: f32) {
    if progress <= 0.0 {
        return;
    }
    let top = layout.footer_top;
    canvas.fill_rect(
        Rect::new(
            layout.margin,
            top,
            layout.width - layout.margin,
            top + layout.unit(1.0),
        ),
        &Paint::solid(palette::PANEL_EDGE.with_alpha(140).scale_alpha(progress)),
    );

    let row = top + layout.unit(26.0);
    let entries: [(Icon, &str, String, Color); 4] = [
        (
            Icon::Core,
            "Runtime",
            format!("v{}", env!("CARGO_PKG_VERSION")),
            palette::INK_SOFT,
        ),
        (
            Icon::Shell,
            "Memory",
            format!("{:.1} MB", lab.working_set_bytes as f32 / (1024.0 * 1024.0)),
            palette::BLUE_LIGHT,
        ),
        (
            Icon::Diagnostics,
            "Startup",
            // Foundation checks finish in single-digit milliseconds, which
            // rounds to nothing in seconds. Report the unit that has digits.
            if lab.startup_millis < 1_000 {
                format!("{} ms", lab.startup_millis)
            } else {
                format!("{:.2} s", lab.startup_millis as f32 / 1000.0)
            },
            palette::ACCENT_CORE,
        ),
        (Icon::Package, "Integrity", "OK".to_owned(), palette::READY),
    ];

    let mut cursor = layout.margin;
    for (index, (icon, label, value, tone)) in entries.into_iter().enumerate() {
        if index > 0 {
            canvas.fill_rect(
                Rect::new(
                    cursor,
                    row - layout.unit(9.0),
                    cursor + layout.unit(1.0),
                    row + layout.unit(19.0),
                ),
                &Paint::solid(palette::PANEL_EDGE.with_alpha(130).scale_alpha(progress)),
            );
            cursor += layout.unit(26.0);
        }
        let glyph = Rect::from_size(
            cursor,
            row - layout.unit(2.0),
            layout.unit(15.0),
            layout.unit(15.0),
        );
        icon.draw(
            canvas,
            glyph,
            layout.unit(1.3).max(1.0),
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
        );
        cursor = glyph.right + layout.unit(10.0);

        let name = TextSpec::new(label, layout.font(13.0), WEIGHT_REGULAR);
        cursor = text::draw_run(
            canvas,
            &name,
            point(cursor, row),
            &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
        ) + layout.unit(10.0);

        let reading = TextSpec::new(value, layout.font(13.0), WEIGHT_MEDIUM);
        cursor = text::draw_run(
            canvas,
            &reading,
            point(cursor, row),
            &Paint::solid(tone.scale_alpha(progress)),
        ) + layout.unit(26.0);
    }

    // The right-hand reading is the renderer describing itself: the previous
    // frame's cost, measured by the host that drew it.
    let renderer = TextSpec::new(
        format!(
            "SOFTWARE RENDERER  ·  {}×{}  ·  {:.1} ms",
            canvas.width(),
            canvas.height(),
            lab.last_frame_micros as f32 / 1000.0
        ),
        layout.font(12.0),
        WEIGHT_MEDIUM,
    )
    .tracked(layout.unit(0.4).round() as i32);
    text::draw(
        canvas,
        &renderer,
        point(layout.width - layout.margin, row),
        Align::Right,
        &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        ACTIONS, ActionKind, BASE_HEIGHT, BASE_WIDTH, CARDS, Layout, TextSpec, WEIGHT_MEDIUM,
        WEIGHT_REGULAR, action_at, stage, tile_is_live, tile_subtitle,
    };
    use anodrel_canvas::point;

    #[test]
    fn the_layout_keeps_every_region_inside_the_client_area() {
        for (width, height) in [(BASE_WIDTH, BASE_HEIGHT), (900.0, 660.0), (2400.0, 1500.0)] {
            let layout = Layout::new(width, height);
            assert!(
                layout.mark.top >= layout.header_height,
                "mark overlaps header"
            );
            for index in 0..CARDS.len() {
                let rect = layout.card_rect(index);
                assert!(
                    rect.left >= 0.0 && rect.right <= width,
                    "card {index} overflows"
                );
            }
            assert!(layout.actions.right <= width, "action strip overflows");
            assert!(layout.footer_top < height, "footer falls off the surface");
        }
    }

    #[test]
    fn the_card_status_line_clears_its_badge() {
        // The badge and the status line share a left edge, so any vertical
        // overlap draws the circle's arc straight through the text. This was
        // shipped once: the badge ran to `top + 72` while the status line
        // started at `top + 62`.
        for (width, height) in [
            (BASE_WIDTH, BASE_HEIGHT),
            (900.0, 660.0),
            (2_480.0, 1_800.0),
        ] {
            let layout = Layout::new(width, height);
            for index in 0..CARDS.len() {
                let rect = layout.card_rect(index);
                let badge = super::card_badge(&layout, rect);
                let status_top = super::card_status_top(&layout, rect);
                assert!(
                    status_top >= badge.bottom,
                    "card {index} at {width}x{height}: status starts at {status_top} \
                     but the badge runs to {}",
                    badge.bottom
                );
            }
        }
    }

    #[test]
    fn every_card_element_stays_inside_its_card() {
        let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
        for index in 0..CARDS.len() {
            let rect = layout.card_rect(index);
            let badge = super::card_badge(&layout, rect);
            assert!(badge.left >= rect.left && badge.right <= rect.right);
            assert!(badge.top >= rect.top && badge.bottom <= rect.bottom);
            assert!(
                super::card_status_top(&layout, rect) < rect.bottom,
                "card {index}: the status line falls outside the card"
            );
        }
    }

    #[test]
    fn the_hero_mark_is_square() {
        // The authored asset is a square crop and the geometry fills the unit
        // square, so a non-square target stretches the logo.
        for (width, height) in [(BASE_WIDTH, BASE_HEIGHT), (1_600.0, 1_000.0)] {
            let mark = Layout::new(width, height).mark;
            assert!(
                (mark.width() - mark.height()).abs() < 0.01,
                "the mark is {}x{} at {width}x{height}",
                mark.width(),
                mark.height()
            );
        }
    }

    #[test]
    fn cards_are_ordered_left_to_right_without_overlapping() {
        let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
        for index in 1..CARDS.len() {
            let previous = layout.card_rect(index - 1);
            let current = layout.card_rect(index);
            assert!(previous.right <= current.left, "cards {index} overlap");
        }
    }

    #[test]
    fn every_action_tile_is_hit_testable_at_its_own_centre() {
        let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
        for index in 0..ACTIONS.len() {
            let center = layout.action_rect(index).center();
            assert_eq!(
                action_at(BASE_WIDTH, BASE_HEIGHT, center),
                Some(index),
                "action {index} is not hit-testable"
            );
        }
    }

    #[test]
    fn points_outside_the_action_strip_hit_nothing() {
        assert_eq!(action_at(BASE_WIDTH, BASE_HEIGHT, point(4.0, 4.0)), None);
        assert_eq!(
            action_at(
                BASE_WIDTH,
                BASE_HEIGHT,
                point(BASE_WIDTH / 2.0, BASE_HEIGHT - 4.0)
            ),
            None
        );
    }

    #[test]
    fn hit_testing_follows_the_layout_when_the_window_is_resized() {
        let width = 1600.0;
        let height = 1000.0;
        let layout = Layout::new(width, height);
        let center = layout.action_rect(2).center();
        assert_eq!(action_at(width, height, center), Some(2));
    }

    #[test]
    fn a_stage_runs_from_zero_to_one_and_holds() {
        assert_eq!(stage(0, 100.0, 200.0), 0.0);
        assert!(stage(200, 100.0, 200.0) > 0.0);
        assert!(stage(200, 100.0, 200.0) < 1.0);
        assert_eq!(stage(300, 100.0, 200.0), 1.0);
        assert_eq!(stage(9_999, 100.0, 200.0), 1.0);
    }

    #[test]
    fn only_actions_backed_by_a_host_operation_are_linked() {
        for action in &ACTIONS {
            let expected = matches!(
                action.kind,
                ActionKind::OpenLogs | ActionKind::InspectPackage | ActionKind::RuntimeDiagnostics
            );
            assert_eq!(
                action.linked, expected,
                "{:?} is linked without a documented host operation behind it",
                action.kind
            );
        }
    }

    #[test]
    fn the_launch_tile_is_inert_until_a_preflight_says_the_fixture_validated() {
        let unprovisioned = super::super::tests::startup_lab_fixture(false);
        let provisioned = super::super::tests::startup_lab_fixture(true);

        for action in &ACTIONS {
            let live_when_unprovisioned = tile_is_live(action, &unprovisioned);
            if action.kind == ActionKind::LaunchDevelopmentFixture {
                assert!(
                    !live_when_unprovisioned,
                    "the launch tile is live without a validated fixture"
                );
                assert!(
                    tile_is_live(action, &provisioned),
                    "the launch tile stays inert after a successful preflight"
                );
            } else {
                // Every other tile displays values the host already held, so
                // machine provisioning must not change its availability.
                assert_eq!(live_when_unprovisioned, action.linked);
                assert_eq!(tile_is_live(action, &provisioned), action.linked);
            }
        }
    }

    #[test]
    fn every_tile_label_fits_its_slot_at_the_smallest_supported_size() {
        // Tile text is drawn from the badge's right edge to the slot's right
        // edge and is never wrapped or ellipsized, so a label that overruns is
        // simply painted over its neighbour.
        let unprovisioned = super::super::tests::startup_lab_fixture(false);
        let provisioned = super::super::tests::startup_lab_fixture(true);

        for (width, height) in [(900.0, 660.0), (BASE_WIDTH, BASE_HEIGHT)] {
            let layout = Layout::new(width, height);
            for (index, action) in ACTIONS.iter().enumerate() {
                let slot = layout.action_rect(index);
                // Mirrors the badge geometry the drawing code uses.
                let text_left =
                    slot.left + layout.unit(22.0) + layout.unit(44.0) + layout.unit(15.0);
                let available = slot.right - text_left - layout.unit(30.0);

                for label in [
                    action.title,
                    tile_subtitle(action, &unprovisioned),
                    tile_subtitle(action, &provisioned),
                ] {
                    let size = if label == action.title { 16.0 } else { 12.0 };
                    let weight = if label == action.title {
                        WEIGHT_MEDIUM
                    } else {
                        WEIGHT_REGULAR
                    };
                    let measured =
                        super::text::width(&TextSpec::new(label, layout.font(size), weight));
                    assert!(
                        measured <= available,
                        "{label:?} needs {measured} of {available} at {width}x{height}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_launch_tile_names_itself_a_development_fixture_in_both_states() {
        // The tile must never read as a product launch. The word "development"
        // in its title is what stops it being mistaken for one.
        let launch = ACTIONS
            .iter()
            .find(|action| action.kind == ActionKind::LaunchDevelopmentFixture)
            .expect("the launch tile exists");
        assert!(launch.title.contains("Development"));

        let unprovisioned = super::super::tests::startup_lab_fixture(false);
        assert_eq!(tile_subtitle(launch, &unprovisioned), "Not provisioned");

        let provisioned = super::super::tests::startup_lab_fixture(true);
        let live = tile_subtitle(launch, &provisioned);
        assert_eq!(live, "Development only, not a product");
        // A live tile must not claim more than it is. "Verified" beside a
        // launch control invites reading a fixture as a product.
        assert!(!live.to_ascii_lowercase().contains("verified"));
    }
}
