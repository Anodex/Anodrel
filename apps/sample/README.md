# Anodrel sample application

This sample is a deliberately small SDK consumer. It does not import native
host internals; it receives a transport backed by the mock host and uses the
public client API.

Run it from the repository root with:

~~~text
npm run demo
~~~

`src/native-client.ts` is a separate development-only entry point. The direct
Windows host launches it with a private standard-input invitation so it can
authenticate to the real named pipe, call `platform.health`, and submit one
strict UI document. Its command is documented in `docs/DEVELOPMENT.md`; it is
not a packaged or trusted content host. Use `--sample-ui-client` to display
that authenticated replacement through the native UI Session Lab. Its visible
action is delivered only as a revision-bound semantic event after the client
calls `ui.events.read`; it has no direct native authority.

Use `--sample-ui-file-client` or `--sample-ui-save-client` with the Windows
host to run the same authenticated session through the real open or save
picker. These diagnostics prove only user-mediated path selection; they never
read, create, truncate, or write a selected file. `--sample-ui-file-write-client`
is separate: it captures one host-retained output object after the save picker,
then writes a fixed diagnostic line through its one-use save reference. Choose
a new temporary `.txt` filename for that diagnostic, then inspect its contents.

`--sample-ui-file-binary-write-client` is the separate bounded binary-output
diagnostic. It captures one output object after the save picker and writes the
fixed bytes `41 6E 6F 64 72 65 6C 00 FF` through its one-use save reference.
Choose a new temporary `.bin` filename, then inspect the result. It cannot
write a later path, attach a type, stream data, or reuse that reference.

Use `--sample-ui-window-state-client` to exercise the separately granted
`window.state.set` command. The development client asks its own session window
to minimise, maximise, and restore; it never receives a handle, target, or
state readback. See `docs/WINDOW_STATE.md` and `docs/DEVELOPMENT.md`.

Use `--sample-ui-window-fullscreen-client` to exercise the separately granted
`window.fullscreen.set` command. The development client asks only for
borderless fullscreen and then windowed restoration of its own session window;
it never receives a handle, monitor, geometry, display mode, or fullscreen
state. See `docs/WINDOW_FULLSCREEN.md` and `docs/DEVELOPMENT.md`.

Use `--sample-ui-window-size-client` to exercise the separately granted
`window.size.set` command. The development client asks only for an 800 by 520
logical client area for its own session window; it never receives a handle,
position, monitor, DPI, outer bounds, or size readback. See
`docs/WINDOW_SIZE.md` and `docs/DEVELOPMENT.md`.

`--sample-ui-window-size-fullscreen-client` is the separate boundary
diagnostic: it enters reversible fullscreen, expects a size request to return
only `window.unavailable`, then restores the session window.

`anodrel.application.json` and `content/main.txt` are a separate static
application package for the first Windows content surface. The host
verifies the declared SHA-256 digest and package containment before drawing the
text. To view it, run:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
~~~

If `content/main.txt` changes, its SHA-256 value in the manifest must be
updated in the same review. Its Git attribute preserves exact bytes so line
ending conversion cannot invalidate that digest. This format has no scripts,
executable entry point, navigation, or native bridge.

## Native UI preview

`anodrel.ui.json` is a strict native UI document for the explicit Windows
developer preview. From the repository root, run:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-preview apps/sample/anodrel.ui.json
~~~

It exercises the document decoder, direct native layout, pointer hit testing,
and keyboard focus without opening an application session or native capability.
See `docs/UI_PREVIEW.md`.
