# Anodrel Windows UI Automation Invoke probe

## Purpose

`--uia-invoke-probe <native-client.exe>` is a development-only Windows
acceptance diagnostic. It launches one selected compiled native UI child through
the normal fixed-grant authenticated session, then uses a direct Windows UI
Automation client in a separate MTA apartment to invoke that child's one
compiled `native.ui.complete` button.

The child exits successfully only after its ordinary `ui.events.read` receives
that action at revision 1 and it requests its own session close. A successful
probe therefore proves the real provider, Windows client, bounded semantic
mailbox, child event read, and session close sequence together. It does not
prove screen-reader speech, arbitrary action selection, disabled-button
refusal, or application behaviour beyond this compiled diagnostic.

## Running it

From the repository root on Windows:

~~~text
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-ui-client-sample
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-invoke-probe native/target/release/anodrel-native-ui-client-sample.exe
~~~

Or double-click `start-uia-invoke-probe.bat` in the repository root.

The temporary authenticated session window may appear briefly and closes after
the compiled child consumes the invoked event. A successful run prints `UI
Automation Invoke probe passed.` A failure closes only that development
session, terminates its selected child, and exits non-zero with a fixed error.

## Boundary

This route is private to the Windows development host. It uses only direct
Ole32 and Windows UI Automation client APIs; it ships no browser, webview,
test framework, or third-party runtime binding.

The probe accepts a development executable path but no document, element ID,
selector, coordinate, action value, pattern, result, callback, or application
input. The host supplies the one immutable document and finds its one compiled
button by its exact AutomationId, name, and control type. It calls standard
`IUIAutomationInvokePattern::Invoke` once and never exposes the interface,
candidate, mailbox event, or outcome to an application, protocol, or SDK
caller.

It supplements the non-Invoke UI Lab property probe and the manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`; it is evidence for one authenticated
action route, not a replacement for Narrator or Inspect. See Decisions 0069
and 0111.
