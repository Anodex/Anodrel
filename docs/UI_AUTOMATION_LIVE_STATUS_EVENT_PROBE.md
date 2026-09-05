# Anodrel Windows UI Automation live-status event probe

## Purpose

`--uia-live-status-event-probe <native-client.exe>` is a development-only
Windows acceptance diagnostic. It runs one compiled two-document child through
the ordinary authenticated session and verifies one real
`UIA_LiveRegionChangedEventId` callback from its fixed visible status.

The compiled child first publishes a polite version-3 status and waits for one
fixed `native.live.prepare` action. A private direct Windows UI Automation
client registers one subtree-scoped listener on the session root, arms it, and
invokes that prepared action. The child publishes a changed assertive status at
revision 2 and waits for `native.live.complete`.

The probe passes only when Windows delivers the fixed live-event ID from
`native.live.status`. It unregisters the listener, invokes the fixed complete
action through a fresh private client, and passes only after the child closes
normally.

## Running it

From the repository root on Windows:

~~~text
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-live-status-event-client
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-live-status-event-probe native/target/release/anodrel-native-live-status-event-client.exe
~~~

Or double-click `start-uia-live-status-event-probe.bat` in the repository root.

The temporary authenticated window may appear briefly and closes after the
compiled child consumes its second fixed action. A successful run prints `UI
Automation live-status event probe passed.` A failure closes only that
development session, terminates its selected child, and exits non-zero.

## Boundary

This route uses only first-party Anodrel code, the Rust standard library, and
direct Windows UI Automation APIs. It accepts a development executable path,
but no document, selector, event choice, callback, status text, coordinate,
or application data. The child cannot learn that a listener exists or whether
Windows delivered an event.

It proves one real event route, not Narrator speech, Inspect correctness,
repeated updates, silent rejected states, or a delivery guarantee. Manual
screen-reader and Inspect verification remains required. See Decision 0100 and
Decision 0202.
