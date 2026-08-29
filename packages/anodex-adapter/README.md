# Anodex title-bar adapter

This package is the first deliberate bridge between Anodrel and Anodex. It
uses only Anodrel's public protocol types to map a session-owned window-state
snapshot to a maximize or restore title-bar action.

It does not import Electron, host a renderer, copy Anodex source, expose a
native window object, or implement application close behaviour. Its consumer
must refresh the snapshot after requesting a transition because Anodrel
intentionally provides no state-change event.

See `docs/ANODEX_ADAPTER.md` and `docs/WINDOW_STATE_OBSERVATION.md` at the
repository root for the supported migration boundary and its limits.
