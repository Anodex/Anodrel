# Windows release readiness

**Status:** Windows is Anodrel's reference platform. This document defines the
remaining release gates; it does not describe Linux or macOS work.

## Purpose

"Windows complete" means a user can install, trust, run, use, update, and
remove an Anodrel application with the same discipline that protects the
development host today. It does **not** mean feature-for-feature Electron
parity, nor that every possible desktop application is already expressible.

The estimate is currently **about 72%** of that Windows release goal. The host,
private transport, package validation, native UI foundation, direct Windows
services, performance guard, accessibility provider, and Apps & features
registration/removal path are substantial working foundations. The remaining
work is concentrated in distribution, production identity, update trust, manual
desktop proof, and application-surface breadth rather than in a missing window
or transport base.

## Release gates

| Gate | State | Current evidence | Required to close it |
| --- | --- | --- | --- |
| Native host and private child session | Built | Direct Win32 host, authenticated named pipe, CNG invitation, single-instance behaviour, group shutdown, and product-session coordinator. | Repeat the joined fixture check as part of each release candidate. |
| Controlled package and executable launch | Built for development | Canonical containment, content digest, locked executable revalidation, Authenticode verification, external installed record, and a separately verified Start-menu launcher route are implemented. | Adopt a production identity and prove the installed signed launcher path. |
| Native application UI | In progress | Owned layout, input, menus, scrolling, text entry, high-contrast palette, and multiple windows are directly rendered by the Windows host. Portable owned font, glyph, and unshaped-run foundations exist but are not yet connected to the current GDI text painter. | Expand only the reusable controls and behaviours required by the first real application; test each as a bounded host capability. |
| Accessibility | In progress | The UI Automation provider supports reading, navigation, hit testing, focus, Invoke, Value, structure, live-status, and scroll boundaries. | Complete every documented manual Narrator and Inspect check for the current provider, then add further patterns only when a real UI requires them. |
| Performance | Guarded | The release-only frame guard currently measures 6.43 ms average and 8.04 ms worst sustained frame against a 16 ms interval; a fixed static-window report measures this process's 30-second idle CPU and memory. | Keep the release guard, startup report, idle report, and equivalent real-application measurements in each release candidate. |
| Product fixture | Built, operator verification pending | A first-party child and launcher fixture exercises machine record, dual-signature verification, child bootstrap, session UI, and shutdown. | Run its elevated provision, launcher launch, action, and removal paths on the release machine. |
| Signed distribution and installation | Development acceptance prepared | The owned tools author bounded bundles, validate and embed strict release manifests, sign one fresh checked image through direct Windows APIs and an explicit current-user certificate, verify current-image Authenticode, privately stage, match extracted signers, promote without overwrite, publish fixed policy, register Start menu and Installed apps, recover, and remove through native consent and direct elevation—without an installer framework. A fixed development script now prepares that complete signed installer chain without auto-installing it. | Choose certificate custody and timestamp policy, run the signed installer fixture through consent, UAC, Start menu, Installed apps, uninstall, restart cleanup, and recovery proof, then complete production release verification. |
| Updates | Foundation in progress | A current signed candidate must match the selected installed publisher and be strictly newer; a no-argument transaction refreshes that decision, retains one fixed prior record, and a separately verified fixed command can restore it. A signed release can declare one fixed catalogue source; direct Windows CMS verifies one exact publisher; the direct downloader can stream a preflight-eligible image into one fresh hash-verified private file; a product window's fixed native system-menu action reaches consent, bounded signed-byte caption/taskbar progress, UAC, and postcondition proof off its UI thread, then presents restart-needed completion only after policy proof. | Define production endpoint operation, key rotation, signed-fixture recovery proof, automatic-restart policy, and production release verification after distribution identity is chosen. |
| Release documentation and templates | Development-complete | Native SDK, templates, package tool, Startup Lab, diagnostics, and contract documents are maintained in the repository. | Publish installation, upgrade, recovery, and support documentation with the production package design. |

## Decisions that need product authority

The next two gates cannot be completed honestly by code alone:

1. **Production signing identity.** Choose the certificate issuer, who holds the
   private key, and how renewal or loss is handled. A development certificate is
   intentionally not a production identity.
2. **Production signing and release operation.** The owned installer contract
   defines a machine-scoped route, but the certificate, custody, renewal,
   release procedure, and any stable package identity still need product
   authority.

Those choices determine the update trust model, so updating must follow them.
The platform will not create machine trust or select a certificate authority on
an operator's behalf.

## Required desktop proof

Automated checks protect contracts but cannot prove every Windows desktop
interaction. Before a Windows release, run and record the relevant procedures
in `docs/DEVELOPMENT.md` and the feature documents:

- native menu bar, click, shortcut, and semantic event delivery;
- pointer-originated context menu;
- file and folder picker accepted and cancelled paths;
- foreground, resize, fullscreen, and multi-window behaviour;
- Narrator and Inspect checks for the published accessibility tree, focus,
  actions, field values, structure changes, hierarchy, scroll, and live status;
- signed fixture provision, launch, action, close, verification, and removal;
- signed installer fixture preparation, consent, UAC, Start-menu launch,
  verification, uninstall, and removal;
- release frame-budget, startup-report, and transport performance measurements.

A procedure that needs an elevated trust change remains an operator action. A
passing automated check must never be presented as proof of a desktop result it
cannot observe.

`scripts/verify-windows-release.ps1` runs the repeatable non-interactive
evidence set: formatting, source and documentation guards, the native workspace
suite, release frame budget, and sample startup report. For a release candidate,
run it with `-IncludeIdleReport` to add the fixed 30-second static-window idle
measurement. That opt-in opens one diagnostic window but performs no trust,
installation, network, or application interaction; it does not replace any
manual item above.

## Windows-first sequence

1. Close the remaining manual Windows acceptance checks and record their
   results without widening application authority.
2. Implement only the Windows UI and service capabilities required by the first
   real Anodrel application, with a protocol contract and decision record first.
3. Resolve production signing, packaging, installation, and update decisions;
   then implement and test that distribution path.
4. Ship a Windows reference release and retain its performance, memory, startup,
   security, and accessibility evidence.
5. Port the completed contracts to Linux, then macOS, without treating their
   foundation labs as equivalent to the Windows release.

Portable work is allowed during this phase only when it directly closes one of
the Windows gates above. This preserves a clear reference implementation rather
than three partially complete platforms.

## Related documents

- [Roadmap](../ROADMAP.md)
- [Architecture](ARCHITECTURE.md)
- [Windows signing foundation](SIGNING.md)
- [Development product fixture](PRODUCT_FIXTURE.md)
- [Installed development fixture](INSTALLED_PRODUCT_FIXTURE.md)
- [Performance plan](PERFORMANCE.md)
- [Accessibility](ACCESSIBILITY.md)
- [Development verification](DEVELOPMENT.md)
