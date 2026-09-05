//! Private GDI source for the one fixed Windows surface typeface.
//!
//! The source is intentionally separate from the GDI painter. It lets an owned
//! future text path consume the bytes of the exact host-selected face without
//! accepting a font path, directory, or application-provided font value.

use std::{ffi::c_void, ptr};

use super::{Bool, Dword, Hdc};

type Hgdiobj = isize;

const DEFAULT_CHARSET: Dword = 1;
const OUT_TT_PRECIS: Dword = 4;
const CLIP_DEFAULT_PRECIS: Dword = 0;
const ANTIALIASED_QUALITY: Dword = 4;
const DEFAULT_PITCH: Dword = 0;
const GDI_ERROR: Dword = Dword::MAX;
const HGDI_ERROR: Hgdiobj = -1;
const SURFACE_FONT_SIZE: i32 = 16;
const SURFACE_FONT_WEIGHT: i32 = 400;

/// The fixed typeface selected by every first-party Windows surface.
pub(super) const SURFACE_FACE_NAME: &str = "Segoe UI";
/// Maximum private byte size accepted from Windows for one selected face.
pub(super) const MAX_SELECTED_FACE_BYTES: usize = 8 * 1024 * 1024;

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(device_context: Hdc) -> Hdc;
    fn DeleteDC(device_context: Hdc) -> Bool;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: Dword,
        underline: Dword,
        strike_out: Dword,
        char_set: Dword,
        output_precision: Dword,
        clip_precision: Dword,
        quality: Dword,
        pitch_and_family: Dword,
        face_name: *const u16,
    ) -> Hgdiobj;
    fn SelectObject(device_context: Hdc, object: Hgdiobj) -> Hgdiobj;
    fn DeleteObject(object: Hgdiobj) -> Bool;
    fn GetFontData(
        device_context: Hdc,
        table: Dword,
        offset: Dword,
        buffer: *mut c_void,
        length: Dword,
    ) -> Dword;
}

/// One private memory DC with one selected font, released even during unwind.
struct SelectedSurfaceFont {
    device_context: Hdc,
    font: Hgdiobj,
    previous_font: Hgdiobj,
}

impl SelectedSurfaceFont {
    /// Creates a memory-only DC with the fixed surface typeface selected.
    fn create(size: i32, weight: i32) -> Option<Self> {
        let face = wide_null(SURFACE_FACE_NAME);
        // SAFETY: the created handles are either moved into the RAII owner or
        // released on the failed creation path below.
        unsafe {
            let device_context = CreateCompatibleDC(0);
            if device_context == 0 {
                return None;
            }
            let font = CreateFontW(
                -size.max(1),
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                DEFAULT_PITCH,
                face.as_ptr(),
            );
            if font == 0 {
                DeleteDC(device_context);
                return None;
            }
            let previous_font = SelectObject(device_context, font);
            if previous_font == 0 || previous_font == HGDI_ERROR {
                DeleteObject(font);
                DeleteDC(device_context);
                return None;
            }
            Some(Self {
                device_context,
                font,
                previous_font,
            })
        }
    }
}

impl Drop for SelectedSurfaceFont {
    fn drop(&mut self) {
        // SAFETY: the owner created these handles and the selected object is
        // restored before deletion, including when the caller unwinds.
        unsafe {
            SelectObject(self.device_context, self.previous_font);
            DeleteObject(self.font);
            DeleteDC(self.device_context);
        }
    }
}

/// Runs `work` with the fixed surface font selected into a private memory DC.
///
/// The DC and font selection are restored and released on every outcome.
pub(super) fn with_surface_font<R>(
    size: i32,
    weight: i32,
    work: impl FnOnce(Hdc) -> Option<R>,
) -> Option<R> {
    let selected = SelectedSurfaceFont::create(size, weight)?;
    work(selected.device_context)
}

/// Reads one bounded, process-private copy of the fixed regular surface face.
///
/// The return value is not persisted, exposed through the protocol, or used by
/// the current GDI painter. A later owned text path must explicitly consume it.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn selected_face_data() -> Option<Vec<u8>> {
    with_surface_font(SURFACE_FONT_SIZE, SURFACE_FONT_WEIGHT, |device_context| {
        // SAFETY: the memory DC owns a selected TrueType surface font for the
        // duration of the closure. The first call asks only for a byte count;
        // the second writes exactly into the owned vector's initialized range.
        unsafe {
            let reported = GetFontData(device_context, 0, 0, ptr::null_mut(), 0);
            let length = selected_face_length(reported)?;
            let mut data = vec![0; length];
            let copied = GetFontData(
                device_context,
                0,
                0,
                data.as_mut_ptr().cast::<c_void>(),
                reported,
            );
            (copied == reported).then_some(data)
        }
    })
}

/// Converts one GDI-reported source length into an accepted allocation length.
fn selected_face_length(reported: Dword) -> Option<usize> {
    if reported == 0 || reported == GDI_ERROR {
        return None;
    }
    let length = usize::try_from(reported).ok()?;
    (length <= MAX_SELECTED_FACE_BYTES).then_some(length)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Dword, GDI_ERROR, MAX_SELECTED_FACE_BYTES, selected_face_data, selected_face_length,
        with_surface_font,
    };
    use anodrel_font::FontFace;

    #[test]
    fn selected_surface_face_is_bounded_and_accepted_by_the_owned_parser() {
        let data =
            selected_face_data().expect("fixed Windows surface face is available as TrueType");
        assert!(!data.is_empty());
        assert!(data.len() <= MAX_SELECTED_FACE_BYTES);

        let face = FontFace::parse(&data).expect("selected face is accepted by anodrel-font");
        let glyph = face.glyph_id('A').expect("selected face maps Latin A");
        assert!(face.horizontal_metric(glyph).is_ok());
        assert!(face.glyph_outline(glyph).is_ok());
    }

    #[test]
    fn selected_face_length_rejects_failed_empty_and_oversized_reports() {
        assert_eq!(selected_face_length(GDI_ERROR), None);
        assert_eq!(selected_face_length(0), None);
        assert_eq!(
            selected_face_length(MAX_SELECTED_FACE_BYTES as Dword),
            Some(MAX_SELECTED_FACE_BYTES)
        );
        assert_eq!(
            selected_face_length(MAX_SELECTED_FACE_BYTES as Dword + 1),
            None
        );
    }

    #[test]
    fn selected_font_lifetime_survives_a_callback_unwind() {
        let result = std::panic::catch_unwind(|| {
            let _ = with_surface_font(16, 400, |_| -> Option<()> { panic!("test unwind") });
        });
        assert!(result.is_err());
        assert!(selected_face_data().is_some());
    }
}
