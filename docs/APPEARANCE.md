# Windows system appearance foundation

**Status:** Windows native UI foundation. This is not an application theme API.

## Purpose

Anodrel's portable UI tree expresses only semantic presentation roles. The
Windows host needs a small, direct way to honour a user who enables Windows
high-contrast mode without importing a browser, webview, UI framework, or theme
runtime.

`anodrel-windows-appearance` reads the current Windows high-contrast flag with
`SystemParametersInfoW(SPI_GETHIGHCONTRAST)` and a fixed system-colour set with
`GetSysColor`. It has no write API, no observer, no registry access, no process
authority, and no application or protocol input.

## Values

The adapter returns only:

| Value | Use |
| --- | --- |
| high-contrast enabled | Whether a host should use system colours for an accessible fallback. |
| window / window text | Main surface and readable foreground. |
| button face / button text | Raised and neutral-action surfaces. |
| highlight / highlight text | Accent action, hover, and focus treatment. |

If Windows cannot report the high-contrast flag, the adapter safely leaves the
host's standard appearance in place. It still reads only the fixed colour set.

## First consumer

The Windows UI Lab and its authenticated UI-session view map their portable
semantic roles to the direct system colours only while Windows reports
high-contrast mode. In normal mode they retain Anodrel's authored visual
identity. This changes colour selection only: document validation, layout,
scrolling, hit testing, focus order, action delivery, protocol grants, and
window lifecycle are unaffected.

The adapter is queried once for a UI-Lab paint. It does not install a listener
or retain settings, so a future live appearance-notification policy needs a
separate decision.

## Verification

The adapter unit-tests Windows `COLORREF` channel conversion. The Windows host
tests render the UI Lab through its palette seam without a web surface; native
tests and strict linting cover the direct adapter and the host together.
