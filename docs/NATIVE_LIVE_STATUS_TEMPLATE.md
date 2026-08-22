# Native live-status template

**Status:** Windows development template implemented by Decision 0101.

`anodrel-native-app-tool init-live-status` creates a small Rust application
that demonstrates Anodrel's exact version-3 visible-status path. It uses only
first-party Anodrel crates, the Rust standard library, and the direct invited
Windows pipe adapter. It has no Node.js, webview, browser runtime, raw protocol
construction, callback, native handle, or accessibility-listener access.

## Generate and run

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-native-app-tool -- init-live-status .\live-status-app live-status-app "Live Status App"
Set-Location .\live-status-app
cargo build --release
$clientPath = (Resolve-Path .\target\release\live-status-app.exe).Path
$hostManifest = (Resolve-Path ..\native\Cargo.toml).Path
cargo run --release --manifest-path $hostManifest -p anodrel-windows-host -- --native-live-status-template-client $clientPath
~~~

After the host window opens, start Narrator if needed. Activate the fixed
actions in order: **Publish polite result**, **Publish assertive result**, and
**Complete status session**. The host publishes each changed status as ordinary
visible text and may raise its one best-effort UI Automation live-region event.
The executable receives only its document revisions and the three semantic
actions; it cannot tell whether Narrator or any other assistive technology heard
a result.

Inspect or Accessibility Insights should report each status as a `Text` element
with the matching `LiveSetting`: `Polite` for the initial and second documents,
then `Assertive` for the third. See `docs/UI_LIVE_ANNOUNCEMENTS.md` for the
manual verification rules.

## Fixed boundary

The host grants exactly `ui.document.write`, `ui.events.read`, and
`session.close`. The template cannot receive or choose capabilities, a window
title, application identity, native window handle, UI Automation event,
recipient, callback, notification, path, network value, or configuration.
It is explicitly unverified development code, not an installed application or
production packaging path.

## Generator contract

`anodrel-native-app-tool init-live-status <destination> <project-slug>
<display-label>` accepts only the existing validated new-directory arguments.
It refuses an existing destination and writes only `Cargo.toml`, `README.md`,
and `src/main.rs` with checkout-relative Anodrel paths. It cannot accept status
text, politeness, timing, an action, a capability list, a native setting,
identity, source path, network input, or secret.

The generated source has exactly three `replace_document_v3` calls. Their
statuses are, in order: **Ready to publish a visible result.** (`polite`),
**Verification is complete.** (`polite`), and **Verification succeeded.**
(`assertive`). Each document contains exactly one matching enabled semantic
action. No timer advances the sequence.

## Verification

Focused tests decode the generated v3 document and check the frozen source
shape. A real invited-pipe integration test builds a generated release
executable, verifies each exact status and revision, supplies the matching
revision-bound action, then verifies clean child exit and self-close. It does
not need Narrator because it proves the host/session boundary, not an operating
system announcement.

The manual Windows acceptance check uses Narrator and Inspect or Accessibility
Insights. Open the template window first, then start Narrator. Confirm the
polite-to-polite-to-assertive visible sequence and that Inspect reports the
matching `LiveSetting` values. The generated application must receive no
evidence that either result was announced. See `docs/UI_LIVE_ANNOUNCEMENTS.md`.
