# Linux windowing

## Status

The first Linux desktop surface is implemented as a development-only direct
Wayland diagnostic: **Anodrel Linux Lab**. It proves that an Anodrel canvas can
reach a compositor and consume one host-owned local pointer activation without
a browser engine, webview, UI toolkit, graphics library, or libwayland runtime
dependency.

It is not a Linux application host. It cannot load an application, start an
invited client, use Anodrel's Linux transport, or provide a product launch.
Those are later composition decisions.

## Ownership

anodrel-linux-wayland owns a small, fixed Wayland client lifecycle:

~~~text
host-selected desktop session
    │
    ▼
XDG_RUNTIME_DIR + WAYLAND_DISPLAY validation
    │
    ▼
direct local Wayland socket
    │
    ├── wl_registry discovery and sync
    ├── wl_compositor + wl_shm + xdg_wm_base binding
    ├── one fixed xdg_toplevel role
    └── two private memfd-backed XRGB8888 buffers
              │
              ▼
      one Anodrel canvas copied and committed
              │
              ▼
  optional wl_seat + wl_pointer activation probe
~~~

The application protocol never sees the socket, its path, a Wayland object ID,
a native window handle, a buffer, a display reading, a compositor event,
pointer value, or an environment value. The lab chooses its own fixed title
and desktop label.

## Supported environment

This slice requires a local little-endian Linux Wayland session. The inherited
environment must provide:

- XDG_RUNTIME_DIR, an absolute runtime directory; and
- WAYLAND_DISPLAY, a non-empty single filename under that directory.

The adapter rejects an empty name, an absolute display path, dot or parent
components, embedded NUL, and a Unix-socket address that cannot fit in Linux's
fixed sockaddr_un path storage. It intentionally does not guess wayland-0,
inspect other session state, or fall back to X11/XWayland.

Failure categories are closed and do not reveal the runtime directory, socket
name, server text, native error number, or received file descriptor.

## Protocol boundary

Wayland starts with the implicit display object. The lab gets a registry,
waits for the initial sync callback, and requires the compositor to announce
wl_compositor, wl_shm, and xdg_wm_base. It binds only its supported versions
and waits for the wl_shm format announcement before creating a surface.

The surface receives a permanent xdg_toplevel role. The lab sends an empty
initial commit, acknowledges each later xdg_surface.configure, answers each
xdg_wm_base.ping, and exits on xdg_toplevel.close. It fixes both minimum and
maximum size at 960 × 640 pixels. A later configure is therefore acknowledged
but does not broaden this slice into resize handling.

When the desktop advertises a seat with pointer capability, the lab binds it
at version 1 and asks for one pointer only after its capability event. The
pointer can activate one compiled lower-panel target with a left-button press
and release. Coordinates, serials, button values, timestamps, scrolling, and
seat identity stay inside the adapter. The only result is one closed
development-only `Activated` outcome, which the lab uses once to show its
completed appearance. A pointerless session remains able to present and close
the lab; it simply has no activation probe.

The wire codec accepts only bounded, 4-byte-aligned messages. It keeps
client-created object IDs dense, validates every string terminator and array
length, and treats a wl_display.error, malformed message, unexpected object,
or any inbound file descriptor as a failed local desktop connection. The
server's error text is discarded.

## Presentation and performance

The portable canvas stores pixels as 0xAARRGGBB. On little-endian Linux, their
bytes line up with Wayland's XRGB8888 shared-memory layout, so the lab can copy
exactly one canvas into a mapped memfd with no colour conversion. Transparent
canvas pixels are made opaque only by the completed diagnostic surface before
it is presented.

There are exactly two shared mappings:

1. a frame copies into one currently available mapping;
2. the lab attaches, damages, and commits that buffer;
3. the compositor later sends wl_buffer.release;
4. only then may the lab overwrite that mapping.

If both mappings are busy, the lab returns Backpressured. It does not allocate
another mapping, block waiting for the compositor, or overwrite a buffer the
compositor might still read. The first lab renders one settled frame only, so
it uses no animation timer or frame callback.

## Run and verify manually

On a Linux desktop that provides the two required environment values:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-linux-window-lab
~~~

Expected result:

1. A fixed-size **Anodrel Linux Lab** window opens with the first-party
   branded diagnostic surface.
2. Clicking its highlighted lower panel once changes that panel to its
   completed appearance when the desktop provides a pointer.
3. Attempting to resize it does not turn it into a resizable application
   surface.
4. Closing it from the desktop chrome makes the process exit normally.

The automated Linux job tests the pure wire, locator, registry, pointer,
activation, and back-pressure rules only. It does not start a compositor, and
a green workflow does not prove visual presentation, desktop decoration,
compositor compatibility, or physical user interaction.

## Deliberate limits

No X11/XWayland route, application input, text shaping, accessibility, scale
awareness, resizing, window state, fullscreen, menus, dialogs, clipboard,
notifications, application documents, IPC composition, executable identity,
product launch, packaging, installation, or updates exists in this component.

See Decisions 0128 and 0129, docs/RENDERER.md, and docs/LINUX_TRANSPORT.md.
