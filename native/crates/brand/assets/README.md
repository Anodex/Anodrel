# Brand assets

## `mark-512.bgra`

The Anodrel mark, as authored. This is the brand artwork itself, not a
reconstruction of it.

| | |
| --- | --- |
| Format | Raw straight-alpha `B, G, R, A`, row-major, no header |
| Size | 512 × 512 |
| Bytes | 1,048,576 |

### Why raw pixels

Anodrel ships no third-party runtime dependency (Decision 0005), so it has no
image decoder and should not grow one just to display its own logo. Storing the
artwork pre-decoded removes the question entirely: `include_bytes!` and a
length check is the whole loading path.

The byte order matches a 32-bit Windows bitmap, so the asset also needs no
conversion pass on the platform it is used on today.

### Provenance

Cropped from a 1024 × 1024 authored export with a transparent background. The
crop is squared about the artwork's centre, so the asset has the same 1:1
placement semantics as the unit-square geometry in `src/mark.rs`; a raster and
the vector fallback therefore occupy identical bounds and cannot shift when the
renderer switches between them at [`RASTER_MIN_EDGE`].

The source has no glow or shadow baked into it. The glow is applied at draw
time from the artwork's own alpha channel, which is why the asset must stay a
clean cut-out — re-exporting it with a halo would double the bloom.

### Regenerating

`tests/generate_asset.rs` performs the conversion. It is an ignored test rather
than a script so the step is code that compiles and gets reviewed.

Export the authored artwork to raw 1024 × 1024 BGRA, then:

~~~text
ANODREL_MARK_SOURCE=<path to raw bgra> \
ANODREL_ASSET_OUT=<path to this directory> \
cargo test --release -p anodrel-brand --test generate_asset -- --ignored --nocapture
~~~

Run it in release: the reduction is a full box filter over a million pixels and
is slow in an unoptimised build.

The reduction is performed in premultiplied space. Averaging straight alpha
would let transparent pixels bleed toward black and leave a dark fringe on
every edge of the cut-out.

### If the asset changes

`RASTER_SIDE` in `src/mark.rs` must match the new edge length; loading fails
closed if it does not. Check the mark still renders at the hero size and in the
window icon, and re-read `docs/RENDERER.md` if the format changes.
