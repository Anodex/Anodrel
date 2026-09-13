//! The fixed Anodrel application icon for native shell surfaces.
//!
//! This module renders the same rounded brand plate used by the Windows host
//! and encodes it into the small, decoder-free Windows ICO container needed by
//! the Start menu. It has no path, product, operating-system, or application
//! input.

use anodrel_canvas::{Canvas, Paint, Rect, Stop, point};

use crate::{mark, mark::MarkStyle, palette};

/// The fixed filename every Windows shell registration uses for the mark.
pub const WINDOWS_FILE_NAME: &str = "Anodrel.ico";

const SIZES: [u32; 4] = [16, 32, 48, 256];

/// Renders the shared platform icon at one supported pixel size.
#[must_use]
pub fn render(size: u32) -> Canvas {
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

/// Encodes the fixed multi-size Windows ICO file from the rendered mark.
///
/// Every entry is a 32-bit DIB with alpha and an all-clear compatibility mask.
/// The container has no decoder, metadata, product input, or external asset.
#[must_use]
pub fn windows_ico() -> Vec<u8> {
    let images = SIZES.into_iter().map(dib).collect::<Vec<_>>();
    let directory_bytes = 6 + images.len() * 16;
    let total_bytes = directory_bytes + images.iter().map(Vec::len).sum::<usize>();
    let mut output = Vec::with_capacity(total_bytes);
    write_u16(&mut output, 0);
    write_u16(&mut output, 1);
    write_u16(&mut output, images.len() as u16);
    let mut offset = directory_bytes as u32;
    for (size, image) in SIZES.into_iter().zip(&images) {
        output.push((size % 256) as u8);
        output.push((size % 256) as u8);
        output.extend_from_slice(&[0, 0]);
        write_u16(&mut output, 1);
        write_u16(&mut output, 32);
        write_u32(&mut output, image.len() as u32);
        write_u32(&mut output, offset);
        offset += image.len() as u32;
    }
    for image in images {
        output.extend_from_slice(&image);
    }
    output
}

fn dib(size: u32) -> Vec<u8> {
    let canvas = render(size);
    let pixel_bytes = (size * size * 4) as usize;
    let mask_stride = size.div_ceil(32) * 4;
    let mut output = Vec::with_capacity(40 + pixel_bytes + (mask_stride * size) as usize);
    write_u32(&mut output, 40);
    write_u32(&mut output, size);
    write_u32(&mut output, size * 2);
    write_u16(&mut output, 1);
    write_u16(&mut output, 32);
    write_u32(&mut output, 0);
    write_u32(&mut output, pixel_bytes as u32);
    output.extend_from_slice(&[0; 16]);
    for row in (0..size as usize).rev() {
        for pixel in &canvas.pixels()[row * size as usize..(row + 1) * size as usize] {
            output.extend_from_slice(&pixel.to_le_bytes());
        }
    }
    output.resize(output.len() + (mask_stride * size) as usize, 0);
    output
}

fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::{SIZES, render, windows_ico};
    use crate::palette;

    #[test]
    fn icon_plate_and_mark_share_the_window_icon_appearance() {
        for size in SIZES {
            let canvas = render(size);
            assert!(canvas.pixel(0, 0).alpha < 255);
            let center = (size / 2) as i32;
            assert_eq!(canvas.pixel(center, center).alpha, 255);
        }
        assert_ne!(render(64).pixel(32, 40), palette::BACKDROP);
    }

    #[test]
    fn ico_carries_every_fixed_32_bit_dib_at_a_bounded_offset() {
        let ico = windows_ico();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, SIZES.len() as u8, 0]);
        let mut previous_end = 6 + SIZES.len() * 16;
        for (entry, size) in ico[6..6 + SIZES.len() * 16].chunks_exact(16).zip(SIZES) {
            assert_eq!(&entry[..4], &[(size % 256) as u8, (size % 256) as u8, 0, 0]);
            assert_eq!(u16::from_le_bytes(entry[4..6].try_into().unwrap()), 1);
            assert_eq!(u16::from_le_bytes(entry[6..8].try_into().unwrap()), 32);
            let image_bytes = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let mask_bytes = (size.div_ceil(32) * 4 * size) as usize;
            assert_eq!(image_bytes, 40 + (size * size * 4) as usize + mask_bytes);
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            assert_eq!(offset, previous_end);
            assert_eq!(
                u32::from_le_bytes(ico[offset..offset + 4].try_into().unwrap()),
                40
            );
            assert_eq!(
                u32::from_le_bytes(ico[offset + 4..offset + 8].try_into().unwrap()),
                size
            );
            assert_eq!(
                u32::from_le_bytes(ico[offset + 8..offset + 12].try_into().unwrap()),
                size * 2
            );
            assert_eq!(
                u16::from_le_bytes(ico[offset + 12..offset + 14].try_into().unwrap()),
                1
            );
            assert_eq!(
                u16::from_le_bytes(ico[offset + 14..offset + 16].try_into().unwrap()),
                32
            );
            assert_eq!(
                u32::from_le_bytes(ico[offset + 20..offset + 24].try_into().unwrap()),
                (size * size * 4)
            );
            previous_end += image_bytes;
        }
        assert_eq!(previous_end, ico.len());
    }
}
