# Anodex title-bar adapter

This package is the first deliberate bridge between Anodrel and Anodex. It
uses only Anodrel's public protocol types to map a session-owned window-state
snapshot and optional coalesced refresh to a maximize or restore title-bar
action. It also maps a title-bar close intent to the current session's existing
close request and a minimize click to the existing closed window-state request.

It does not import Electron, host a renderer, copy Anodex source, expose a
native window object, or implement native close behaviour. A close response
proves only that the host accepted the session-end request; it does not prove a
native window or product process has ended. Its consumer chooses when to make
an explicit coalesced refresh, and the package never creates polling or a
state-change callback.

See `docs/ANODEX_ADAPTER.md` and `docs/WINDOW_STATE_OBSERVATION.md` at the
repository root for the supported migration boundary and its limits. Decision
0119 records the title-bar close mapping.
