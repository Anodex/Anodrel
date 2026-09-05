# Decision 0211: Windows selected-font data stays private and bounded

- Status: Accepted
- Date: 2026-09-05

## Context

Anodrel now owns the portable parsing, metrics, conventional pair kerning,
outline, flattening, coverage, and single-line run layers needed by a software
text path. It deliberately does not own a font file, enumerate system fonts, or
allow an application to select a face. The current Windows host still asks GDI
to shape and rasterize each surface run.

Replacing that painter without first proving that the host can supply its own
portable parser with the same host-selected face would either add an unowned
font runtime or quietly make the renderer depend on a filesystem font path.
Neither is a suitable boundary for the Windows reference host.

## Decision

Add one private Windows-host source that selects the fixed surface face in a
memory GDI device context and reads its selected TrueType data through
`GetFontData`. It first queries the byte count, rejects an empty, failed, or
larger-than-8-MiB result before allocating, then accepts only an exact second
read into process-private memory. Any failed source operation is a closed
unavailable result.

The source has no public protocol operation, file path, directory scan, font
enumeration, fallback, application-selected family, persistence, package
embedding, or cross-process delivery. It keeps the bytes only for an owned
host consumer. A host test passes the live selected data into `anodrel-font` so
the source and the portable parser are proved together on Windows.

The visible painter remains the existing GDI shaping and hinting route. This
decision establishes only the source seam; it does not imply that the owned
unshaped renderer has matching quality, script coverage, font fallback, or
production readiness.

## Consequences

- The next owned Windows text slice can use the same fixed host face without a
  third-party runtime or a hidden filesystem dependency.
- The direct GDI call is isolated to the Windows host while parsing stays in the
  portable, unsafe-free crate.
- A font that Windows cannot expose as selected TrueType data simply remains on
  the GDI route; Anodrel does not reinterpret or substitute the bytes.
- The 8 MiB maximum makes the future source's first allocation explicit and
  bounded before any font table parser sees the data.

## Deliberately absent

- ownership, redistribution, embedding, installation, or a bundled Anodrel
  typeface;
- system-font discovery, a user preference, fallback, a CSS-like family list,
  a public font API, or application-supplied font bytes;
- changing the current GDI painter, shaping, layout, hinting, or accessibility
  surface;
- Linux/macOS source adapters, a protocol change, or an application capability.

## Alternatives considered

**Read a known Windows font file.** This couples the host to a machine path and
font-installation layout rather than to the face that Windows selected. Refused.

**Ship a font or use a third-party text stack.** Either broadens the shipped
runtime and typeface ownership boundary. Refused under Decision 0005.

**Replace GDI immediately.** The current first-party run is deliberately
unshaped and would reduce text quality for real surfaces. Deferred until owned
rendering can be measured against the existing route.

## Revisit conditions

Revisit before retaining a live parsed face, selecting a different host face,
using the data in the painter, adding fallback or shaping, persisting font bytes,
or adding a non-Windows adapter.
