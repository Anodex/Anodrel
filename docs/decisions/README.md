# Architecture Decision Records

Important decisions are recorded here using sequential numbers.

Each record should include:

- status;
- date;
- context;
- decision;
- consequences;
- conditions that would cause the decision to be revisited.

Current records (newest first):

- 0072: Session window state is a closed, write-only command.

- 0071: UI Automation field values are read-only host snapshots.

- 0070: UI Automation reports the owned focus snapshot without controlling it.

- 0069: UI Automation button invocation uses the existing semantic action
  path.

- 0068: The host decides where text breaks.
- 0067: An application learns a value, never the typing.
- 0066: An application proposes a window title; the host composes it.
- 0065: A crash record is host-only, payload-free, and covers the easy case
  honestly.
- 0064: Retained raster effects may trade a bounded, tested error for frame
  cost.
- 0063: Windows accessibility maps the owned snapshot, one direction only.

- 0062: Notifications start as a one-way bounded announce over Shell32.

- 0061: Verified product sessions are proved by a development-only signed
  fixture.

- 0060: Verified Windows product components share one host-owned lifetime.

- 0059: Windows pipe workers use a host-only stop signal.

- 0058: Registered interactive-session resources stay host-owned.

- 0057: Registered sessions compose identity-bound native services.

- 0056: Credential protocol uses separate exact grants.

- 0055: Windows high contrast uses direct system colours.

- 0054: Native cancellation is bounded, ordered, and pre-execution.

- 0053: Diagnostic log reads stay bounded and closed.

- 0052: Storage protocol uses independent state grants.

- 0051: Application state starts as one bounded atomic snapshot.

- 0050: File text reads use selection references.
- 0049: File access requires session-bound selection identity.
- 0048: Save-file dialogs use a dedicated session capability.
- 0047: Save-file selection stays separate from writing.
- 0046: Open-file dialogs use a dedicated session capability.
- 0044: File dialogs start with bounded portable values.
- 0045: Modal file dialogs cross through a bounded UI-thread bridge.

- 0043: External link protocol access is capability-checked.

- 0042: External links start as validated HTTPS handoff.

- 0041: Clipboard protocol operations are separate and bounded.

- 0040: Clipboard starts with bounded Unicode text.

- 0039: Scroll documents use a new exact format version.

- 0038: Scroll containers use host-retained state and layout metrics.

- 0037: Scroll state starts as an owned bounded primitive.

- 0036: Session close uses a host-owned coalescing signal.

- 0035 — UI actions use bounded authenticated pull delivery.

- 0034 — Windows UI Session Lab consumes one bounded mailbox.

- 0033 — UI document delivery coalesces in one session mailbox.

- 0032 — UI document replacement is capability-checked and session-bound.

- 0031 — Windows UI preview is an explicit bounded developer tool.

- 0030 — Native UI session state uses atomic document revisions.

- 0029 — Native UI document interchange is strict and capability-free.

- 0028 — Native UI appearance is portable semantic data.

- 0027 — Native UI focus starts as owned layout-bound traversal.

- 0026 — Native UI accessibility begins with an owned semantic snapshot.

- 0025 — Native UI starts with a constrained declarative foundation.

- 0024 — Native transport performance uses an owned repeatable measurement
  tool.

- 0023 — Session capability grants are bound to installed machine policy.

- 0022 — Windows credentials use a narrow current-user Credential Manager
  store.

- 0021 — Application directories are derived from host-validated identity.

- 0020 — Windows launch requires a locked and revalidated executable.

- 0019 — Installed application policy is read from the machine-wide Windows
  registry.

- 0018 — Launch policy is bound through an external installed application
  record.

- 0017 — Windows Authenticode verification is isolated from launch authority.

- 0016 — The first diagnostic log is typed and internal.

- 0015 — The brand mark ships as the authored asset, not a reconstruction.
  Supersedes the asset reasoning in 0013.

- 0014 — Startup Lab shows planned actions in a declared pending state.

- 0013 — First-party surfaces are drawn by a software renderer.

- 0012 — Windows host owns per-window state and final-window shutdown.

- 0011 — Windows host uses a bounded single-instance lifecycle.

- 0010 — Application hosting starts with a verified text package.

- 0001 â€” Anodrel lives in its own repository.
- 0002 â€” Windows is the first supported operating system.
- 0003 â€” Establish the protocol and mock host before a native host.
- 0004 â€” Tao/Wry Windows proof host (superseded).
- 0005 â€” Production native hosts are first-party modules over OS APIs.
- 0006 â€” First production-path Windows host uses direct Win32 modules.
- 0007 â€” Native transport starts with a bounded session engine.
- 0008 â€” Windows transport uses an authenticated named pipe.
- 0009 â€” Windows child bootstrap uses a one-use inherited anonymous pipe.
