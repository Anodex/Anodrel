# Anodrel Windows UI Automation focus probe

## Purpose

`--uia-focus-probe` is a development-only Windows acceptance diagnostic. It
opens the fixed UI Lab and uses a direct Windows UI Automation client in a
separate MTA apartment to request focus for its compiled `ui.lab.field`.

The probe then asks Windows for the focused UI Automation element and passes
only when its AutomationId is the same fixed `ui.lab.field` value. It gives an
operator one process exit status and a fixed console result; it does not print
or expose a focus identity to an application.

## Running it

From the repository root on Windows:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-focus-probe
~~~

Or double-click `start-uia-focus-probe.bat` in the repository root.

The temporary UI Lab window is visible briefly, then closes itself. A
successful run prints `UI Automation focus probe passed`. A failure closes the
window and exits non-zero with a fixed failure category.

## Boundary

This route is host-only. It uses only direct `Ole32` and Windows UI Automation
client APIs; it ships no browser, webview, test framework, or third-party
runtime binding.

The probe accepts no window, selector, document, coordinate, action, value,
event, or focus target. It finds the one compiled field in the host-created UI
Lab, calls standard `SetFocus`, then reads standard `GetFocusedElement`. It
does not invoke a control, write a value, send synthetic input, activate or
foreground a window, subscribe to events, or test for assistive technology.
Its only mutation is the existing bounded host-owned focus transition for its
temporary window.

The focus probe supplements rather than replaces the manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`: Narrator proves usable spoken and input
behaviour, while Inspect or Accessibility Insights still proves every property
and highlight rectangle. See Decisions 0073 and 0109.
