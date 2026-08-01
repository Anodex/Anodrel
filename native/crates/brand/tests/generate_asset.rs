//! One-off brand asset generation.
//!
//! This is not part of the test suite; it is the recorded, repeatable step that
//! turns the authored logo into the committed asset. Keeping it as an ignored
//! test means the conversion is code that compiles and is reviewed, rather than
//! a command someone once ran. See `crates/brand/assets/README.md`.

use anodrel_canvas::Image;

/// Edge of the authored source, in pixels.
const SOURCE_SIDE: u32 = 1024;
/// Edge of the committed asset, in pixels.
const ASSET_SIDE: u32 = 512;
/// Alpha above which a pixel counts as artwork rather than as padding.
const ARTWORK_ALPHA: u8 = 3;

#[test]
#[ignore = "asset generation; run explicitly with ANODREL_MARK_SOURCE set"]
fn generate_mark_asset() {
    let source_path = std::env::var("ANODREL_MARK_SOURCE")
        .expect("set ANODREL_MARK_SOURCE to the raw BGRA export of the authored logo");
    let out_dir = std::env::var("ANODREL_ASSET_OUT").expect("set ANODREL_ASSET_OUT");

    let bytes = std::fs::read(&source_path).expect("source readable");
    let source =
        Image::from_bgra_bytes(SOURCE_SIDE, SOURCE_SIDE, &bytes).expect("source is 1024x1024 BGRA");

    let (left, top, right, bottom) = source
        .opaque_bounds(ARTWORK_ALPHA)
        .expect("the source contains artwork");
    let (width, height) = (right - left, bottom - top);
    println!("artwork bbox: ({left},{top})-({right},{bottom}) = {width}x{height}");

    // Square the crop about the artwork's centre. The asset then has the same
    // 1:1 placement semantics as the unit-square geometry, so the raster and
    // the vector fallback occupy identical bounds and cannot jump when the
    // renderer switches between them.
    let side = width.max(height);
    let crop_left = (left + width / 2).saturating_sub(side / 2);
    let crop_top = (top + height / 2).saturating_sub(side / 2);
    let squared = source.cropped(crop_left, crop_top, side, side);
    println!("squared crop: ({crop_left},{crop_top}) side {side}");

    let resized = squared.resized(ASSET_SIDE, ASSET_SIDE);
    let out = resized.to_bgra_bytes();
    let path = format!("{out_dir}/mark-{ASSET_SIDE}.bgra");
    std::fs::write(&path, &out).expect("asset written");
    println!("wrote {path}: {} bytes", out.len());
}
