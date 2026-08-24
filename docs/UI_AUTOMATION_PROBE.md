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
  Lab's immutable document; and
- the fixed visible `ui.lab.field` rectangle is non-empty, contained by the
  window rectangle, and resolves through Windows UI Automation at its centre;
  and
- that same field's provider-side `IValueProvider` is observable through
  Windows' client-side `IUIAutomationValuePattern`, with its compiled empty
  initial value and `IsReadOnly = true`.
- no Anodrel semantic node in the fixed UI Lab exposes the standard `Invoke`
  pattern; its displayed buttons are local diagnostics, not authenticated
  application actions.

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
it. For the one desktop-level centre-point check, the host temporarily places
that private window above ordinary windows; it is destroyed when the run ends.
A successful run prints `UI Automation property probe passed` and closes the
window itself. A failure closes the window and exits non-zero with a fixed
failure category.

## Boundary

The route is host-only and read-only. It uses direct `Ole32`, `OleAut32`, and
UI Automation client APIs; it ships no browser, webview, test framework, or
third-party runtime binding.

No application protocol message, SDK method, capability, installed-record
field, UI document field, callback, listener check, or UI Automation pointer
crosses this boundary. The client inspects only the fixed host-created UI Lab
window. It neither calls Invoke, SetFocus, Scroll, SetValue, ClickablePoint,
nor registers an event handler. It reads only the compiled empty value from the
fixed field's read-only client-side Value pattern and checks only the presence
of the standard Invoke pattern; it never obtains an Invoke-method interface or
calls an action. Its one geometry query derives the centre from the fixed
field's current published rectangle; it accepts no point or selector.
The host changes only the temporary test window's z-order for that query, not
an application's window state or any other process's window state.

The property probe supplements rather than replaces the manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`: Narrator proves spoken behaviour, and
Inspect or Accessibility Insights still proves highlight geometry and visual
tool interoperability.

See Decisions 0106 through 0110 and `docs/ACCESSIBILITY.md`.
