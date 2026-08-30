# Decision 0133: First-party font faces parse owned memory

- Status: Accepted
- Date: 2026-08-30

## Context

The portable canvas composites coverage masks but intentionally does not know
how text is shaped or rasterized. The Windows host temporarily fills that seam
through GDI. Linux has a direct Wayland canvas path but cannot present general
Anodrel text without either a native text stack or a first-party font path.

Shipping a toolkit, browser runtime, FreeType, HarfBuzz, or a system-font
wrapper would add an unowned runtime to the platform boundary. A tiny fixed
bitmap alphabet would avoid that dependency but would not support the bounded
Unicode text the portable UI already accepts.

## Decision

Add the portable `anodrel-font` crate. Its first slice parses one caller-owned
in-memory TrueType SFNT face, validates its `cmap` table, and maps one Unicode
scalar to a nonzero glyph ID. It supports only the standard Unicode format-4
and format-12 map records chosen by the documented fixed priority. It borrows
the input bytes rather than copying or loading them, and lookup is a bounded
binary search without allocation or operating-system work.

The crate is not a font source. It does not accept paths, enumerate fonts,
load a default family, choose fallback, expose a font through the protocol, or
allow an application to nominate font data. A later host-owned face source and
outline/raster path must have their own contracts before Linux text appears.

## Consequences

- Anodrel gains an owned, testable Unicode character-map foundation shared by
  native hosts without taking a graphics or toolkit dependency.
- The parser has a narrow malicious-input boundary, so every offset, table
  range, ordering rule, and glyph calculation is checked before it is used.
- Future shaping and fallback cannot be implied by a successful character-map
  lookup; their absence remains visible in the API and documentation.
- The initial parser stays fast enough for retained face use: it scans once at
  construction and performs no heap work per glyph lookup.

## Deliberately absent

- font files, collections, CFF, variable fonts, system discovery, packaging,
  installation, and a default Anodrel typeface;
- glyph outlines, hinting, rasterization, text measurement, or canvas calls;
- shaping, kerning, ligatures, scripts, bidirectional text, line layout, and
  fallback;
- a Linux application host, public font API, or application-selected font.

## Alternatives considered

**Use FreeType and HarfBuzz.** Mature and feature-rich, but they would be
shipped third-party runtimes at the core native-text boundary. Refused under
Decision 0005.

**Use a fixed ASCII bitmap font.** Small but incompatible with Anodrel's
Unicode text model and incapable of becoming a general application host.
Refused.

**Ask each native host to use its system text API indefinitely.** This would
make rendering quality and behavior platform-specific before a portable
baseline exists. Deferred only as the existing Windows bridge.

## Revisit conditions

Revisit before adding an owned face source, another SFNT flavour, outlines,
rasterization, shaping, fallback, a public font operation, or a Linux
application text surface.
