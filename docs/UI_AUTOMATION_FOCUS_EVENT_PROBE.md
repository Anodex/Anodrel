# Anodrel Windows UI Automation focus-event probe

## Purpose

`--uia-focus-event-probe` is a development-only Windows acceptance diagnostic.
It opens the fixed UI Lab, registers one private direct Windows UI Automation
focus-change callback in a separate MTA apartment, and calls `SetFocus` for
the compiled `ui.lab.field` control.

The probe passes only when Windows delivers one focus-change event whose sender
has that same fixed AutomationId. It reports one fixed process result and
never exposes an event, element, listener, or identity to an application.

## Running it

From the repository root on Windows:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-focus-event-probe
~~~

Or double-click `start-uia-focus-event-probe.bat` in the repository root.

The temporary UI Lab window is visible briefly, then closes itself. A
successful run prints `UI Automation focus-event probe passed.` A failure
closes the window and exits non-zero with a fixed failure category.

## Boundary

This route is host-only. It uses direct Ole32 and Windows UI Automation client
APIs; it ships no browser, webview, test framework, or third-party runtime
binding.

The probe accepts no window, selector, document, coordinate, action, value,
event target, listener, or focus target. The listener is fixed inside the
short-lived diagnostic, records at most one sender AutomationId, and is
unregistered before the fixed outcome is reported. The production host creates
no listeners and does not check whether assistive technology is present.

It does not invoke a control, write a value, send synthetic input, activate or
foreground a window, or test screen-reader delivery. Its only mutation is the
existing bounded host-owned focus transition for its temporary window. This
probe supplements the separate focus-property probe and manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`; see Decisions 0074 and 0112.
