# Anodrel UI Automation ScrollItem probe

## Purpose

`--uia-scroll-item-probe` is a fixed Windows development diagnostic. It opens
one host-owned UI Lab window and gives a private MTA UI Automation client one
compiled `ScrollItemPattern` descendant, `ui.lab.scroll.exercise-9`.

The client first confirms that the item is published but fully clipped: its
`IsOffscreen` property is true, its bounding rectangle is empty, and it has no
Invoke pattern. It then calls Windows' standard `ScrollIntoView` method and
reads a fresh provider publication that must report the same item visible with
a non-empty rectangle and still no Invoke pattern.

## Running it

Run it directly from the repository root:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-scroll-item-probe
~~~

Or run the full eight-probe suite:

~~~powershell
.\scripts\verify-windows-accessibility.ps1
~~~

The probe opens and closes one temporary host-owned window. It creates no
trust, installer, package, network, application, or persistent user state. On
2026-09-19 it passed against the release build.

## Boundary

The host owns the HWND, direct UI Automation client, compiled target ID,
pattern interface, geometry, and result. The UI Lab has no authenticated action
mailbox, so the target remains non-Invoke before and after reveal. No
application, protocol message, SDK method, or user-supplied value can select a
target, request scrolling, read a rectangle or scroll position, or learn the
result.

This proves the real Windows client/provider call and fresh publication for one
fixed host route. It does not prove Narrator speech, Inspect's visible
highlight, the authenticated v2 session path, already-visible no-op behavior,
nested-scroll refusal, or multi-monitor/user interaction. Those remain manual
or focused host-test evidence as documented in
[the scroll-item contract](UI_AUTOMATION_SCROLL_ITEMS.md).
