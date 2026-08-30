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
`dialog.open_file.v2`, `dialog.save_file`, `dialog.save_file.v2`,
`dialog.open_folder`,
`file.read_text`, `file.write_text`, `file.write_binary`,
`storage.state.read`, `storage.state.replace`, `storage.state.clear`, and
`diagnostics.entries.read`, `credential.read`, `credential.write`,
`credential.delete`, and `notification.show`.
Protocol 1.18 implements the portable `menu.replace` boundary and its direct
Windows menu attachment and activation delivery. Manual verification of the
development-only real-menu diagnostic remains pending.
Decision 0120 records the Protocol 1.32 contract for a separate host-owned
semantic context menu. Its portable model and mailbox are implemented, but no
public operation or Windows surface exists yet. It carries no browser
selection, link, coordinate, callback, or native-handle boundary.
Protocol 1.19 implements the portable host-authorized HTTPS text-fetch core,
SDK, mock-host contract, and direct WinHTTP adapter in `docs/NETWORK.md`. One
explicit Windows development diagnostic attaches it to a compiled
`example.com:443` policy and gives an operator-selected child only the
`network.fetch` grant. Decision 0099 also permits a registered installed
application session to receive the same service only when a version 1.14
machine record grants `network.fetch` and supplies one through eight exact
origins; templates and the product fixture remain excluded.
Protocol 1.20 implements the separately granted session-window foreground
request through one UI-thread bridge and direct `SetForegroundWindow`; it has
no target, focus observation, input, retry, or cross-window path. The manual
Windows foreground-policy diagnostic remains an explicit acceptance check.
Protocol 1.21 implements separately granted reversible session-window
fullscreen through one UI-thread bridge. It uses only the monitor Windows
associates with that host-selected window and retains its native style and
placement privately; it has no target, monitor selection, display control,
geometry, fullscreen-state observation, or cross-window path. The manual
Windows entry-and-restore diagnostic remains an explicit acceptance check.
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
| A selected folder path becomes arbitrary directory access, or a selected folder is replaced before a later enumeration. | Keep Protocol 1.28 `dialog.open_folder` display-only. The separately specified Protocol 1.29 companion may create a one-use `FolderReference` only after a host adapter captures a non-reparse directory handle that blocks replacement, rename, and deletion. A distinct `folder.read_entries` grant may consume it for at most 32 immediate entry names and safe kinds; it accepts no path, traversal, child selector, cursor, or write request. The adapter enumerates directly from that retained handle rather than reopening a path; a reparse point, unknown reference, or native failure becomes only `folder.unavailable`. |
| Application data, cache, or log output crosses an application boundary. | Derive the fixed location only from a host-validated application ID below the current user's Local AppData root; accept no application-supplied absolute path, and do not expose the location through the current protocol. |
| An application-state read, replacement, or clear exceeds its authority or exposes a partial, substituted, or cross-application value. | Keep the first state store to one bounded opaque snapshot per host-validated identity below the host-owned data location; require an independent immediate read, replace, or clear grant; allow no caller path, key, range, or filename; stage and flush a complete replacement before retaining the prior state as a recovery candidate; and expose no content, path, temporary name, recovery source, or native detail in diagnostics. |
| An application reads, replaces, or leaks another application's credential. | Bind one injected credential service to the host-validated application identity; permit only exact restricted names and canonical bounded secret values through separately granted read, write, and delete operations; prohibit arbitrary targets, enumeration, metadata, sharing, and diagnostics or logs carrying secret material. |
| An application reads rich clipboard data, targets another window, or leaks clipboard contents through diagnostics. | Accept only an immediate `clipboard.read` or `clipboard.write` grant from host policy; permit only bounded Unicode text with no format, source, owner, history, or handle selector; return safe categories and never log clipboard text or native failure detail. |
| A link launches a command, file, custom protocol, or leaks a destination through diagnostics. | Validate only bounded ASCII HTTPS links with one DNS-style authority before the native call; pass no verb, parameters, directory, or shell string; return a safe unavailable category and never log the address or native status. |
| A network request probes arbitrary, redirected, credentialed, or browser-state destinations; leaks response data or OS diagnostics; or leaves native network handles behind. | The implemented portable boundary accepts only an explicit `network.fetch` grant, a strict HTTPS DNS URL with no user information, IP literal, fragment, method, body, header, cookie, credential, proxy, redirect, timeout, certificate, callback, or application-selected policy field, and an exact host-created origin set. The compiled diagnostic creates one fixed `example.com:443` policy before authentication, grants only `network.fetch`, and its first-party child requests only compiled `https://example.com/` without printing the response. A registered installed session receives the service only when its machine-selected version 1.14 record carries both `network.fetch` and one through eight unique exact `{host, port}` origins; an empty list without the grant is required, and every other record version rejects both the grant and the field. A package, protocol message, rendered UI, template, or fixture cannot select or read that policy. The direct WinHTTP adapter uses fixed phase timeouts; disables proxy discovery, automatic authentication, cookies, redirects, and keep-alive; retains normal TLS plus revocation checking; bounds UTF-8 reads; and gives every handle one RAII owner. Its origin mismatch or native failure maps only to `network.unavailable`; response bytes, URL, headers, certificate, address, proxy, timing, and native status never enter host diagnostics. See `docs/NETWORK.md` and Decisions 0084 and 0099. |
| A child process gains arbitrary shell authority or outlives the host. | The launch service supplies only the policy-approved `.exe` with no child arguments or shell. The product-session owner retains the child, pipe worker, and window as one lifetime: child exit stops the pipe and closes the window; pipe exit closes the window and terminates the child. Ending that owner always shuts the session down and joins both workers, whether it is ended explicitly or simply dropped. A session that finishes starting after its surface has closed is ended by its own worker, and again by the host once its message loop returns, because a session left waiting for a window would never be dropped and its child would outlive the host. |
| A mutable package substitutes a trusted but unauthorized executable. | Do not treat a package-held manifest or an Authenticode result alone as launch authority; require an external installed application record, lock the contained executable against write/delete/rename, hash it through that lock, then match its verified signer to the record's application-ID-bound publisher fingerprint. |
| A host surface offers a launch that cannot actually be verified, or reveals why it cannot. | Resolve the Startup Lab launch tile from one verification-only preflight — machine record, locked digest revalidation, Authenticode, publisher fingerprint — that creates no process, pipe, or bootstrap material. Drawing, hover, and hit-testing read that single value, so the tile is inert and drawn as planned unless the record and signature validate right now. A failed preflight or a failed start reports only that same planned state: no path, certificate, digest, fingerprint, or Windows error reaches the surface. |
| A development verification fixture becomes a way to weaken production trust. | The fixture satisfies every existing check rather than bypassing one: it is machine-provisioned, digest-locked, signer-matched, argument-free, and granted only `ui.document.write`, `ui.events.read`, and `session.close`. Its identity is a compile-time constant distinct from the shipped sample. Its certificate is generated on the developer's own machine, installed into machine trust only for a development session, and removed by the same script. The native host never creates a certificate, installs trust, writes the registry, or signs anything. See `docs/PRODUCT_FIXTURE.md` and Decision 0061. |
| A provisioning tool writes a machine-policy record the host would reject, or writes one for another application. | Keep record writing in one development helper that the host does not link. Compose the record only from a recomputed executable digest and an Authenticode leaf fingerprint Windows accepted, validate it through the host's own parser before writing, and expose no hive, key path, value name, application ID, or capability argument. |
| An application chooses or substitutes its launch policy. | Read the installed record only from the fixed 64-bit `HKEY_LOCAL_MACHINE` policy location selected by the host; accept no current-user, package, environment, protocol, or UI policy source. Require the registry key, record, and validated package to carry the same application ID. |
| A child process grants itself a capability. | Convert only the validated installed record's strict capability array into the host session policy; reject unknown or duplicate grants, treat version 1.0 records as grant-free, and never accept grants from package, bootstrap, pipe, protocol, or UI data. |
| A notification impersonates another application, spoofs a second message, or becomes a channel back to the application. | Accept only a bounded title and body validated as UTF-16 code units with control characters rejected, so text cannot forge a second message or a source. Keep the notification icon host-owned and generated from the brand crate, so artwork cannot impersonate an identity. Provide no identifier, replace, revoke, callback, or read surface at all, so a notification carries no return path. Report only that the host accepted the values: an application must not be able to observe that the user has silenced, muted, or ignored it. See `docs/NOTIFICATIONS.md` and Decision 0062. |
| Accessibility becomes a channel back to an application, a way to detect that someone uses assistive technology, or a shortcut around UI-action validation. | Keep accessibility semantics outbound only: derive UI Automation values from the already-validated semantic snapshot and provide no tree read, announcement callback, or presence signal, so an application cannot learn that a screen reader is running any more than it can learn a notification was seen. `GetFocus` and `HasKeyboardFocus` report only a provider's host-owned snapshot to Windows; they never enter the protocol or read live registry state. `SetFocus` can only travel through the separate bounded, revision-checked host route in Decision 0073; it has no application operation, readback, native input, or activation effect. After a genuine host focus change, Decision 0074 raises one best-effort focus event from a fresh immutable provider, without recording listeners or surfacing delivery. Decision 0100 lets a version-3 document carry at most one visible semantic status; only a changed visible status in an already-established authenticated session view raises one best-effort live-region event from a fresh provider. Initial, unchanged, removed, clipped, diagnostic, and preview statuses are silent. Its result, listener presence, and delivery never reach an application. A visible field's plain v1 text is copied only into an immutable read-only UIA provider snapshot; it never enters the protocol, supports `SetValue`, exposes caret or selection data, or permits secret fields. The sole action path is `IInvokeProvider` on an enabled authenticated-session button: it offers one revision-bound `ActionInvoked` candidate to the existing 32-slot session mailbox, then the granted `ui.events.read` route revalidates it against the current document. It has no native message, focus movement, application callback, or queue-state surface; every other role and diagnostic surface has no Invoke pattern. An application cannot pass a handle, see a UI Automation identifier, register a provider, raise or receive an event, force focus, or override a mapping. See `docs/ACCESSIBILITY.md`, `docs/UI_LIVE_ANNOUNCEMENTS.md`, and Decisions 0063, 0069, 0070, 0071, 0073, 0074, and 0100. |
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
| A save path becomes arbitrary write authority, a selected target is replaced, or an unused new filename leaves a file behind. | Protocol 1.17 `dialog.save_file.v2` returns a one-use opaque `saveReference`, never a writable path. The Windows UI-thread capture retains and validates the exact regular output object before returning it, while the legacy save path remains data only. An existing target is untouched until the separate `file.write_text` grant consumes its retained handle. A new target is created only through create-new, marked for deletion while unused, and removed through that same handle at reference or session cleanup. The write accepts no path or mode and has a fixed 8 KiB text bound. It is explicitly non-atomic: native failure after mutation can leave partial output, so atomic replacement remains a separate future design. |
| A binary export becomes an alternate filesystem, transfer, or raw-byte authority; a second representation bypasses one-use output selection. | Protocol 1.22 requires the separate `file.write_binary` grant and accepts only a host-issued `saveReference` plus canonical unpadded base64url that decodes to at most 32 KiB. The portable first-party decoder rejects padding, whitespace, malformed length, alternate trailing bits, and every non-base64url character; representations above the decoded bound return only `file.binary_too_large`. After authorization, malformed or oversized data still consumes the retained reference, so a text and binary operation cannot race to reuse it. The Windows adapter receives only bounded decoded bytes and writes through the captured handle; it accepts no path, type, length claim, handle, offset, stream, progress, readback, or native failure disclosure. See `docs/FILE_BINARY_WRITE.md` and Decision 0087. |
| A stale or forged UI action reaches application logic. | The UI thread queues only a revision and action ID from its current host-rendered layout. `ui.events.read` checks the host-issued `ui.events.read` grant, then revalidates each candidate against the current session document and enabled action before delivery; stale or unavailable candidates are counted and discarded. |
| An application learns pointer movement or treats scrolling as a command. | The direct Windows scrollbar derives its track and thumb only from the current host layout metric and retained offset. Track clicks and captured thumb movement update only that offset on the owning UI thread; they do not focus an element, edit a field, queue a semantic action, change a revision, or emit an event. The document supplies neither scrollbar style nor a target, and the protocol carries no position, pointer, drag, or scroll message. See `docs/SCROLLING.md` and Decision 0096. |
| An automation caller scrolls a different or stale viewport, reveals an unrelated or nested item, turns a provider into input authority, or reports a person's position to an application. | Decision 0097 exposes `IScrollProvider` only for the current first visible overflowing scroll group, carrying its semantic ID and immutable provider revision through one 250 ms host-owned request slot. Decision 0098 permits `IScrollItemProvider` only for a bounded descendant whose nearest semantic scroll ancestor is that same group; its request additionally carries that fixed semantic item ID, never an offset or alignment. A payload-free private wake message carries no pointer, position, handle, document, or application data. The owner UI thread takes the request once, rechecks the same revision, first-overflowing metric, permitted nearest descendant, and current layout, then changes only the existing retained scroll state. A timeout clears the exact slot before a late completion can change it. There is no protocol operation, grant, event, focus move, semantic action, field value, position readback, automation-presence signal, horizontal target, nested target, or application callback. See `docs/UI_AUTOMATION_SCROLL.md` and `docs/UI_AUTOMATION_SCROLL_ITEMS.md`. |
| A menu model gains native window, shell, command, or keyboard authority; a stale command is delivered; or menu interaction creates an unbounded event path. | Protocol 1.18 `menu.replace` requires its own `menu.write` grant and validates only a bounded complete model of labels and enabled semantic IDs. Protocol 1.24 optionally accepts one unique canonical local `Ctrl+<A-Z0-9>` or `Ctrl+Shift+<A-Z0-9>` declaration per action; it never accepts a native command number, virtual key code, target, global hotkey, `Alt` mnemonic, callback, payload, system-menu, context-menu, or submenu field. Its core state commits only after the host menu service accepts the complete replacement; an unattached service returns `menu.unavailable`. The Windows UI thread owns every private command ID and accepts a click only from the documented normal-menu `WM_COMMAND` shape and a current mapped ID. It considers a shortcut only for the first ordinary key-down in its active session window, with Control down, exactly the declared Shift state, and Alt absent; it registers no system-wide hotkey and emits no keyboard data. Both routes place the same menu-revision candidate into the existing bounded ordered interaction mailbox. The existing `ui.events.read` grant revalidates that revision and enabled command before returning a `menu.action.invoked` event; disabled, replaced, or removed commands are discarded. |
| A context popup becomes a browser-data, pointer-telemetry, native-window, or callback bridge. | The accepted Protocol 1.32 contract has a distinct `menu.context.write` grant and accepts only one bounded complete set of semantic IDs, labels, and enabled flags. The host, not the application, chooses whether to show it and derives placement only from a pointer-originated local `WM_CONTEXTMENU` for the session's primary view. No coordinate, target element, selection, link, URL, menu handle, command ID, callback, keyboard state, menu opening/dismissal, or result readback crosses the boundary. The UI thread owns the popup and private mapping; an enabled choice enters the existing fixed interaction mailbox as a revision-bound candidate, and `ui.events.read` later revalidates it before returning only an action ID and opaque context-menu revision. See Decision 0120 and `docs/CONTEXT_MENUS.md`. |
| An automation caller focuses another view, turns a stale provider into native input, or uses a window message to inject focus. | `IRawElementProviderFragment::SetFocus` exists only for a visible enabled focusable child in its own immutable provider snapshot. Its route carries only that semantic ID and, for an authenticated session, the exact document revision; the owner UI thread revalidates both against its current layout. A one-request 250 ms route is per window, releases its exact slot on timeout before it can apply focus, and ignores late completion. Its private wake message carries no pointer, ID, text, or input data, so an externally posted copy can only ask a view to inspect its own pending request. A real focus transition then raises an outbound event only from a fresh current provider; it neither activates a window nor sends input, and applications have no focus operation, readback, event, or accessibility-presence signal. See `docs/UI_AUTOMATION_FOCUS.md`, `docs/UI_AUTOMATION_EVENTS.md`, and Decisions 0073 and 0074. |
| UI input exhausts memory or silently loses state. | Keep a per-session queue of 32 candidates. Drop newer candidates only when full and report the exact dropped count on the next `ui.events.read`; return a separate discarded count for stale or unavailable actions. |
| An application closes another window or turns a close request into process control. | Accept `session.close` only from the authenticated session carrying its host-issued `session.close` grant. Carry no target or native handle, coalesce it into one host-owned signal, and let the host UI or lifecycle owner decide and perform cleanup. |
| An application steals focus, observes another application's foreground state, or turns a focus request into input authority. | Require the separate `window.focus` grant and exact empty Protocol 1.20 payload. Resolve the target only from the requesting authenticated session, transfer it through one five-second UI-thread mailbox, and call `SetForegroundWindow` only for that host-owned window. Windows may refuse; map refusal, timeout, and no associated window only to `window.unavailable`, return no resulting focus or prior-foreground data, and provide no target, input, callback, retry, `AllowSetForegroundWindow`, or accessibility path. See `docs/WINDOW_FOCUS.md` and Decision 0085. |
| An application gains arbitrary desktop or display control through fullscreen, loses a window's restoration facts, or learns topology from the result. | Require the separate `window.fullscreen` grant and exact one-field Protocol 1.21 payload with only `fullscreen` or `windowed`. Resolve the target only from the requesting authenticated session and transfer it through one five-second UI-thread mailbox. The Windows adapter chooses the monitor only from the known host window, stores its original style and placement only beside that session view, applies borderless **windowed** fullscreen rather than exclusive display control, and restores with the matching placement API. No target, handle, monitor, coordinate, geometry, display mode, topmost flag, visibility, state readback, event, callback, shortcut, or retry crosses the boundary. Missing surface, timeout, and native failure map only to `window.unavailable`; a concurrent request maps only to `window.busy`. See `docs/WINDOW_FULLSCREEN.md` and Decision 0086. |
| A size request becomes a route to move, target, inspect, or control the desktop, or corrupts a fullscreen restore. | Protocol 1.23 `window.size.set` accepts only bounded logical client width and height behind its separate `window.size` grant. The host resolves the window only from the authenticated session, converts at the known window's current DPI, and resizes without moving, activating, changing z-order, selecting a monitor, or returning geometry. It has no target, position, DPI, bounds, state, event, callback, constraint, animation, or readback field. A request while Anodrel fullscreen is active returns only `window.unavailable`, preserving that private restore state. See `docs/WINDOW_SIZE.md` and Decision 0088. |
| Two host invocations race to display one package identity. | Claim a current-session mutex from the validated application ID; a secondary waits at most one second and can only issue a no-data best-effort activation request. |
| A same-session process signals or reserves an instance object. | Treat the instance channel as local coordination only: it carries no payload or authority and returns a safe failure instead of creating a second window when readiness cannot be established. |
| Two native windows render each other's state or one close ends the host early. | Keep immutable host-created views in a handle-keyed registry and exit the UI loop only after the final registered window is destroyed. |
| A host defect aborts the process and strands a tracked child. | The window procedure is `extern "system"` and does not unwind, so an escaping panic would abort and run no destructor, leaving a verified product child with no host. Contain each window message: a panic ends the message loop, the host clears every remaining view, and the ordinary drop paths shut down the child, join its workers, and remove any notification entry. The payload is discarded, never inspected, so nothing derived from a panic reaches a response, the ledger, or an application. The host does not resume afterwards. |
| An application rebuilds a keystroke stream by polling for values. | `ui.fields.read` takes an empty payload and returns every field on the current surface at once. There is no selector, so a caller cannot narrow a read to one field and repeat it: each read costs the same and returns the same shape, which removes the gain from reading often. The result carries the values only — no caret, selection, character count, timestamp, or edited/focused/touched flag — so an empty field and an untouched field are indistinguishable, because the difference between them is behaviour rather than content. The operation needs the separate `ui.fields.read` grant at protocol 1.15, answers only for the requesting session's own current document, and has no change event or subscription counterpart. See `docs/UI_FIELDS.md` and Decision 0067. |
| An application harvests what a person is typing before they decide to send it. | A text field's value, caret, and selection are owned by the host, and keyboard input is handled on the host UI thread and never leaves it as input. An application supplies an initial value in its document and can otherwise obtain only a **snapshot** of the current text through a separate granted operation — there is no change event, no subscription, and no keystroke or timing information, because each of those delivers the typing rather than the value. Someone who types, deletes, and retypes has told the application one thing: the final text. The host draws and handles the field itself, so there is no native edit control, window handle, or message hook to reach, and pasting is the person's action into the host's buffer rather than a `clipboard.read` grant. There is no masked or password field: masking pixels while the value crosses the protocol as ordinary text would be a promise the platform cannot keep, so secrets stay with `docs/CREDENTIALS.md`. See `docs/UI_FIELDS.md` and Decision 0067. |
| An application titles its window to impersonate another application or the operating system. | A window title appears in the task switcher, taskbar, window lists, screen-reader announcements, and screenshots — where a person decides what they are talking to. The application proposes only part of it: the host appends `— <display name>` from the machine-validated installed record after validation, so a proposal can neither suppress nor forge it, and `Windows Security` renders as `Windows Security — Anodrel Sample`. The proposal is bounded to 96 UTF-16 units and rejects every control character, including a line feed, so it cannot split one title into two or push the visible text away from the host's suffix. The request names no window, handle, or target — the host resolves the window from the authenticated session — so it cannot be aimed at another session, application, or host surface. `window.title.set` needs the separate `window.title` grant at protocol 1.14, is write-only, and no failure echoes the proposed text. See `docs/WINDOW_TITLE.md`. |
| An application manipulates another window, learns host topology, or turns a presentation request into arbitrary native-window control. | `window.state.set` accepts only `minimized`, `maximized`, or `restored`, with an exact one-field payload and a separate `window.state` grant at protocol 1.16. The request names no target, handle, identifier, geometry, focus, or native command; the host resolves only the authenticated session's own window and applies it from that window's UI thread. Protocol 1.30 adds the separately granted `window.state.get` snapshot only because an owned title bar needs its own immediate maximize/restore glyph. It accepts exactly `{}` and samples only the same UI-thread-owned window, returning the same closed three-state vocabulary. It contains no handle, ID, position, bounds, monitor, DPI, visibility, z-order, focus, fullscreen state, timestamp, sequence, subscription, callback, event, or delivery result; it can be stale immediately. Protocol 1.31 `window.state.changes.read` separately exposes only one coalesced latest state or `null`, with no target, time, sequence, count, history, wait, callback, or subscription. Separate session-owned bridges and bounded retained state keep an unavailable UI surface or rapid resize from stranding a worker or growing an event queue. See `docs/WINDOW_STATE.md`, `docs/WINDOW_STATE_OBSERVATION.md`, `docs/WINDOW_STATE_CHANGES.md`, and Decisions 0072, 0117, and 0118. |
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
Protocol 1.25 narrows the authenticated exception to a session-owned group:
`window.open` validates a bounded document and title before the host UI thread
creates a private view, and `window.close` queues only a current opaque
secondary identity. The host resolves that identity through its own group map;
applications cannot supply, read, enumerate, or retain a raw handle, native
geometry, lifecycle state, or another session's route. A failed or timed-out
creation rolls back its logical identity, and native destruction removes the
matching identity only after the actual private view is gone.

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

The development samples exercise this private path with either a
developer-supplied Node.js executable and Anodrel sample script, an explicitly
selected compiled native health probe, an explicitly selected compiled native UI
probe, or an explicitly selected executable created by one of the native-template
generators. The UI probe and regular generated-template route receive only
document replacement, semantic-event pull, and their own close grant. The
separate menu route adds only `menu.write`; the separate multi-window route adds
only `window.open` and `window.close`; the separate form route adds only
`ui.fields.read`. Neither route lets a template supply grants, identity, title,
a session ID, a native handle, field selector, keyboard source, or
operating-system authority. In particular, a multi-window executable receives
an opaque logical secondary identity only after the host has validated, created,
and registered the private native view, while a form executable receives one
complete current-field snapshot only after it requests the host-owned
field-read bridge. None has executable identity verification and all end with
the host process, so none creates production application-launch authority.
Their output is intentionally discarded; an exit status is the only result used
by the host.

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
