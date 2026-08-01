# Decision 0015: The brand mark is the authored asset, not a reconstruction

**Status:** Accepted. Supersedes the asset reasoning in Decision 0013.

**Date:** 2026-07-31

## Context

Decision 0013 chose to carry the Anodrel mark as geometry — normalised polygon
coordinates in `anodrel-brand` — and rejected shipping the authored artwork.
The reasons it gave were:

1. an image asset needs a decoder the project does not own;
2. it fixes the artwork at one resolution;
3. it cannot be asserted on in a test.

The first and third are wrong, and the record should say so plainly.

An asset does not need a runtime decoder if it is committed **pre-decoded**:
raw pixels plus a length check is the entire loading path. And an asset can be
asserted on — that it is a clean cut-out, that its artwork reaches the edges of
its crop, that its legs carry the brand gradient. Those are exactly the checks
now in place.

Only the second reason survives, and it is a bounded one: a raster has a
maximum useful size, and reduces poorly past a point.

The deeper problem was not the reasoning but what it produced. The geometry was
derived by eye from the authored logo: the notch width, the chamfer depths, and
the foot chamfers were all estimated. The result is close, and it is not the
mark. For a logo, "close" is the whole failure — a brand mark is an exact
artefact, and a platform that renders an approximation of its own identity is
misrepresenting itself in the one place users look first.

## Decision

The authored artwork is the mark.

`anodrel-brand` commits `assets/mark-512.bgra`: the authored logo, cropped
square to its artwork, reduced to 512 × 512 with a premultiplied box filter, and
stored as raw straight-alpha `B, G, R, A`. It is embedded with `include_bytes!`.
No decoder ships, and no dependency is added.

The geometry stays, in a defined role. Below `RASTER_MIN_EDGE` (64 px) the
raster has to be reduced so far that its chamfers smear, while geometry stays
crisp because it is rasterized at the size actually requested. So:

- **at or above 64 px** — the authored raster, with the glow taken from the
  artwork's own alpha channel;
- **below 64 px** — the geometry, which is what the small window icon and the
  document header use.

Both occupy identical bounds. The asset is cropped square about its artwork's
centre and the geometry fills the unit square, so nothing shifts when the
renderer crosses the threshold; a test asserts that the painted extent agrees
across the boundary.

The conversion lives in `crates/brand/tests/generate_asset.rs` as an ignored
test rather than a shell script, so the step that produces a committed binary is
code that compiles and gets reviewed. `assets/README.md` records provenance,
format, and the constraint that the source must stay a clean cut-out — the glow
is applied at draw time, so a re-export with a baked halo would double it.

## Consequences

Positive:

- the platform displays its own logo rather than a lookalike;
- still no decoder and no third-party dependency;
- the asset is testable, and the tests cover the properties that actually
  matter: transparency, crop, and brand colour;
- geometry keeps a real job at small sizes, where it genuinely beats a raster.

Tradeoffs:

- 1 MB of committed binary, which is most of the repository's weight;
- the mark has a maximum useful size of 512 px; a larger hero would need a
  larger asset, and the constant and asset must be changed together;
- the artwork can no longer be recoloured or re-themed programmatically at
  hero size, only at the geometry sizes;
- two rendering paths for one mark, which the shared-bounds test exists to keep
  honest.

## Revisit conditions

Revisit if a surface needs the mark above 512 px, if the identity needs runtime
theming, or if the committed size becomes a problem — at which point owning a
small inflate decoder and committing the compressed original is the next step,
and is well within what this project already writes by hand.
