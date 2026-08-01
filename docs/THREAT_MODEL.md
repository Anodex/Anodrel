# Anodrel Initial Threat Model

**Status:** Foundation baseline. Update this document before the native host
accepts untrusted rendered content or adds a privileged capability.

## Scope

This model covers the protocol, SDK, and mock-host foundation. It defines the
controls a Windows native host must satisfy; it does not claim that the current
in-memory mock provides operating-system isolation.

The current operations are `platform.ping`, `platform.capabilities`,
`platform.health`, and `ui.document.replace`. They expose no filesystem,
process, credential, window, or network authority.

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
| An application reads, replaces, or leaks another application's credential. | Derive one exact generic Credential Manager target from the host-validated application ID and a restricted credential name; prohibit arbitrary targets and enumeration, keep secrets opaque and bounded, and expose no credential protocol operation until authenticated capability checks exist. |
| A child process gains arbitrary shell authority or outlives the host. | The launch service supplies only the policy-approved `.exe` with no child arguments or shell, retains a child handle, and terminates it on shutdown. |
| A mutable package substitutes a trusted but unauthorized executable. | Do not treat a package-held manifest or an Authenticode result alone as launch authority; require an external installed application record, lock the contained executable against write/delete/rename, hash it through that lock, then match its verified signer to the record's application-ID-bound publisher fingerprint. |
| An application chooses or substitutes its launch policy. | Read the installed record only from the fixed 64-bit `HKEY_LOCAL_MACHINE` policy location selected by the host; accept no current-user, package, environment, protocol, or UI policy source. Require the registry key, record, and validated package to carry the same application ID. |
| A child process grants itself a capability. | Convert only the validated installed record's strict capability array into the host session policy; reject unknown or duplicate grants, treat version 1.0 records as grant-free, and never accept grants from package, bootstrap, pipe, protocol, or UI data. |
| A secret reaches the renderer or logs. | Use operating-system credential storage; redact secrets, raw native errors, and sensitive paths from protocol diagnostics and logs. |
| A host diagnostic log captures untrusted or sensitive data. | The first log accepts only a closed typed host-event enum; it has no dynamic message, payload, path, error, credential, persistence, export, or protocol input. |
| Message floods exhaust CPU or memory. | Reject frames above 64 KiB before decoding, reject more than four complete frames in one receive burst, and add authenticated-session queue, concurrency, timeout, and cancellation limits in the OS adapter. |
| A local or remote process connects to the endpoint by guessing its name. | Create one random-suffix pipe, restrict its DACL to the current logon SID, require a 32-byte CNG-generated token as the first frame, and close on any failed handshake. |
| A session invitation reaches another process or durable diagnostic. | Deliver one bounded record only through a child-only inherited standard-input handle; never put it in arguments, environment, logs, telemetry, crash data, or files. |
| Content path traversal or a symlink escapes a package. | Canonicalize the manifest directory and declared content file, reject root/prefix and dot path components, and require the resolved content to remain below the package root. |
| Package content is changed after its manifest is created. | Read bounded bytes and verify the manifest's lowercase SHA-256 digest before exposing text to the host surface. |
| Rendered text acquires script or native authority. | Support only `anodrel.text.v1`: bounded UTF-8 plain text with no scripts, navigation, URLs, resource loads, or native bridge. |
| A future application supplies malformed or oversized UI data. | Decode only the exact `anodrel.ui.document.v1` schema through the 64 KiB strict JSON boundary and validate every existing UI model limit before returning a document. The host accepts an external document only through its separate explicit bounded developer preview command, never an application session. |
| An operator-selected preview file creates broader host access. | The preview opens only one bounded regular UTF-8 file named directly on the local command line, validates it before window creation, and loads no companion file, package, policy, session, asset, executable, URL, or native capability. |
| A late input event targets a replaced UI document. | Bind every accepted semantic action to the exact monotonic document revision that produced its layout; reject events for an empty, replaced, removed, or disabled action. The current state crate has no I/O or application delivery path. |
| An authenticated application overwhelms or corrupts its UI session. | Require the host-issued `ui.document.write` grant immediately before `ui.document.replace`; limit its encoded document string to 24 KiB within the 64 KiB wire message; use the strict v1 codec and atomic replacement; expose only a revision string and retain the prior state on failure. No window binding, document readback, or event delivery exists yet. |
| UI replacement traffic builds an unbounded cross-thread backlog. | Transfer accepted snapshots only through one per-session mailbox slot. A newer revision replaces the pending older revision; no update callback, pipe I/O, renderer work, or semantic event enters that slot. |
| A pipe worker manipulates a native window or a session document drives another window. | Give a UI Session Lab exactly one mailbox and poll it only from its own Windows UI thread; accept only a newer revision into that one view. The pipe worker never calls User32, and session documents have no window-selection field. |
| Two host invocations race to display one package identity. | Claim a current-session mutex from the validated application ID; a secondary waits at most one second and can only issue a no-data best-effort activation request. |
| A same-session process signals or reserves an instance object. | Treat the instance channel as local coordination only: it carries no payload or authority and returns a safe failure instead of creating a second window when readiness cannot be established. |
| Two native windows render each other's state or one close ends the host early. | Keep immutable host-created views in a handle-keyed registry and exit the UI loop only after the final registered window is destroyed. |

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
tracked child lifetime, and post-verification bootstrap binding. Record
provisioning and host UI integration remain before the first Launch Sample
capability becomes available.

## Before the first privileged capability

The native-host decision must extend this model with:

- a signed package and verified executable identity model;
- session-authentication binding and bridge bootstrap details for any future
  executable application;
- the Windows named-pipe session bootstrap, authentication, and I/O scheduling;
- the Windows permission, filesystem, process, credential-store, and update
  assumptions; and
- tests for traversal, malformed input, capability bypass, shutdown races, and
  overload handling.

No filesystem, process, credential, clipboard, dialog, notification, or
external-link operation may be implemented until its contract and these
host-specific controls are documented and tested.
