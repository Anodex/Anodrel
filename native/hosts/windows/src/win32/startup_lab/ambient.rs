//! Settled-frame cache and bounded ambient-mark repainting.

use super::*;

/// Length of one ambient cycle, in milliseconds.
///
/// The mark breathes across the whole cycle; the highlight crosses it once near
/// the start and is then absent, so the motion reads as light occasionally
/// catching a solid object rather than as a loop.
pub(in crate::win32) const AMBIENT_CYCLE_MILLIS: u64 = 7_200;

/// Fraction of a cycle the highlight takes to cross.
const SWEEP_FRACTION: f32 = 0.22;

thread_local! {
    /// The composed backdrop, keyed by client size.
    ///
    /// The backdrop is a full-surface radial gradient — around a million paint
    /// samples — and it is identical on every frame of the reveal. Computing it
    /// once per size turns the per-frame cost into a copy, which is the
    /// difference between a smooth reveal and a slideshow.
    pub(in crate::win32) static BACKDROP: RefCell<Option<Canvas>> = const { RefCell::new(None) };

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
pub(in crate::win32) fn ambient_region(width: f32, height: f32) -> Option<Rect> {
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
pub(in crate::win32) fn draw_ambient(
    canvas: &mut Canvas,
    lab: &StartupLab,
    elapsed_millis: u64,
) -> bool {
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
pub(super) fn draw_ambient_layers(canvas: &mut Canvas, elapsed_millis: u64) -> bool {
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
pub(in crate::win32) fn invalidate_caches() {
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
pub(super) fn draw_settled(
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
