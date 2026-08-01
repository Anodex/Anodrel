//! The window icon, rasterized from the brand mark at run time.
//!
//! The icon is drawn by the same renderer that draws the hero mark, from the
//! same geometry, so it cannot drift from the identity on screen. Generating it
//! also means the host ships no image file and needs no image decoder.

use anodrel_brand::{mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Paint, Rect, Stop, point};

use super::present::BitmapInfo;
use super::{Bool, Dword, Hdc, Uint};

type Hbitmap = isize;
type Hicon = isize;

const DIB_RGB_COLORS: Uint = 0;

#[repr(C)]
struct IconInfo {
    is_icon: Bool,
    hotspot_x: Dword,
    hotspot_y: Dword,
    mask: Hbitmap,
    color: Hbitmap,
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateDIBSection(
        device_context: Hdc,
        bitmap_info: *const BitmapInfo,
        usage: Uint,
        bits: *mut *mut core::ffi::c_void,
        section: isize,
        offset: Dword,
    ) -> Hbitmap;
    fn CreateBitmap(
        width: i32,
        height: i32,
        planes: Uint,
        bits_per_pixel: Uint,
        bits: *const core::ffi::c_void,
    ) -> Hbitmap;
    fn DeleteObject(object: isize) -> Bool;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateIconIndirect(info: *const IconInfo) -> Hicon;
}

/// Renders the app icon at `size` pixels square.
///
/// The treatment is the mark on a rounded plate, matching the product icon: a
/// bare mark loses its silhouette against a light taskbar.
fn render(size: u32) -> Canvas {
    let mut canvas = Canvas::new(size, size);
    let extent = size as f32;
    let plate = Rect::new(0.0, 0.0, extent, extent);
    let radius = extent * 0.22;

    canvas.fill_rounded_rect(
        plate,
        radius,
        &Paint::linear(
            point(0.0, 0.0),
            point(extent, extent),
            vec![
                Stop::new(0.0, palette::BACKDROP_LIFT),
                Stop::new(1.0, palette::BACKDROP),
            ],
        ),
    );
    canvas.stroke_rounded_rect(
        plate.inflate(-extent * 0.012),
        radius,
        (extent * 0.016).max(1.0),
        &Paint::solid(palette::PANEL_EDGE),
    );

    let inset = extent * 0.17;
    mark::draw(
        &mut canvas,
        Rect::new(inset, inset, extent - inset, extent - inset),
        if size >= 64 {
            MarkStyle::hero()
        } else {
            MarkStyle::compact()
        },
    );
    canvas
}

/// Builds a Windows icon handle from a rendered canvas.
///
/// Returns `None` if any handle cannot be created; a missing icon is cosmetic
/// and must not fail window creation.
fn to_icon(canvas: &Canvas) -> Option<Hicon> {
    let size = canvas.width() as i32;
    if size <= 0 || canvas.height() != canvas.width() {
        return None;
    }
    let info = BitmapInfo::top_down(size, size);
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();

    // SAFETY: `info` describes a size * size 32-bit top-down surface and stays
    // live for the call; `bits` receives the section's pixel pointer.
    let color = unsafe { CreateDIBSection(0, &info, DIB_RGB_COLORS, &mut bits, 0, 0) };
    if color == 0 || bits.is_null() {
        return None;
    }

    let pixel_count = (size as usize) * (size as usize);
    // SAFETY: CreateDIBSection guarantees `bits` addresses pixel_count 32-bit
    // pixels while the bitmap is alive, which spans this block.
    let destination = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u32>(), pixel_count) };
    for (target, source) in destination.iter_mut().zip(canvas.pixels()) {
        *target = *source;
    }

    // A 32-bit icon is composited from its alpha channel, but a fully zeroed
    // AND mask is still required for the classic drawing path.
    let mask_bytes = vec![0_u8; ((size as usize).div_ceil(16) * 2) * (size as usize)];
    // SAFETY: the buffer holds one bit per pixel with rows padded to a 16-bit
    // boundary, which is the layout CreateBitmap documents for 1bpp.
    let mask = unsafe { CreateBitmap(size, size, 1, 1, mask_bytes.as_ptr().cast()) };
    if mask == 0 {
        // SAFETY: `color` was created above and is not selected anywhere.
        unsafe { DeleteObject(color) };
        return None;
    }

    let icon_info = IconInfo {
        is_icon: 1,
        hotspot_x: 0,
        hotspot_y: 0,
        mask,
        color,
    };
    // SAFETY: both bitmaps are valid and unselected, and `icon_info` outlives
    // the call. CreateIconIndirect copies the bitmaps, so they are released
    // immediately afterward.
    let icon = unsafe { CreateIconIndirect(&icon_info) };
    // SAFETY: the bitmaps were copied into the icon and are owned here.
    unsafe {
        DeleteObject(mask);
        DeleteObject(color);
    }
    (icon != 0).then_some(icon)
}

/// Builds the small (title bar) and large (task switcher) icons.
///
/// Returns `(small, large)`, either of which may be `None`.
pub(super) fn create() -> (Option<Hicon>, Option<Hicon>) {
    (to_icon(&render(32)), to_icon(&render(256)))
}

#[cfg(test)]
mod tests {
    use super::render;
    use anodrel_brand::palette;

    #[test]
    fn the_icon_plate_is_opaque_at_its_centre() {
        let canvas = render(64);
        assert_eq!(canvas.pixel(32, 32).alpha, 255);
    }

    #[test]
    fn the_icon_corners_stay_transparent_for_the_rounded_plate() {
        let canvas = render(64);
        assert_eq!(canvas.pixel(0, 0).alpha, 0);
        assert_eq!(canvas.pixel(63, 0).alpha, 0);
        assert_eq!(canvas.pixel(0, 63).alpha, 0);
        assert_eq!(canvas.pixel(63, 63).alpha, 0);
    }

    #[test]
    fn the_mark_is_drawn_over_the_plate() {
        let canvas = render(128);
        // The apex sits on the centre line in the upper half of the mark.
        let apex = canvas.pixel(64, 40);
        assert_ne!(apex, palette::BACKDROP);
        assert!(apex.alpha > 0);
    }

    #[test]
    fn both_shipped_sizes_render() {
        for size in [32, 256] {
            let canvas = render(size);
            assert_eq!(canvas.width(), size);
            assert_eq!(canvas.height(), size);
        }
    }
}
