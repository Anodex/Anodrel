# Decision 0128: Linux desktop lab uses direct Wayland shared-memory presentation

- Status: Accepted
- Date: 2026-08-30

## Context

Linux now has private local transport, a child invitation, a development
launcher, paths, state storage, and crash records. None of those components
creates a visible Linux surface. The portable Anodrel canvas and brand crates
already compose 32-bit pixel buffers without an operating-system dependency,
so the missing seam is presenting one of those buffers to a Linux compositor.

Adding a GUI framework, a browser runtime, an X11 compatibility layer, or the
wayland-client library would contradict Decision 0005. Reimplementing
application UI, text layout, input, accessibility, and desktop services before
one first-party buffer reaches a compositor would make the first Linux host too
large to audit or verify.

## Decision

The first Linux desktop surface is a development-only **Anodrel Linux Lab**
implemented by anodrel-linux-wayland and anodrel-linux-window-lab. It talks
directly to the standard Wayland wire protocol and stable xdg-shell desktop
role. It is a Linux desktop-protocol adapter, not a third-party runtime
dependency.

The lab:

- reads only XDG_RUNTIME_DIR and WAYLAND_DISPLAY from its inherited desktop
  session; both are treated as untrusted locator input and never appear in
  public errors or debug output;
- accepts a non-empty, single-component display name below the runtime
  directory; it does not interpret an absolute path, a parent component, a
  command, or a connection selected by application input;
- discovers globals through wl_registry, finishes the initial announcement
  burst with wl_display.sync, and binds only wl_compositor, wl_shm, and
  xdg_wm_base at versions the adapter supports;
- creates one fixed-size xdg_toplevel role, answers compositor pings,
  acknowledges every xdg_surface.configure, and obeys a compositor close
  request;
- requires the compositor to advertise wl_shm XRGB8888 before it maps a
  buffer;
- creates exactly two host-owned, close-on-exec Linux memfd mappings, copies a
  same-sized Anodrel canvas into one available mapping, commits it through
  wl_surface, and reuses that mapping only after the matching wl_buffer.release;
- presents one fixed first-party branded diagnostic canvas. It has no supplied
  title, application identifier, document, input handler, native handle,
  pointer location, display geometry, callback, or public window capability.

The adapter speaks only the small message set required for that lifecycle. Its
manual wire codec performs object-ID allocation, 4-byte message alignment,
bounded string and array parsing, compositor error handling, descriptor
passing for shared memory, and fail-closed rejection of unexpected inbound
descriptors. It does not link to libwayland or generate bindings at build time.

The first lab is little-endian Linux only because the portable canvas's
0xAARRGGBB values map directly to the required XRGB8888 bytes on that
architecture. It fixes both its minimum and maximum surface size, so it
acknowledges later configure events but never silently claims resize support.
When both buffers are busy, presentation reports one closed back-pressure
outcome rather than allocating a third buffer, blocking a UI loop, or writing
into a compositor-owned mapping.

## Consequences

- Anodrel obtains an auditable first Linux window path while sharing its
  renderer and identity artwork with Windows.
- Unit tests can validate protocol encoding, strict decoder bounds, global
  discovery, locator validation, and buffer availability without a compositor.
- A real Wayland session has a short manual acceptance check: open the lab,
  see the branded surface, resize attempts remain fixed, and close exits
  cleanly.
- The adapter introduces one explicit environment-dependent desktop boundary.
  Missing, malformed, inaccessible, incompatible, or rejected compositor
  state produces a closed local failure; it is never exposed through Anodrel's
  application protocol.

## Deliberately absent

- X11, XWayland, a fallback toolkit, and remote display support;
- application packages, product executable identity, session IPC composition,
  installation, updates, or a product launcher;
- text shaping, arbitrary text, app-proposed titles or identifiers, menus,
  dialogs, clipboard, notifications, networking, credentials, storage
  capability, or logging;
- pointer, keyboard, touch, drag-and-drop, clipboard, accessibility, display
  scale, resize, fullscreen, focus, multi-window, or application-event
  support;
- GPU, DMA-BUF, OpenGL, Vulkan, EGL, frame callbacks, animation, or damage
  optimisation beyond the fixed two-buffer back-pressure rule.

## Alternatives considered

**Use libwayland-client.** It is a mature implementation but would become a
shipped library Anodrel does not own. Refused under Decision 0005.

**Use a cross-platform toolkit or webview.** Both would replace the native
boundary this platform is meant to own. Refused.

**Start with X11 or XWayland.** That adds a second protocol route and its
compatibility policy before the owned Wayland route is proven. Deferred.

**Expose a general Wayland connection API.** It would let application code
choose compositor objects, raw requests, and native authority. Refused.

**Allocate on every frame.** That makes compositor delay become unbounded
memory growth. Refused.

## Revisit conditions

Revisit before adding any application-controlled Linux window, dynamic title
or app ID, resizing, input, text, accessibility, animation, frame pacing,
additional graphics path, X11/XWayland, Linux product launch, package
identity, installation, updates, or non-Linux platform support.
