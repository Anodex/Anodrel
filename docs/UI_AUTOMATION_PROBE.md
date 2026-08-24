# Anodrel Windows UI Automation property probe

## Purpose

`--uia-property-probe` is a development-only Windows acceptance diagnostic. It
opens the fixed UI Lab and queries its published UI Automation tree through a
direct Windows UI Automation client in the same host process.

The probe is deliberately narrower than Inspect, Accessibility Insights, or
Narrator. It repeats the checks that can be compared to a fixed contract:

- the UI Lab window has the Anodrel automation root;
- Windows contributes the expected native `TitleBar` peer for the framed
  window, while every Anodrel semantic node appears after it in its documented
  parent and sibling order in both the raw and control views; and
- each node's `Name`, `AutomationId`, and `ControlType` match the native UI
  Lab's immutable document.

It gives an operator one process exit status and a fixed console result. It
does not print returned property values, expose them to an application, or
accept a window handle, selector, document, or UI Automation input.

## Running it

From the repository root on Windows:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-property-probe
~~~

Or double-click `start-uia-property-probe.bat` in the repository root.

The diagnostic window is visible briefly while a separate MTA worker queries
it. A successful run prints `UI Automation property probe passed` and closes
the window itself. A failure closes the window and exits non-zero with a fixed
failure category.

## Boundary

The route is host-only and read-only. It uses direct `Ole32`, `OleAut32`, and
UI Automation client APIs; it ships no browser, webview, test framework, or
third-party runtime binding.

No application protocol message, SDK method, capability, installed-record
field, UI document field, callback, listener check, or UI Automation pointer
crosses this boundary. The client inspects only the fixed host-created UI Lab
window. It neither calls Invoke, SetFocus, Scroll, SetValue, nor registers an
event handler.

The property probe supplements rather than replaces the manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`: Narrator proves spoken behaviour, and
Inspect or Accessibility Insights still proves highlight geometry and visual
tool interoperability.

See Decision 0106 and `docs/ACCESSIBILITY.md`.
