# Decision 0105: Development native window-controls template keeps session controls targetless

**Status:** Accepted

**Date:** 2026-08-24

## Context

The stable in-repository `anodrel-windows-ui-sdk` exposes five existing,
targetless controls for the authenticated session's own host window: a title
proposal, a closed presentation state, a foreground request, reversible
fullscreen, and a bounded logical client size. Unit and protocol tests prove
each method, but none of the six generated native projects exercises all five
through a real invited session.

Giving those grants to the ordinary native template would silently turn its
three-grant document example into a window-control sample. Allowing a generic
template to accept a capability list, window identity, native handle, geometry,
monitor, title readback, focus state, or host command would undermine the
fixed-grant development boundary established by Decisions 0082 through 0103.

## Decision

Add a separately selected development template and host route:

- `anodrel-native-app-tool init-window-controls <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-window-controls-template-client <client.exe>`.

The host creates one fresh session with exactly these fixed grants:

- `ui.document.write`;
- `ui.events.read`;
- `window.title`;
- `window.state`;
- `window.focus`;
- `window.fullscreen`;
- `window.size`; and
- `session.close`.

The generated program has one fixed, person-driven walkthrough. Each
revision-bound semantic action invokes one existing typed SDK control, then
publishes the next fixed document. The walkthrough proposes a composed title,
requests a bounded client size, maximises and restores, requests foreground
attention, enters and leaves fullscreen, then closes its own session. It never
receives native state, geometry, focus, monitor, title, or delivery readback;
all accepted results remain acknowledgements only.

The generated source imports only `anodrel-windows-ui-sdk`. It has no raw
protocol operation, capability input, configuration or document input, native
handle, window target, identity, package, signing, installation, network, menu,
or process-launch surface. The host owns the window, composed caption,
mailboxes, User32 calls, fullscreen restoration facts, process lifetime,
invitation, pipe worker, and cleanup.

## Consequences

- The stable SDK's complete targetless window-control surface gains generated
  project and real invited-session compatibility coverage without adding a
  protocol operation or runtime dependency.
- A developer can visibly exercise the existing controls from a first-party
  Rust executable while their actual operating-system effects remain the
  host's responsibility.
- The title, state, focus, fullscreen, and size contracts remain separate:
  the generated example does not merge their grants, change their wire shapes,
  or provide one combined window-management API.
- Manual Windows verification remains necessary. In particular, accepted
  foreground requests do not prove that Windows granted foreground focus, and
  accepted controls do not provide native readback to the child.

## Revisit conditions

Revisit before adding an application-selected title, size, state sequence,
document, capability, window target, geometry or focus readback, monitor,
keyboard shortcut, menu, background event delivery, product launch, packaging,
signing, installation, another operating-system adapter, or a stable published
SDK line. Each would expand the generated application's authority or the
platform's public compatibility surface.
