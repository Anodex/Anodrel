# Windows UI document preview

**Status:** Development diagnostic. The direct Windows host can render one
operator-selected `anodrel.ui.document.v1` file through the native UI pipeline.
It is not an application package format, protocol operation, session, or
capability.

## Command

From the repository root on Windows:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-preview path-to-document.json
~~~

The host opens one **Anodrel UI Preview** window. Hovering, Tab/Shift+Tab, and
Enter use the same native layout, hit test, and focus code as UI Lab. A preview
action remains local diagnostic state: it does not launch a process, read
another file, send a protocol message, create an application session, or grant
a capability.

## Input boundary

The argument names exactly one regular file selected by the local operator. The
host opens it once, reads no more than **64 KiB** of UTF-8, and validates the
complete contents with `anodrel-ui-document` before creating a window. It does
not follow a manifest, resolve a package, load an asset, execute code, fetch a
resource, read a second path, or retain the source file after startup.

The exact `docs/UI_DOCUMENTS.md` schema and all its document limits apply. A
malformed, oversized, unsupported, or invalid document fails before any native
window opens. Safe errors identify only the validation category, not document
content.

## Boundary and compatibility

This command proves the external document decoder and renderer together under
a deliberate developer-controlled path. It does not make UI documents trusted
application content and must not be used as the future package or session
loading path. A package or authenticated client still needs its own application
identity, lifecycle, queue, update, event-delivery, and permission contract.

The preview accepts only v1. Unknown fields and later versions fail closed.
Adding an input feature requires a new documented format version and tests.

## Verification

The host tests bounded regular-file loading, rejects oversized input before
parsing, validates the document before window construction, and headlessly
renders the same UI view state used by the preview. Manual verification is to
open the compiled UI Lab fixture or another valid v1 JSON file, use pointer and
keyboard focus, and close the window normally.
