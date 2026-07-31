# Anodrel Initial Threat Model

**Status:** Foundation baseline. Update this document before the native host
accepts untrusted rendered content or adds a privileged capability.

## Scope

This model covers the protocol, SDK, and mock-host foundation. It defines the
controls a Windows native host must satisfy; it does not claim that the current
in-memory mock provides operating-system isolation.

The only current operations are `platform.ping`, `platform.capabilities`, and
`platform.health`. They expose no filesystem, process, credential, window, or
network authority.

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
| An application uses an operation it was not granted. | Check the host-owned capability immediately before executing every privileged operation. |
| A file operation escapes an approved location. | Canonicalize and validate paths after resolving links; enforce a per-operation scope. |
| A child process gains arbitrary shell authority or outlives the host. | Expose allowlisted operations, avoid shell interpolation, track child processes, and terminate them during shutdown. |
| A secret reaches the renderer or logs. | Use operating-system credential storage; redact secrets, raw native errors, and sensitive paths from protocol diagnostics and logs. |
| Message floods exhaust CPU or memory. | Reject frames above 64 KiB before decoding, reject more than four complete frames in one receive burst, and add authenticated-session queue, concurrency, timeout, and cancellation limits in the OS adapter. |
| A local or remote process connects to the endpoint by guessing its name. | Create one random-suffix pipe, restrict its DACL to the current logon SID, require a 32-byte CNG-generated token as the first frame, and close on any failed handshake. |
| A session invitation reaches another process or durable diagnostic. | Deliver one bounded record only through a child-only inherited standard-input handle; never put it in arguments, environment, logs, telemetry, crash data, or files. |
| Content path traversal or a symlink escapes a package. | Canonicalize the manifest directory and declared content file, reject root/prefix and dot path components, and require the resolved content to remain below the package root. |
| Package content is changed after its manifest is created. | Read bounded bytes and verify the manifest's lowercase SHA-256 digest before exposing text to the host surface. |
| Rendered text acquires script or native authority. | Support only `anodrel.text.v1`: bounded UTF-8 plain text with no scripts, navigation, URLs, resource loads, or native bridge. |
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

The owned wire codec rejects encoded messages larger than 64 KiB before UTF-8
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
file, checking its owned SHA-256 digest, and drawing only bounded plain text.
This detects content tampering but does not authenticate a publisher who can
replace the entire unpackaged directory. The surface has no queue, origin,
navigation, native bridge, or executable trust; those controls remain required
before untrusted application code or a privileged operation is accepted.

The package text surface also uses an owned current-session instance mutex and
readiness event derived from its validated application ID. A second invocation
cannot forward arguments or application data; it only sends a bounded
best-effort User32 activation message after the primary has created its window.
This preserves one-window coordination without treating the named object as an
identity or authorization mechanism.

The direct window host routes painting through its own handle-keyed registry;
the current package and Startup Lab surfaces cannot supply entries to it. A
failed multi-window creation is rolled back before the message loop starts, and
closing one window does not end the host while another registered window exists.

The development sample exercises this private path with a developer-supplied
Node.js executable and an owned sample script. It has no executable identity
verification and ends with the host process, so it creates no production
application-launch authority. Its output is intentionally discarded; an exit
status is the only result used by the host.

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
