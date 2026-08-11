# Anodrel Initial Threat Model

**Status:** Foundation baseline. Update this document before the native host
accepts untrusted rendered content or adds a privileged capability.

## Scope

This model covers the protocol, SDK, and mock-host foundation. It defines the
controls a Windows native host must satisfy; it does not claim that the current
in-memory mock provides operating-system isolation.

The current operations are `platform.ping`, `platform.capabilities`,
`platform.health`, `ui.document.replace`, `ui.events.read`, `session.close`,
`clipboard.read`, `clipboard.write`, `external.open`, `dialog.open_file`,
`dialog.open_file.v2`, `dialog.save_file`, `file.read_text`,
`storage.state.read`, `storage.state.replace`, `storage.state.clear`, and
`diagnostics.entries.read`, `credential.read`, `credential.write`,
`credential.delete`, and `notification.show`.
Clipboard, external-link, file-dialog, selection-scoped file-text, and
application-state operations each have their own bounded values and separate
host-issued grants. The development UI-session sample exercises these only with
a host-derived test identity; installed-application policy remains separate.

## Assets to protect

- operating-system credentials and application secrets;
- user files, folders, clipboard contents, and external-link destinations;
- process execution authority and child-process lifetime;
- application identity, capability grants, and protocol integrity;
- availability of the host process and native message loop;
- logs and diagnostic reports that may otherwise expose sensitive data.

## Trust boundaries

~~~text
Application UI / rendered content
          |
          | unbound request
          v
Transport adapter and authenticated session
          |
          | host-bound request + host-issued capability context
          v
Anodrel native host ----> Windows APIs, files, processes, credential store
~~~

Rendered content is untrusted with respect to native authority, even when it
belongs to an approved application. Remote data displayed by the application is
also untrusted. The native host and its installed application identity are the
authority for permissions.

## Primary threats and required controls

| Threat | Required control |
| --- | --- |
| Rendered content forges a capability or application identity. | Bind identity and grants to an authenticated host session; never accept them from a request payload. |
| A malformed or future message reaches native code. | Validate the envelope, version, operation, and payload at the transport boundary; return only structured safe errors. |
| An application uses an operation it was not granted. | Check the host capability immediately before executing every privileged operation. |
| A file operation escapes an approved location. | Canonicalize and validate paths after resolving links; enforce a per-operation scope. |
| Application data, cache, or log output crosses an application boundary. | Derive the fixed location only from a host-validated application ID below the current user's Local AppData root; accept no application-supplied absolute path, and do not expose the location through the current protocol. |
| An application-state read, replacement, or clear exceeds its authority or exposes a partial, substituted, or cross-application value. | Keep the first state store to one bounded opaque snapshot per host-validated identity below the host-owned data location; require an independent immediate read, replace, or clear grant; allow no caller path, key, range, or filename; stage and flush a complete replacement before retaining the prior state as a recovery candidate; and expose no content, path, temporary name, recovery source, or native detail in diagnostics. |
| An application reads, replaces, or leaks another application's credential. | Bind one injected credential service to the host-validated application identity; permit only exact restricted names and canonical bounded secret values through separately granted read, write, and delete operations; prohibit arbitrary targets, enumeration, metadata, sharing, and diagnostics or logs carrying secret material. |
| An application reads rich clipboard data, targets another window, or leaks clipboard contents through diagnostics. | Accept only an immediate `clipboard.read` or `clipboard.write` grant from host policy; permit only bounded Unicode text with no format, source, owner, history, or handle selector; return safe categories and never log clipboard text or native failure detail. |
| A link launches a command, file, custom protocol, or leaks a destination through diagnostics. | Validate only bounded ASCII HTTPS links with one DNS-style authority before the native call; pass no verb, parameters, directory, or shell string; return a safe unavailable category and never log the address or native status. |
| A child process gains arbitrary shell authority or outlives the host. | The launch service supplies only the policy-approved `.exe` with no child arguments or shell. The product-session owner retains the child, pipe worker, and window as one lifetime: child exit stops the pipe and closes the window; pipe exit closes the window and terminates the child. Ending that owner always shuts the session down and joins both workers, whether it is ended explicitly or simply dropped. A session that finishes starting after its surface has closed is ended by its own worker, and again by the host once its message loop returns, because a session left waiting for a window would never be dropped and its child would outlive the host. |
| A mutable package substitutes a trusted but unauthorized executable. | Do not treat a package-held manifest or an Authenticode result alone as launch authority; require an external installed application record, lock the contained executable against write/delete/rename, hash it through that lock, then match its verified signer to the record's application-ID-bound publisher fingerprint. |
| A host surface offers a launch that cannot actually be verified, or reveals why it cannot. | Resolve the Startup Lab launch tile from one verification-only preflight — machine record, locked digest revalidation, Authenticode, publisher fingerprint — that creates no process, pipe, or bootstrap material. Drawing, hover, and hit-testing read that single value, so the tile is inert and drawn as planned unless the record and signature validate right now. A failed preflight or a failed start reports only that same planned state: no path, certificate, digest, fingerprint, or Windows error reaches the surface. |
| A development verification fixture becomes a way to weaken production trust. | The fixture satisfies every existing check rather than bypassing one: it is machine-provisioned, digest-locked, signer-matched, argument-free, and granted only `ui.document.write`, `ui.events.read`, and `session.close`. Its identity is a compile-time constant distinct from the shipped sample. Its certificate is generated on the developer's own machine, installed into machine trust only for a development session, and removed by the same script. The native host never creates a certificate, installs trust, writes the registry, or signs anything. See `docs/PRODUCT_FIXTURE.md` and Decision 0061. |
| A provisioning tool writes a machine-policy record the host would reject, or writes one for another application. | Keep record writing in one development helper that the host does not link. Compose the record only from a recomputed executable digest and an Authenticode leaf fingerprint Windows accepted, validate it through the host's own parser before writing, and expose no hive, key path, value name, application ID, or capability argument. |
| An application chooses or substitutes its launch policy. | Read the installed record only from the fixed 64-bit `HKEY_LOCAL_MACHINE` policy location selected by the host; accept no current-user, package, environment, protocol, or UI policy source. Require the registry key, record, and validated package to carry the same application ID. |
| A child process grants itself a capability. | Convert only the validated installed record's strict capability array into the host session policy; reject unknown or duplicate grants, treat version 1.0 records as grant-free, and never accept grants from package, bootstrap, pipe, protocol, or UI data. |
| A notification impersonates another application, spoofs a second message, or becomes a channel back to the application. | Accept only a bounded title and body validated as UTF-16 code units with control characters rejected, so text cannot forge a second message or a source. Keep the notification icon host-owned and generated from the brand crate, so artwork cannot impersonate an identity. Provide no identifier, replace, revoke, callback, or read surface at all, so a notification carries no return path. Report only that the host accepted the values: an application must not be able to observe that the user has silenced, muted, or ignored it. See `docs/NOTIFICATIONS.md` and Decision 0062. |
| Accessibility becomes a channel back to an application, or a way to detect that someone uses assistive technology. | Keep the boundary one-directional: derive UI Automation values from the already-validated semantic snapshot and send them outward only. Provide no tree read, focus query, announcement callback, or presence signal, so an application cannot learn that a screen reader is running any more than it can learn a notification was seen. An application supplies a UI document and cannot pass a handle, see a UI Automation identifier, register a provider, raise an event, force focus, or override a mapping. See `docs/ACCESSIBILITY.md` and Decision 0063. |
| A secret reaches the renderer or logs. | Use operating-system credential storage; redact secrets, raw native errors, and sensitive paths from protocol diagnostics and logs. |
| A host diagnostic log captures untrusted or sensitive data. | The first log accepts only a closed typed host-event enum; it has no dynamic message, payload, path, error, credential, persistence, export, or protocol input. |
| An application uses diagnostics to obtain arbitrary host data or an unbounded event stream. | `diagnostics.entries.read` requires the existing immediate `diagnostics.read` grant, accepts only `{}`, returns at most 64 records from the closed typed catalogue, and exposes no filter, cursor, time, native detail, write, clear, export, or subscription operation. |
| Message floods exhaust CPU or memory. | Reject frames above 64 KiB before decoding, reject more than four complete frames in one receive burst, process input in arrival order, and retain at most 32 unresolved pre-execution cancellation IDs; an invalid or overflowing cancellation control closes the session. |
| A local or remote process connects to the endpoint by guessing its name. | Create one random-suffix pipe, restrict its DACL to the current logon SID, require a 32-byte CNG-generated token as the first frame, and close on any failed handshake. |
| A session invitation reaches another process or durable diagnostic. | Deliver one bounded record only through a child-only inherited standard-input handle; never put it in arguments, environment, logs, telemetry, crash data, or files. |
| Content path traversal or a symlink escapes a package. | Canonicalize the manifest directory and declared content file, reject root/prefix and dot path components, and require the resolved content to remain below the package root. |
| Package content is changed after its manifest is created. | Read bounded bytes and verify the manifest's lowercase SHA-256 digest before exposing text to the host surface. |
| Rendered text acquires script or native authority. | Support only `anodrel.text.v1`: bounded UTF-8 plain text with no scripts, navigation, URLs, resource loads, or native bridge. |
| A future application supplies malformed or oversized UI data. | Decode only the exact `anodrel.ui.document.v1` schema through the 64 KiB strict JSON boundary and validate every existing UI model limit before returning a document. The host accepts an external document only through its separate explicit bounded developer preview command, never an application session. |
| An operator-selected preview file creates broader host access. | The preview opens only one bounded regular UTF-8 file named directly on the local command line, validates it before window creation, and loads no companion file, package, policy, session, asset, executable, URL, or native capability. |
| A late input event targets a replaced UI document. | Bind every accepted semantic action to the exact monotonic document revision that produced its layout; reject events for an empty, replaced, removed, or disabled action. The current state crate has no I/O or application delivery path. |
| An authenticated application overwhelms or corrupts its UI session. | Require the host-issued `ui.document.write` grant immediately before `ui.document.replace`; limit its encoded document string to 24 KiB within the 64 KiB wire message; use the strict v1 codec and atomic replacement; expose only a revision string and retain the prior state on failure. A separate `ui.events.read` grant provides no document readback or native authority. |
| UI replacement traffic builds an unbounded cross-thread backlog. | Transfer accepted snapshots only through one per-session mailbox slot. A newer revision replaces the pending older revision; no update callback, pipe I/O, renderer work, or semantic event enters that slot. |
| A pipe worker manipulates a native window or a session document drives another window. | Give a UI Session Lab exactly one mailbox and poll it only from its own Windows UI thread; accept only a newer revision into that one view. The pipe worker never calls User32, and session documents have no window-selection field. |
| A pipe worker opens a modal native dialog or queues unbounded dialog work. | Keep each file-dialog request in one per-session `FileDialogMailbox`; only the owning UI thread can take and complete it. Allow one pending or displayed request, no history, and a fixed two-minute safe failure. The request carries strict filters only: no window handle, initial directory, raw flags, or filesystem authority. `dialog.open_file` and `dialog.save_file` require distinct immediate grants; either returned path is data only, and save selection never creates, truncates, or writes. |
| A selected path becomes arbitrary-file access or resolves to a replacement file. | Protocol 1.9 `file.read_text` accepts only a one-use unguessable reference created by the host for the same authenticated session, never a caller-supplied path. The Windows UI-session host opens and validates the Windows-confirmed regular file on its UI thread before returning `dialog.open_file.v2`; it retains a read-only object that blocks later write/delete/rename sharing, rejects reparse points, and is consumed once. The read has its own immediate grant, fixed 8 KiB strict-UTF-8 result bound, and safe failure categories. |
| A stale or forged UI action reaches application logic. | The UI thread queues only a revision and action ID from its current host-rendered layout. `ui.events.read` checks the host-issued `ui.events.read` grant, then revalidates each candidate against the current session document and enabled action before delivery; stale or unavailable candidates are counted and discarded. |
| UI input exhausts memory or silently loses state. | Keep a per-session queue of 32 candidates. Drop newer candidates only when full and report the exact dropped count on the next `ui.events.read`; return a separate discarded count for stale or unavailable actions. |
| An application closes another window or turns a close request into process control. | Accept `session.close` only from the authenticated session carrying its host-issued `session.close` grant. Carry no target or native handle, coalesce it into one host-owned signal, and let the host UI or lifecycle owner decide and perform cleanup. |
| Two host invocations race to display one package identity. | Claim a current-session mutex from the validated application ID; a secondary waits at most one second and can only issue a no-data best-effort activation request. |
| A same-session process signals or reserves an instance object. | Treat the instance channel as local coordination only: it carries no payload or authority and returns a safe failure instead of creating a second window when readiness cannot be established. |
| Two native windows render each other's state or one close ends the host early. | Keep immutable host-created views in a handle-keyed registry and exit the UI loop only after the final registered window is destroyed. |
| A host defect aborts the process and strands a tracked child. | The window procedure is `extern "system"` and does not unwind, so an escaping panic would abort and run no destructor, leaving a verified product child with no host. Contain each window message: a panic ends the message loop, the host clears every remaining view, and the ordinary drop paths shut down the child, join its workers, and remove any notification entry. The payload is discarded, never inspected, so nothing derived from a panic reaches a response, the ledger, or an application. The host does not resume afterwards. |
| An application harvests what a person is typing before they decide to send it. | A text field's value, caret, and selection are owned by the host, and keyboard input is handled on the host UI thread and never leaves it as input. An application supplies an initial value in its document and can otherwise obtain only a **snapshot** of the current text through a separate granted operation — there is no change event, no subscription, and no keystroke or timing information, because each of those delivers the typing rather than the value. Someone who types, deletes, and retypes has told the application one thing: the final text. The host draws and handles the field itself, so there is no native edit control, window handle, or message hook to reach, and pasting is the person's action into the host's buffer rather than a `clipboard.read` grant. There is no masked or password field: masking pixels while the value crosses the protocol as ordinary text would be a promise the platform cannot keep, so secrets stay with `docs/CREDENTIALS.md`. See `docs/UI_FIELDS.md` and Decision 0067. |
| An application titles its window to impersonate another application or the operating system. | A window title appears in the task switcher, taskbar, window lists, screen-reader announcements, and screenshots — where a person decides what they are talking to. The application proposes only part of it: the host appends `— <display name>` from the machine-validated installed record after validation, so a proposal can neither suppress nor forge it, and `Windows Security` renders as `Windows Security — Anodrel Sample`. The proposal is bounded to 96 UTF-16 units and rejects every control character, including a line feed, so it cannot split one title into two or push the visible text away from the host's suffix. The request names no window, handle, or target — the host resolves the window from the authenticated session — so it cannot be aimed at another session, application, or host surface. `window.title.set` needs the separate `window.title` grant at protocol 1.14, is write-only, and no failure echoes the proposed text. See `docs/WINDOW_TITLE.md`. |
| A record of a host defect becomes application-visible information or leaks its payload. | A crash record holds only a closed site and surface catalogue, the host version, and a process-local sequence — no panic payload, path, native status, identifier, or clock. It is written to the host's own `Anodrel\Host\logs` location, never an application's, and no protocol operation reads, writes, or observes one; a test asserts no operation names one. Retention is bounded at 8 records, a record is capped at 512 bytes and refused rather than truncated, and every failure to report is silent so a handled defect cannot become an unhandled one. It covers a contained Rust panic only, which `docs/CRASH_REPORTS.md` states as a limitation rather than implying coverage it does not have. |

## Security invariants

1. The SDK cannot grant itself authority. Only the host policy can issue a
   capability context.
2. Application UI code has no raw operating-system API or arbitrary host bridge.
3. Each privileged operation has a documented capability, validation rule,
   failure behavior, cleanup behavior, and contract test.
4. Native diagnostics remain host-controlled and protocol diagnostics contain
   safe metadata only.
5. Cancellation and shutdown leave no untracked child process, incomplete write,
   or elevated operation running in the background.

## Performance and availability baseline

The core protocol uses small JSON-compatible envelopes and does no I/O during
module import. It validates once at the host boundary and keeps SDK, mock-host,
and protocol packages independently buildable.

The wire codec rejects encoded messages larger than 64 KiB before UTF-8
or JSON parsing and stops a receive burst after four complete frames. The core
then rejects duplicate JSON keys, malformed Unicode, trailing bytes, and nesting
beyond 64 levels. The session engine owns host-created capability policy, but
does not listen on an OS endpoint. The current Win32 host renders only an
internal startup response and has no webview, application bridge, or inbound
transport. Before it opens its branded Startup Lab diagnostic window, the host
does run one temporary in-process named-pipe loopback: the current-session DACL,
CNG invitation, first-message authentication, framing, and health policy are
exercised without rendering a secret or accepting an application client. The
one-client named-pipe adapter restricts access to the current logon SID and
requires a CNG-generated session token. Its separate bootstrap
adapter sends that invitation once over a child-only inherited standard-input
handle, with output handles redirected to `NUL`; it does not establish
executable identity. The Windows host separately establishes a content identity
by parsing a strict 16 KiB manifest, canonicalizing and containing the declared
file, checking its built-in SHA-256 digest, and drawing only bounded plain text.
This detects content tampering but does not authenticate a publisher who can
replace the entire unpackaged directory. The surface has no queue, origin,
navigation, native bridge, or executable trust; those controls remain required
before untrusted application code or a privileged operation is accepted.

The package text surface also uses a current-session instance mutex and
readiness event derived from its validated application ID. A second invocation
cannot forward arguments or application data; it only sends a bounded
best-effort User32 activation message after the primary has created its window.
This preserves one-window coordination without treating the named object as an
identity or authorization mechanism.

The direct window host routes painting through its own handle-keyed registry;
the current package and Startup Lab surfaces cannot supply entries to it. A
failed multi-window creation is rolled back before the message loop starts, and
closing one window does not end the host while another registered window exists.

## Rendering boundary

First-party surfaces are composed by portable crates that forbid unsafe
code and read nothing but the values passed to them. Rendering is a pure
function from host-supplied data to pixels: it opens no handle, reads no file,
and reaches no operating-system API. The host's drawing seam is narrow by
construction — a bitmap blit, a private memory device context used only to
rasterize glyphs the host itself chose, and a display-density query.

Application text reaching a surface is laid out, never interpreted. Content
arrives as opaque paragraphs that are measured and wrapped; no character carries
markup, link, or script meaning, and no glyph run can address anything outside
the canvas it is composited into.

The window layer receives a copy of the display-safe facts about a validated
package rather than the package itself. It therefore cannot reach a canonical
filesystem path, an unvalidated manifest field, or content that failed a digest
check. A manifest's declared relative content path may be displayed; the
absolute path it resolves to may not.

Diagnostic readings shown on a surface are limited to measurements this process
can take about itself. Interactive tiles that are not backed by an existing
capability carry a declared pending state that both drawing and hit-testing
read, so a control cannot be made live by changing how it looks. Any tile that
would carry a privileged capability — process launch in particular — requires an
entry in this document before it becomes active.

The host diagnostic log is equally constrained. It is a bounded in-memory
catalogue of typed startup events, not a string sink. Its document view receives
only sequence, severity, component, and fixed event text; no application value,
native error, path, invitation, credential, or request data can enter that
boundary. `docs/LOGGING.md` and Decision 0016 define the catalogue and its
extension gate.

The development sample exercises this private path with a developer-supplied
Node.js executable and an Anodrel sample script. It has no executable identity
verification and ends with the host process, so it creates no production
application-launch authority. Its output is intentionally discarded; an exit
status is the only result used by the host.

Decision 0017 adds a direct Windows Authenticode verification primitive. It
returns the leaf signing certificate fingerprint only after Windows accepts the
embedded signature, and it deliberately does not launch a process or declare a
package trusted. Decision 0018 defines a strict installed application record
outside the package that binds an application ID, executable digest, and
approved signer fingerprint. Decision 0019 adds the fixed, read-only,
machine-wide Windows registry source for that record; it rejects current-user
and fallback sources. Decision 0020 adds locked pre-launch revalidation,
tracked child lifetime, and post-verification bootstrap binding.

Decision 0061 adds the development-only signed fixture that finally exercises
that path. The Startup Lab's Development Fixture tile is now resolved from a
verification-only preflight rather than a compile-time constant, so it exists
only while a machine record and signed executable validate. This is a
development-machine capability: it depends on a locally generated certificate
installed into machine trust, which is a development-environment assumption and
not a production publisher identity. Production packaging, installation,
updates, and a real signing identity remain separate gates before any shipped
application uses this path.

## Before additional privileged capabilities

The native-host decision must extend this model with:

- a signed package and verified executable identity model;
- session-authentication binding and bridge bootstrap details for any future
  executable application;
- the Windows named-pipe session bootstrap, authentication, and I/O scheduling;
- the Windows permission, filesystem, process, credential-store, and update
  assumptions; and
- tests for traversal, malformed input, capability bypass, shutdown races, and
  overload handling.

No application-facing filesystem, process, credential, dialog, notification,
or external-link operation may be implemented until its contract and these
host-specific controls are documented and tested. `docs/NOTIFICATIONS.md` and Decision 0062 now supply the notification contract,
and its portable values, UI-thread bridge, Windows adapter, Protocol 1.13
operation, and record version 1.3 grant are implemented and tested. It is a
one-way announce with no read surface, so it adds no way for an application to
observe the user. The bounded text clipboard,
validated external links, and UI-thread-routed open-file dialog are the current
application-facing exceptions: `docs/CLIPBOARD.md`, `docs/EXTERNAL_LINKS.md`,
and `docs/FILE_DIALOGS.md` define their separate controls.
Decisions 0040 through 0048, capability checks, portable and native-boundary
tests, and authenticated transport coverage define and verify their
development-session exposure. Production executable trust, consent, and richer
platform features remain separate gates.
