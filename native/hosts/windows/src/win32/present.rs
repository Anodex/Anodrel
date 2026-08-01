//! Presenting an owned canvas through a device-independent bitmap.
//!
//! The whole client area is composed in memory and reaches the screen in one
//! call. Nothing is drawn directly to the window device context, so there is no
//! intermediate state for the compositor to show and no flicker to suppress.

use anodrel_canvas::Canvas;

use super::{Dword, Hdc, Uint};

const BI_RGB: Dword = 0;
const DIB_RGB_COLORS: Uint = 0;
const SRCCOPY: Dword = 0x00CC_0020;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct BitmapInfoHeader {
    pub(super) biSize: Dword,
    pub(super) biWidth: i32,
    pub(super) biHeight: i32,
    pub(super) biPlanes: u16,
    pub(super) biBitCount: u16,
    pub(super) biCompression: Dword,
    pub(super) biSizeImage: Dword,
    pub(super) biXPelsPerMeter: i32,
    pub(super) biYPelsPerMeter: i32,
    pub(super) biClrUsed: Dword,
    pub(super) biClrImportant: Dword,
}

/// A `BITMAPINFO` for a 32-bit uncompressed bitmap.
///
/// The colour table is unused at this depth but the layout must still match
/// what the API expects, so one entry is carried.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct BitmapInfo {
    pub(super) header: BitmapInfoHeader,
    pub(super) colors: [Dword; 1],
}

impl BitmapInfo {
    /// Describes a top-down 32-bit surface.
    ///
    /// A negative height selects top-down row order, matching the canvas's
    /// row-major layout and removing the need to flip anything.
    pub(super) fn top_down(width: i32, height: i32) -> Self {
        Self {
            header: BitmapInfoHeader {
                biSize: size_of::<BitmapInfoHeader>() as Dword,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..BitmapInfoHeader::default()
            },
            colors: [0],
        }
    }
}

#[link(name = "gdi32")]
unsafe extern "system" {
    #[cfg_attr(not(test), allow(dead_code))]
    fn CreateCompatibleDC(device_context: Hdc) -> Hdc;
    #[cfg_attr(not(test), allow(dead_code))]
    fn DeleteDC(device_context: Hdc) -> i32;
    #[cfg_attr(not(test), allow(dead_code))]
    fn CreateDIBSection(
        device_context: Hdc,
        bitmap_info: *const BitmapInfo,
        usage: Uint,
        bits: *mut *mut core::ffi::c_void,
        section: isize,
        offset: Dword,
    ) -> isize;
    #[cfg_attr(not(test), allow(dead_code))]
    fn SelectObject(device_context: Hdc, object: isize) -> isize;
    #[cfg_attr(not(test), allow(dead_code))]
    fn DeleteObject(object: isize) -> i32;
    #[cfg_attr(not(test), allow(dead_code))]
    fn GdiFlush() -> i32;
    fn StretchDIBits(
        device_context: Hdc,
        destination_x: i32,
        destination_y: i32,
        destination_width: i32,
        destination_height: i32,
        source_x: i32,
        source_y: i32,
        source_width: i32,
        source_height: i32,
        bits: *const core::ffi::c_void,
        bitmap_info: *const BitmapInfo,
        usage: Uint,
        raster_operation: Dword,
    ) -> i32;
}

/// Copies a canvas to a device context at the origin.
pub(super) fn present(device_context: Hdc, canvas: &Canvas) {
    present_region(
        device_context,
        canvas,
        0,
        0,
        canvas.width(),
        canvas.height(),
    );
}

/// Copies one rectangle of a canvas to the matching place on a device context.
///
/// An animation that only changes part of the surface should only send that
/// part. The DIB describes a contiguous band beginning at the requested source
/// row, so its origin remains zero even for a partial update.
pub(super) fn present_region(
    device_context: Hdc,
    canvas: &Canvas,
    x: u32,
    y: u32,
    region_width: u32,
    region_height: u32,
) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let region_width = region_width.min(canvas.width().saturating_sub(x)) as i32;
    let region_height = region_height.min(canvas.height().saturating_sub(y)) as i32;
    if width <= 0 || height <= 0 || region_width <= 0 || region_height <= 0 {
        return;
    }
    let info = BitmapInfo::top_down(width, region_height);
    let row_start = (y as usize) * (canvas.width() as usize);
    let bits = canvas.pixels()[row_start..].as_ptr();
    // SAFETY: `bits` starts at a complete canvas row and the remaining slice
    // holds at least region_height rows at the declared width. Both it and
    // `info` outlive this synchronous call. The device context belongs to the
    // paint currently in progress.
    unsafe {
        StretchDIBits(
            device_context,
            x as i32,
            y as i32,
            region_width,
            region_height,
            x as i32,
            0,
            region_width,
            region_height,
            bits.cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BitmapInfo, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
        GdiFlush, SelectObject, present, present_region,
    };
    use anodrel_canvas::{Canvas, Color, Paint, Rect};

    /// Presents a canvas into a memory bitmap and returns what landed there.
    ///
    /// This exercises the real blit rather than a stand-in, so a wrong bitmap
    /// header or row order shows up as mismatched pixels instead of as a blank
    /// window discovered by hand.
    fn present_to_memory(canvas: &Canvas) -> Vec<u32> {
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        let info = BitmapInfo::top_down(width, height);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: every handle below is released before returning, and `info`
        // describes exactly the surface CreateDIBSection is asked for.
        unsafe {
            let device_context = CreateCompatibleDC(0);
            assert_ne!(device_context, 0, "memory device context");
            let bitmap = CreateDIBSection(device_context, &info, DIB_RGB_COLORS, &mut bits, 0, 0);
            assert_ne!(bitmap, 0, "destination bitmap");
            assert!(!bits.is_null());
            let previous = SelectObject(device_context, bitmap);

            present(device_context, canvas);
            GdiFlush();

            let count = (width as usize) * (height as usize);
            let copied = std::slice::from_raw_parts(bits.cast::<u32>(), count).to_vec();

            SelectObject(device_context, previous);
            DeleteObject(bitmap);
            DeleteDC(device_context);
            copied
        }
    }

    /// Presents one source rectangle into a cleared memory bitmap.
    fn present_region_to_memory(
        canvas: &Canvas,
        x: u32,
        y: u32,
        region_width: u32,
        region_height: u32,
    ) -> Vec<u32> {
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        let info = BitmapInfo::top_down(width, height);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: every handle below is released before returning, and `info`
        // describes exactly the surface CreateDIBSection is asked for.
        unsafe {
            let device_context = CreateCompatibleDC(0);
            assert_ne!(device_context, 0, "memory device context");
            let bitmap = CreateDIBSection(device_context, &info, DIB_RGB_COLORS, &mut bits, 0, 0);
            assert_ne!(bitmap, 0, "destination bitmap");
            assert!(!bits.is_null());
            let previous = SelectObject(device_context, bitmap);
            let count = (width as usize) * (height as usize);
            std::ptr::write_bytes(bits, 0, count * std::mem::size_of::<u32>());

            present_region(device_context, canvas, x, y, region_width, region_height);
            GdiFlush();

            let copied = std::slice::from_raw_parts(bits.cast::<u32>(), count).to_vec();

            SelectObject(device_context, previous);
            DeleteObject(bitmap);
            DeleteDC(device_context);
            copied
        }
    }

    #[test]
    fn a_presented_canvas_arrives_pixel_for_pixel() {
        let mut canvas = Canvas::new(16, 12);
        canvas.clear(Color::hex(0x102030));
        canvas.fill_rect(
            Rect::new(2.0, 3.0, 9.0, 7.0),
            &Paint::solid(Color::hex(0xA855F7)),
        );

        let presented = present_to_memory(&canvas);
        for y in 0..12 {
            for x in 0..16 {
                let expected = canvas.pixel(x, y);
                let actual = channel(presented[(y as usize) * 16 + (x as usize)]);
                assert_eq!(
                    (expected.red, expected.green, expected.blue),
                    actual,
                    "pixel ({x}, {y}) differs"
                );
            }
        }
    }

    /// Unpacks the colour channels of a presented pixel, ignoring alpha, which
    /// a `BI_RGB` surface does not carry.
    fn channel(packed: u32) -> (u8, u8, u8) {
        (
            ((packed >> 16) & 0xFF) as u8,
            ((packed >> 8) & 0xFF) as u8,
            (packed & 0xFF) as u8,
        )
    }

    #[test]
    fn the_top_left_pixel_is_not_flipped_to_the_bottom() {
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(Rect::new(0.0, 0.0, 8.0, 1.0), &Paint::solid(Color::WHITE));

        let presented = present_to_memory(&canvas);
        assert_eq!(
            channel(presented[0]),
            (255, 255, 255),
            "top row should be white"
        );
        assert_eq!(
            channel(presented[7 * 8]),
            (0, 0, 0),
            "bottom row should be black; a flipped blit would invert these"
        );
    }

    #[test]
    fn a_partial_present_keeps_the_source_and_destination_regions_aligned() {
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(Color::BLACK);
        for row in 0..8 {
            canvas.fill_rect(
                Rect::new(0.0, row as f32, 8.0, row as f32 + 1.0),
                &Paint::solid(Color::hex((row as u32) << 16)),
            );
        }

        let presented = present_region_to_memory(&canvas, 0, 3, 8, 3);
        for row in 3..6 {
            assert_eq!(
                channel(presented[row * 8]),
                channel(canvas.pixels()[row * 8]),
                "row {row} was copied from the wrong source position"
            );
        }
        assert_eq!(channel(presented[2 * 8]), (0, 0, 0));
        assert_eq!(channel(presented[6 * 8]), (0, 0, 0));
    }

    #[test]
    fn an_empty_canvas_never_reaches_gdi() {
        // Zero-sized client areas occur while a window is being minimised.
        // Passing a null device context proves the early return is taken: if it
        // were not, the blit would fault.
        present(0, &Canvas::new(0, 0));
    }
}
