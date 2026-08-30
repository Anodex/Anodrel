# Decision 0129: Linux Lab pointer input stays local and semantic

- Status: Accepted
- Date: 2026-08-30

## Context

Decision 0128 proves that one Anodrel canvas can reach a Wayland compositor,
but it deliberately has no input path. A visible diagnostic cannot demonstrate
the native-host boundary in both directions until the host can consume one
ordinary desktop interaction.

Giving applications a Wayland object, pointer position, button identity,
timestamp, callback, or general input stream would prematurely make desktop
input a public capability. It would also bypass the existing portable UI and
revision-checked action boundaries. A toolkit, libwayland client runtime, or
input library would contradict Decision 0005.

## Decision

The development-only Anodrel Linux Lab binds at most one version-1 `wl_seat`
and requests one version-1 `wl_pointer` only after that seat has announced its
pointer capability. It accepts the narrow `enter`, `leave`, `motion`, `button`,
and `axis` event set for its own fixed surface, with all values parsed and
retained only inside the native adapter.

The adapter recognises one fixed left-button press-and-release within the
lab's compiled activation region. It exposes only the closed outcome
`Activated` to the diagnostic. It never exposes a coordinate, serial,
timestamp, scroll delta, button value, seat, pointer, native handle, callback,
or input-state query. The diagnostic uses that outcome once to present its
already-owned completion canvas; it does not call application code or compose
with Anodrel IPC.

The lab remains presentable on a compositor without a pointer-capable seat.
Such a desktop has no activation probe rather than becoming an incompatible
windowing environment. If a pointer capability disappears after binding, the
lab stops receiving interaction and continues to obey compositor close.

## Consequences

- The Linux Lab proves a small, auditable local input path without broadening
  the application protocol or adding a general Linux window API.
- Pure tests can cover fixed-point decoding, surface ownership, hit-testing,
  press/release ordering, unsupported button states, and malformed events
  without a compositor or physical input device.
- The visual manual check becomes: open the lab, click the highlighted lower
  panel once, observe its one-time completed appearance, then close the window.
- One extra seat and pointer object is allocated only when the compositor
  advertises the capability; no polling, timer, cursor surface, frame loop, or
  allocation per pointer event is added.

## Deliberately absent

- keyboard, touch, gestures, relative motion, pointer capture, cursor images,
  scroll behavior, drag-and-drop, clipboard, selection, or raw input;
- application-controlled hit regions, input documents, event payloads,
  callbacks, subscriptions, IPC delivery, focus control, or input readback;
- display scaling, resizing, multiple windows, accessibility, text entry, or
  an application desktop host.

## Alternatives considered

**Expose raw pointer events to an application.** This would make native desktop
data a new public protocol surface before a Linux session host exists. Refused.

**Wait for keyboard input instead.** Keyboard handling requires a keymap and
text/input-method policy. Pointer activation is the smaller first input seam.
Deferred.

**Use libwayland-client or a toolkit.** Both add an unowned shipped runtime.
Refused under Decision 0005.

## Revisit conditions

Revisit before adding application-controlled Linux interaction, a public input
capability, keyboard or text entry, accessibility input, more than one local
diagnostic action, or a product Linux host.
