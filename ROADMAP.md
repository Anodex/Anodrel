# Anodrel Roadmap

This roadmap is intentionally staged. The platform must become useful and
stable before Anodex depends on it.

## Phase 0 — Foundation

Status: **In progress**

- Create the standalone repository.
- Establish folder ownership and documentation rules.
- Define the initial architecture and security boundaries.
- Record the repository-separation decision.
- Choose the first supported operating system. **Completed: Windows.**
- Define the first direct Windows API host. **Completed:** a direct Win32
  window, JSON codec, protocol core, and lifecycle smoke test (Decision 0006).
- Define a bounded application-to-host frame and session engine. **Completed:**
  direct wire framing and host session limits (Decision 0007).
- Implement the authenticated Windows named-pipe adapter. **Completed:**
  logon-SID-restricted one-client adapter with CNG invitation (Decision 0008).
- Implement private invitation delivery. **Completed:** bounded `ANBI` record
  over a child-only inherited handle (Decision 0009).
- Define controlled application-content hosting and application identity.
  **Completed for the first no-script package surface:** strict manifest,
  canonical containment, built-in SHA-256 verification, and direct Win32 text
  rendering (Decision 0010). Publisher trust and executable identity remain
  required before product process launch.

Acceptance gate: the project has an agreed architecture, a documented first
milestone, and no dependency on Anodex source code.

## Phase 1 — Contracts and protocol

Status: **Foundation slice in progress**

- Define the platform service interfaces.
- Define protocol envelopes, request IDs, errors, cancellation, and events.
  **Completed for the first native transport cancellation rule:** a bounded,
  ordered pre-execution cancellation control is handled by the authenticated
  session (Decision 0054).
- Define capability and permission declarations.
- Create compatibility and schema tests.
- Build a minimal mock host for application development.

Acceptance gate: a small sample application can communicate with the mock host
using only documented interfaces.

The initial protocol, SDK, mock host, sample application, contract suite, and
bounded native session engine are established. The minimum Phase 0 content
boundary and associated threat-model controls are complete. Product launch,
publisher trust, and a capability bridge remain later native-host gates.

## Phase 2 — Native host

Status: **Direct Windows host in progress**

- Create the first native host, beginning with Windows.
- Implement lifecycle and single-instance behavior. **Completed for the first
  Windows package text surface and Startup Lab:** current-session primary
  instance claim, bounded readiness, and no-data activation request (Decision
  0011). A verified product executable lifecycle remains required.
- Implement window creation and controlled application content loading.
  **Completed for native Windows surfaces:** per-window immutable view
  routing, final-window shutdown, a two-window diagnostic, and the verified
  no-script text package surface (Decision 0012). **The first public window
  capability now exists:** Protocol 1.14 `window.title.set` lets an
  authenticated session holding the separate `window.title` grant propose the
  title of the window it already owns, and the host composes the displayed
  caption with a display name taken from the machine-validated installed record
  (Decision 0066). The request carries no window target, handle, or identifier,
  there is no read counterpart, and installed record version 1.4 adds the grant
  as a strict superset of 1.3. The second narrow public lifecycle capability is
  `window.state.set`: Protocol 1.16 and record version 1.6 define separately
  granted minimise, maximise, and restore requests for a session's own window.
  The SDK, mock host, policy parser, session-local UI-thread bridge, direct
  Windows `ShowWindow` adapter, development diagnostic, and contract tests are
  complete (Decision 0072). The third narrow public lifecycle capability is
  `window.focus.request`: Protocol 1.20 and record version 1.9 define an exact
  empty, separately granted request for Windows to foreground only the
  requesting session's own window. The SDK, mock host, policy parser,
  session-local UI-thread bridge, direct Windows `SetForegroundWindow` adapter,
  product-session composition, and development diagnostic are implemented
  (Decision 0085); its foreground-policy outcome remains a manual desktop
  check. The fourth narrow public lifecycle capability is
  `window.fullscreen.set`: Protocol 1.21 and record version 1.10 define the
  separately granted closed `fullscreen` or `windowed` request for only that
  same session window. The SDK, mock host, policy parser, session-local
  UI-thread bridge, private placement restoration, direct User32 and monitor
  adapter, product-session composition, and development diagnostic are
  implemented (Decision 0086); its desktop entry-and-restore check remains
  manual. Window creation, cross-session closing, geometry, enumeration,
  exclusive display control, monitor selection, state or focus readback, and
  every other window property remain deferred, each needing its own grant,
  decision, and threat-model entry. Protocol 1.23 `window.size.set` is the
  fifth narrow public capability: it chooses only a bounded logical client area
  for that same session window, without a position, target, monitor, DPI,
  geometry readback, or presentation-state change. The SDK, mock host, record
  version 1.12 policy, session-local UI-thread bridge, direct DPI-aware User32
  adapter, product-session composition, and development diagnostic are
  implemented (Decision 0088); scaling and fullscreen-interaction checks remain
  manual. Decision 0092 now specifies the next multi-window section: a
  bounded four-view session group with opaque identities, separate document
  revisions and input queues, and group-wide shutdown. **Completed for the
  portable state and worker-to-UI creation handoff:** the group has a bounded
  take-once request, commit-only snapshot publication, and rollback of failed
  or timed-out native creation. The core, authenticated transport, and
  registered Windows-session composition now use the group for the existing
  primary view without copying its mailboxes. Protocol, direct Windows-host,
  and group-lifetime integration remain in progress; no released protocol
  version lets an application create or target another view yet.
- Define safe application-controlled file output. **Completed for the direct
  Windows UI-session host:** Decisions 0079 and 0087 preserve the legacy
  non-mutating save picker while implementing an independent, one-use
  retained-output-object flow for Protocol 1.17 `file.write_text` and Protocol
  1.22 `file.write_binary`. The latter has its own grant, first-party canonical
  base64url decoder, and 32 KiB decoded bound. Its picker diagnostic remains a
  manual desktop check. Decision 0091 records and defers atomic replacement:
  the direct Windows experiment proved that the available rename path cannot
  preserve the existing retained-target race boundary. Persistent file grants
  remain a separate gate.
- Draw first-party surfaces with a software renderer. **Completed:** a portable
  software rasterizer and brand crate, single-blit presentation, glyph coverage
  lifted from the platform text engine, a run-time generated window icon, and
  per-monitor DPI awareness (Decision 0013). See `docs/RENDERER.md`.
- Define an owned native application UI foundation. **In progress:** a portable
  declarative layout tree with semantic actions establishes the first reusable
  UI contract (Decision 0025), and the Windows UI Lab renders and hit-tests a
  fixed host-owned document through that contract. Its portable accessibility
  snapshot, focus traversal, semantic appearance roles, a strict external
  document format, revision-bound session state, and a capability-checked
  authenticated replacement operation are available. The Windows
  UI Lab renders one compiled-in format fixture, and the explicit Windows
  developer preview can render one bounded operator-selected file. The preview
  is not an application session. The replacement operation can transfer only
  its latest accepted snapshot through a bounded per-session mailbox. The
  development-only Windows UI Session Lab consumes one mailbox in one native
  view, and version-bound pointer/keyboard actions are delivered through a
  bounded authenticated pull operation. A capability-gated request can signal
  close only for that same host-owned session. Its version 2 scroll diagnostic
  delivers a bounded scroll tree through the same path; the Windows host retains
  each viewport position and accepts only local wheel and page input, while the
  client completes a semantic action revealed at the bottom. It has no product
  lifecycle. Operating-system accessibility adapters, public window lifecycle,
  subscriptions, scrollbars, gestures, and broader action-event transport remain
  separate gates.
  The direct Windows UI Lab and session view now also substitute the user's
  fixed high-contrast system colours for their host palette without changing
  the portable document model (Decision 0055).
  Windows accessibility has its contract and its mapping: Decision 0063 targets
  UI Automation and turns the owned semantic snapshot into control types,
  property values, runtime IDs, and screen rectangles, one direction only — an
  application cannot read the tree, learn about focus, or detect that assistive
  technology is present. Both reading-provider slices are built: the window answers
  `WM_GETOBJECT` as a fragment root, and its published elements answer
  `IRawElementProviderFragment` with navigation, runtime identifiers, bounding
  rectangles, and hit testing. **The earlier flat UI Automation reading slice
  is verified:** Narrator reads an Anodrel surface aloud on Windows 11, and an
  Inspect cross-check of each then-published property passed with no failures.
  The new hierarchical tree has focused automated coverage and awaits its own
  manual screen-reader and client check. Reading is
  joined by one bounded action: an enabled button in an authenticated UI session
  exposes `IInvokeProvider`, which queues the same revision-bound semantic
  event as local pointer and keyboard activation (Decision 0069). It has no
  native-input shortcut, accessibility callback, or
  application-specific queue; the diagnostic UI Lab remains readable but
  non-invokable. `GetFocus` and `HasKeyboardFocus` now report a copied current
  host-focus snapshot with no focus event or live-state lookup (Decision 0070).
  A visible enabled focus target can now accept `SetFocus` through a private
  per-window route, and only its owning UI thread revalidates and applies that
  move (Decision 0073). It exposes no application focus surface, native input,
  or activation route. A genuine focus transition now also raises one
  host-only `UIA_AutomationFocusChangedEventId` from a fresh post-change
  provider (Decision 0074); it has no listener, callback, or application
  surface. A visible field now exposes only its copied current text through a
  read-only `IValueProvider`; `SetValue`, selection, caret data, and value
  events stay absent (Decision 0071). After the UI thread accepts and applies
  a strictly newer session document, its root raises one host-only
  `ChildrenInvalidated` structure event; it has no listener, callback, or
  application surface (Decision 0076). Unit and host checks cover all five
  routes; manual screen-reader activation, focus control and event, field-value,
  and structure-event verification remain open. Invoke/property/value/text/
  selection events, live announcements, text patterns and ranges, labelled-by
  or described-by relations, and non-Windows adapters stay deferred. The
  published tree now preserves direct visible parentage, including groups, with
  immutable parent/child/sibling navigation (Decision 0075); its manual
  screen-reader and client hierarchy check remains open. See
  `docs/ACCESSIBILITY.md`, `docs/UI_AUTOMATION_FOCUS.md`,
   `docs/UI_AUTOMATION_EVENTS.md`, and
   `docs/UI_AUTOMATION_STRUCTURE_EVENTS.md`.
   The desktop command surface is a bounded native session menu: Decision 0080
   and `docs/MENUS.md` define a separate `menu.write` grant, a complete model,
   and delivery through the existing revision-checked interaction path.
   **Completed:** portable model and revision state, Protocol 1.18 core
   boundary, installed-record 1.8 grant, SDK, mock host, contract tests, and
   the shared interaction mailbox refactor, one-request UI-thread bridge,
   direct User32 adapter, private numeric mapping, and development diagnostic.
   Protocol 1.24 and Decision 0089 now add one optional unique canonical local
   `Ctrl+<A-Z0-9>` or `Ctrl+Shift+<A-Z0-9>` declaration. The direct User32
   route ignores repeats and nonmatching modifier state, registers no global
   hotkey, and creates the same revision-bound semantic candidate as a menu
   click. **Pending:** manual verification of the real native menu bar, click,
   shortcut, and pull event.
   Decision 0084 now defines the next direct data-egress boundary: one
   host-authorized, origin-bound HTTPS text fetch without a browser runtime,
   headers, bodies, redirects, cookies, credentials, or proxy discovery. Its
   **Completed:** its portable Protocol 1.19 core, strict URL and exact-origin
   policy values, SDK, mock host, contract coverage, direct WinHTTP adapter,
   and a fixed-origin compiled Windows development diagnostic. **Pending:** a
   production installed-record origin policy.
- Establish repeatable native performance measurements. **Completed for the
  owned in-process transport and Windows named-pipe loopback paths:** a
  first-party release performance lab measures 1 KiB and 64 KiB payload latency
  with fixed warmup and documented percentile rules (Decision 0024). **Rendering
  is now covered too:** its `--renderer` workload reports the owned rasterizer's
  drawing stages under a separate `anodrel.renderer.compose.v1` identifier, with
  per-stage pixel counts so a cost can be read per pixel. It opens no window and
  performs no blit, which the report's scope states, so it must never be quoted
  as frame time. **Startup and memory are covered too:** the host's
  `--startup-report` route runs the same checks a Startup Lab surface waits for
  and prints elapsed time, working set, and private bytes as one JSON object. It
  stops before the window exists, which its scope states, so it is a floor for
  cold start rather than time-to-first-frame. Idle CPU after 30 seconds and
  application-level comparisons remain separate workloads.
- Implement file dialogs, external links, clipboard, notifications, and paths.
  **Completed for the path and text-clipboard foundations:** host-only
  per-application `data`,
  `cache`, and `logs` locations derived from a validated identity and the
  current user's Windows Local AppData root (Decision 0021). Filesystem
  access, directory creation, and a public storage protocol remain deferred.
  The first bounded whole-snapshot application-state store now has portable
  values and a direct Windows adapter that stages, flushes, replaces, and
  recovers one host-derived state file (Decision 0051). Protocol 1.10 now
  defines separate state read, replace, and clear grants (Decision 0052).
  Core, SDK, authenticated transport, and the development UI-session diagnostic
  now exercise the complete path. Registered Windows sessions now compose this
  service from the machine-validated application identity before pipe
  authentication; the host-only Windows product-session coordinator now joins
  verified launch, pipe, child exit, and native window shutdown (Decision
  0060). A development-only signed, machine-provisioned fixture now exercises
  that coordinator end to end (Decision 0061); production packaging,
  installation, updates, and non-Windows adapters remain separate work.
  Notifications have a contract and their portable values: Decision 0062
  defines a one-way bounded announce over `Shell_NotifyIconW`, deliberately
  without identifiers, actions, callbacks, or any read surface, and records why
  toast notifications wait for a packaging identity. `anodrel-notifications`
  implements the validated title, body, service boundary, safe failure
  categories, and the one-request UI-thread bridge with its short response
  timeout. `anodrel-windows-notifications` adds the direct Shell32 adapter,
  which keeps one host-owned notification-area entry per process because
  removing an entry also dismisses the balloon it was asked to show. The
  Protocol 1.13 maps the single `notification.show` grant to one operation,
  installed record version 1.3 adds that grant as a strict superset of 1.2, and
  registered interactive sessions carry the mailbox with the host servicing it
  from the owning UI thread. The SDK, mock host, contract tests, and a
  development diagnostic complete the path; confirming a notification is
  visible remains a manual desktop check. Actions, replace, callbacks, toast
  notifications, and non-Windows adapters stay deferred. See
  `docs/NOTIFICATIONS.md`.
  The clipboard is limited to bounded Unicode text through a direct Windows
  adapter and separate Protocol 1.5 read/write grants (Decisions 0040 and
  0041). The external-link foundation accepts only validated HTTPS values and
  hands them directly to the Windows association with no command construction
  through the separate Protocol 1.6 `external.open` grant (Decisions 0042 and
  0043). Rich clipboard formats, consent, subscriptions, custom link schemes,
  and non-Windows adapters remain deferred. The file-dialog foundation defines
  strict portable filters and bounded selected/save-path values (Decisions 0044
  and 0047), with direct Windows open/save adapters and a bounded UI-thread request
  bridge (Decision 0045). Protocol 1.7 grants `dialog.open_file`, and Protocol
  1.8 grants the separate `dialog.save_file`, only through that bridge. Protocol
  1.9 defines a separate `file.read_text` grant and a selection-reference
  result for `dialog.open_file.v2` (Decision 0050). The development Windows UI
  session captures and holds the selected regular-file identity before it
  returns that reference to the authenticated pipe worker (Decision 0049);
  registered interactive-session composition now binds that UI resource to one
  machine-validated application session before authentication. Protocol 1.17
  now adds separate retained-output capture and bounded one-use text writing
  through `dialog.save_file.v2` and `file.write_text` (Decision 0079), while
  Protocol 1.22 adds the distinct 32 KiB binary writer through that same
  one-use retained object (Decision 0087). The legacy save picker stays
  non-mutating. Protocol 1.18 additionally
  implements the bounded `menu.replace` model behind its separate `menu.write`
  grant through the direct Windows menu attachment and delivery path; Protocol
  1.24 adds optional local canonical shortcuts to that same revalidated route.
  Signed product launch,
  atomic replacement, and non-Windows adapters remain deferred.
- Implement secure credential storage through the operating system.
  **Completed for the credential-store foundation:** a host-only Windows
  Credential Manager adapter with per-application target isolation, bounded
  opaque secrets, and current-user local persistence (Decision 0022). Protocol
  1.12 now supplies separately granted exact read, write, and delete operations
  over the authenticated transport, and a development Windows UI-session
  diagnostic proves the direct Credential Manager path. Registered-session
  composition now supplies the same identity-bound service from installed
  policy; consent, signed-launch activation, and non-Windows adapters remain
  deferred.
- Add logging, crash reporting boundaries, and shutdown behavior. **Completed
  for the first in-memory diagnostic log and its bounded authenticated read:**
  a closed event catalogue has no application input, persistence, export, or
  arbitrary error surface; Protocol 1.11 exposes only its fixed records through
  the existing `diagnostics.read` grant (Decision 0053). Windows pipe workers
  also have a host-only pending-I/O stop signal for a later product lifecycle
  owner (Decision 0059). Child-exit lifecycle coordination is now part of the
  product-session owner (Decision 0060). The first crash boundary is panic
  containment at the Win32 callback: an escaping panic would abort the process
  and run no destructor, stranding a tracked child, so a contained panic ends
  the message loop and lets the ordinary drop paths clean up. **Completed for
  the first persisted crash record:** Decision 0065 adds a host-only bounded
  record of a contained panic, written to the host's own `Anodrel\Host\logs`
  location rather than any application's. It carries a closed site and surface
  catalogue, the host version, and a store-assigned sequence — no panic payload,
  path, native status, identifier, or clock. No protocol operation reads,
  writes, or observes one, and a core test asserts that. Eight records are
  retained, reporting failures are silent, and the scope is a contained Rust
  panic only: access violations, stack overflow, `abort`, and hangs are stated
  as out of scope in `docs/CRASH_REPORTS.md` rather than implied to be covered.
  Transmitted reports, a structured-exception handler, and public/application
  logging remain separate work.
- Establish verified executable identity. **In progress:** the direct Windows
  Authenticode adapter verifies an embedded signature and returns a leaf
  certificate fingerprint (Decision 0017). The installed application-record
  foundation now binds the expected executable digest and publisher fingerprint
  to a validated package identity outside the package directory (Decision
  0018). The direct Windows policy adapter now reads that record only from the
  machine-wide 64-bit registry (Decision 0019). The direct launch service locks,
  revalidates, verifies, and tracks a policy-approved executable before
  delivering bootstrap material (Decision 0020), and now also has a
  verification-only entry point that runs the same sequence without creating a
  process. A development-only signed fixture, a controlled provisioning helper,
  the host `--product-session` route, and a preflight-resolved Startup Lab tile
  exercise the whole path on a development machine (Decision 0061). A production
  signing identity, packaging, installation, and updates remain required before
  a shipped application uses it.

Acceptance gate: a sample application can run without Electron and exercise the
core platform services safely.

The direct Windows host creates and paints an Anodrel Win32 window and
validates the core protocol shape under Decision 0006. Decision 0007 adds the
bounded framing and session engine. Decision 0008 adds the authenticated direct
Windows named-pipe adapter. Decision 0009 adds private one-time invitation
delivery. Decision 0010 adds a digest-verified, no-script application-package
text surface. Development-only Node and compiled native samples now prove the
full bootstrap, authentication, and `platform.health` path over the real pipe;
the compiled health probe uses only Anodrel crates, the Rust standard library,
and direct Windows APIs. A separate compiled UI diagnostic now proves one
document replacement, revision-bound semantic action, and session close through
the same owned native window path without Node.js or machine-trust setup.

Decision 0061 adds a first-party signed fixture child that runs the verified
product session end to end: machine policy, locked digest revalidation,
Authenticode publisher match, child-only bootstrap delivery, authenticated
pipe, host-owned native window, one semantic action, and coordinated shutdown.
This is a development-machine fixture, not an installed product: it depends on
a locally generated certificate placed in machine trust, and it says nothing
about packaging, installation, updates, multi-window policy, restart, or
background execution. Remaining acceptance work includes a production signing
and packaging story, a capability bridge, and broader operation-specific native
tests.

## Startup Lab action tiles

The Startup Lab shows every action the platform intends to offer, each in a
declared **linked** or **planned** state (Decision 0014). This table is the
list to work through: a tile moves to linked when its documented host operation
exists and — where it is privileged — has a threat-model entry. Linking a tile
is then a data change plus its operation, not a redesign.

| Tile | State | Gate to link it |
| --- | --- | --- |
| Development Fixture | **Resolved at run time** | Its state is no longer a constant. A verification-only preflight — machine record, locked digest revalidation, Authenticode, publisher fingerprint — decides it before the surface opens. With the development fixture of Decision 0061 provisioned it is live and reads *Development only, not a product*; on any other machine it stays *Not provisioned*, dimmed, and inert. It is deliberately not called a product launch, and there is no product-launch tile: that waits on the deferred signing and packaging decision. |
| Open Logs | **Linked** | Done. Shows only the bounded typed host events defined by `docs/LOGGING.md`; it exposes no application text, persistence, export, or capability. |
| Inspect Package | **Linked** | Done. Displays facts already verified at startup; introduces no capability. |
| Runtime Diagnostics | **Linked** | Done. Displays this process's own readings; introduces no capability. |

Adding a tile beyond these four requires the same treatment: show it planned
with its gate stated, and link it only once its underlying operation is real.

## Phase 3 — Reusable SDK and tooling

Status: **First starter-package slice in progress**

- Provide a small application SDK. **Foundation implemented:** the public
  TypeScript client stays transport-neutral, returns typed protocol results,
  preserves stable host failures, and exposes no native authority. Its public
  contract and development boundary are documented in `docs/SDK.md`; a
  published-package and executable-project template remain later work. A
  first-party native child-client foundation is now specified in Decision 0081:
  its portable bootstrap/authenticated-framing core and direct Windows
  invitation-selected pipe adapter have replaced the product fixture's private
  client implementation, with its real bootstrap-and-pipe test passing. Its
  compiled native development probe now proves bootstrap, authentication, and
  `platform.health` without Node.js. A separate compiled native UI diagnostic
  now proves document delivery, semantic input, and clean session close through
  a host-owned Windows window, still without Node.js or machine-trust setup.
  This is not yet a stable public native SDK or executable template. Decision
  0082 now defines the next development-native UI template boundary. Its typed
  three-operation `anodrel-ui-client` facade is implemented and exercised by
  the compiled native UI diagnostic. The new-directory generator now writes a
  constrained Rust project with only checkout-relative first-party paths and
  proves it through an isolated build plus a real authenticated child session.
  Its explicit Windows host route is also implemented: it grants exactly
  document write, action read, and self-close to an operator-selected
  development executable and shares the compiled probe's bounded lifecycle.
  Decision 0083's typed menu extension is now implemented and locally
  validated: it uses Protocol 1.24 for a strict complete menu model with one
  fixed canonical local shortcut and a bounded mixed event batch, while the
  regular document-only event method fails closed on a menu event. Its explicit
  `init-menu` generator and fixed four-grant Windows host route now expose that
  existing bounded session-menu contract without broadening the regular
  template. A generated child has completed the full authenticated
  shortcut-bearing menu sequence in a real pipe test; manual activation by
  both menu click and keyboard shortcut remains the explicit desktop acceptance
  check.
- Provide development and diagnostic tools. **The first content-package
  native package tool is available:** it creates and verifies only a strict,
  digest-verified `anodrel.text.v1` package in a new directory, never an
  executable, policy record, capability, or signed product (Decisions 0077 and
  0078). See
  `docs/APPLICATION_TEMPLATE.md`.
- Document packaging, signing, updates, and compatibility.
- Add examples for a desktop application and a command-line application.
  **First examples implemented:** the static native text package demonstrates
  host-owned desktop rendering, and the separate command-line example reports
  only public SDK health and capability results through the mock-development
  boundary. Neither claims to be a packaged executable application.

Acceptance gate: a new project can be created from a documented template and
run without knowing the internals of the native host. **Met for the constrained
Windows development template:** the generator, typed client, fixed-grant host
route, isolated release build, and real child-session test are implemented.
Manual window-action verification, production executable identity, a published
stable SDK, packaging, and non-Windows hosts remain separate work.

## Phase 4 — Anodex adapter

Status: **Planned**

- Connect Anodex through the public Anodrel interfaces.
- Keep Anodex's existing Electron adapter working during migration.
- Move platform-specific operations behind the new adapter.
- Compare behavior and performance between the old and new hosts.
- Switch Anodex only after feature parity and recovery procedures are proven.

Acceptance gate: Anodex can run on Anodrel without importing Electron APIs
from its core application logic.

## Phase 5 — Additional applications and platforms

Status: **Planned**

- Add a second sample application.
- Add macOS and Linux host adapters as resources allow.
- Stabilize the protocol and publish SDK documentation.
- Define a long-term release and support policy.

## Explicitly deferred

- **Production signing identity, packaging, installation, and updates.**
  Deferred by decision, not by oversight. Until it is made, the platform has no
  production application identity, so the only thing it can provision is the
  development fixture of Decision 0061, and toast notifications stay out of
  reach because they need an Application User Model ID this platform cannot
  honestly claim. Nothing built so far may be presented as production-ready.
- A full native UI toolkit beyond the constrained foundation in Decision 0025.
- Custom browser engine.
- Custom operating system.
- Full Anodex rewrite before the platform contracts are proven.
