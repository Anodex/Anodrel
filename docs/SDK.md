# Anodrel application SDK

**Status:** Foundation API in this repository. The TypeScript package is not
published independently yet, and it is not a packaged application runtime.

`@anodrel/sdk` is the small application-facing layer above Anodrel's versioned
protocol. It turns a typed method call into one protocol request, verifies that
the response belongs to that request, and turns a structured host failure into
a typed error. It does not contain a native host, a browser engine, operating
system calls, a capability policy, or ambient access to a user's machine.

## Start with the public boundary

The SDK needs a `PlatformTransport`. A host integration owns that transport and
authenticates the application session before it serves requests. The client
never supplies an application ID, a session ID, or a capabilities list.

For local development, use the repository's mock host:

~~~ts
import { MockHost } from "@anodrel/mock-host";
import { PlatformClient, PlatformRemoteError } from "@anodrel/sdk";

const host = new MockHost({
  applicationId: "org.example.hello",
  grantedCapabilities: ["diagnostics.read"],
});
const client = new PlatformClient(host.createTransport());

try {
  const health = await client.getHealth();
  console.log(health.hostName);
} catch (error) {
  if (error instanceof PlatformRemoteError) {
    console.error(error.code);
    throw error;
  }
  throw error;
}
~~~

The mock models the public protocol and capability checks. It is not a native
security boundary, a package launcher, or evidence that a real operating-system
adapter permits an operation.

The repository's `apps/command-line` example uses exactly this path to report
the host name, negotiated protocol version, and host-issued capabilities. Run
it with `npm run cli-demo`; it is a development example, not an installed
command or a replacement for a real authenticated native session.

## Public surface

`PlatformClient` provides typed methods for the exact operations defined by
`docs/PROTOCOL.md`, including health and capability discovery, bounded
diagnostic reads, UI document replacement and semantic-event reads, session
close, clipboard, HTTPS handoff, file and folder dialogs plus retained text read and
bounded text or binary write, state, credentials, notifications, host-authorized
HTTPS text fetches, narrow primary-window commands, and bounded session-owned
view creation, replacement, close, and event reads. Every
method takes only the documented payload fields; it cannot accept a native
handle, arbitrary application identity, capability list, window target,
filesystem path where the protocol does not allow one, or a callback.

`PlatformTransport` is the narrow host-integration interface. It accepts one
typed request and optional ordered cancellation record. An application can
provide a `RequestIdFactory` when deterministic IDs help a test; production
uses `crypto.randomUUID` and fails closed if the runtime cannot provide it.

Failures have two separate meanings:

- `PlatformRemoteError` is a host response with a stable protocol error code,
  retryability flag, and only safe documented details.
- `PlatformProtocolError` means the transport returned an impossible response,
  such as one with a different request ID, or the local runtime cannot generate
  a cryptographically strong request ID.

The result and error shapes, field limits, capability requirements, and
compatibility rules live in `docs/PROTOCOL.md`. A method name is a convenience
for that contract, not a second policy layer.

### Folder selection

Protocol 1.28 adds `openFolderDialog()`. It sends exactly `{}` and requires the
separate `dialog.open_folder` grant. It returns either a selected folder path or
cancellation. The SDK has no filter, initial folder, title, multiple-selection,
or native-dialog option because those are host policy. A selected path is display
data only, never a retained folder permission or later filesystem authority.
See `docs/FOLDER_DIALOGS.md`.

### Selected-folder entries

Protocol 1.29 adds `openFolderDialogWithReference()` and
`readSelectedFolderEntries(folderReference)`. The first retains a selected
folder under the existing `dialog.open_folder` grant and returns either
cancellation or display path plus an opaque, one-use folder reference. The
second needs the separate `folder.read_entries` grant and returns at most 32
direct `name` and conservative `kind` values, plus a `complete` flag. It has
no recursive flag, cursor, child path, content read, metadata, write, delete,
creation, rename, or watch option. The SDK never constructs a folder reference
or turns a display path into authority. See `docs/FOLDER_ACCESS.md`.

### Session-window state observation

Protocol 1.30 adds `getWindowState()`. It sends exactly `{}` and needs the
separate `window.state.read` grant. Its result is one immediate `minimized`,
`maximized`, or `restored` snapshot for the requesting session's own native
window. There is no window argument, target, handle, geometry, monitor, focus,
fullscreen state, timestamp, subscription, or change event. The value may be
stale as soon as it arrives; code that uses `setWindowState()` must refresh
with `getWindowState()` rather than infer a result from action acceptance. See
`docs/WINDOW_STATE_OBSERVATION.md`.

### Session-window state changes

Protocol 1.31 adds `readWindowStateChanges()`. It sends exactly `{}` and needs
the separate `window.state.observe` grant. Its result holds either one latest
coalesced `minimized`, `maximized`, or `restored` transition for the requesting
session's own native window, or `null` when no unread transition exists. The
first native observation establishes a baseline, so use `getWindowState()` for
initial state. This is an immediate pull: it does not wait, subscribe, invoke a
callback, or create a background listener. It cannot take a target, handle,
geometry, focus, timestamp, sequence, history, or event-count option. See
`docs/WINDOW_STATE_CHANGES.md`.

### Host-authorized HTTPS text fetch

Protocol 1.19 adds `fetchHttpsText(url)`. It sends exactly the supplied
validated HTTPS URL and returns only the response's `statusCode` and bounded
UTF-8 `text`. It does not accept a method, body, header, cookie, credential,
redirect, proxy, timeout, client certificate, callback, or network handle.
The host must have issued the separate `network.fetch` capability and attached
a service whose own host-selected exact-origin policy allows the URL. A
successful result, including a non-2xx HTTP status, says only that the bounded
HTTP response was represented; it is not an application-level success or a
  claim about a user's network state. See `docs/NETWORK.md`.

### Bounded binary file output

Protocol 1.22 adds `writeSelectedFileBinary(saveReference, bytes)`. It accepts
only an existing opaque save reference and a `Uint8Array` of at most 32 KiB.
The SDK uses its own first-party canonical unpadded base64url encoder, then
sends the exact `file.write_binary` request. A larger local byte array rejects
before a frame is made. The host still performs all authority checks: it needs
the separate `file.write_binary` grant and consumes the retained output object
once, so the SDK method does not accept a path, filename, type, handle, stream,
offset, callback, or readback option. See `docs/FILE_BINARY_WRITE.md`.

### Session-window foreground request

Protocol 1.20 adds `requestWindowFocus()`. It takes no argument and requests
attention for the one host window already bound to the authenticated session.
It requires the separate `window.focus` grant and returns only
`{ status: "requested" }`: this means Windows accepted the host's foreground
request, never that a person saw or used the window. It cannot target a window,
provide a native handle, change retry
policy, inject input, return focus state, or subscribe to a change. See
`docs/WINDOW_FOCUS.md`.

### Session-window fullscreen request

Protocol 1.21 adds `setWindowFullscreen("fullscreen" | "windowed")`. It
requires the separate `window.fullscreen` grant and returns only
`{ status: "applied" }`. The host chooses and owns the native presentation of
the one host window already bound to the authenticated session: there is no
window argument, monitor selection, coordinate, display mode, geometry,
fullscreen-state readback, or event. `fullscreen` is reversible borderless
windowed fullscreen, not exclusive display control; `windowed` asks the host to
restore its retained presentation facts. See `docs/WINDOW_FULLSCREEN.md`.

### Session-window client-size request

Protocol 1.23 adds `setWindowSize(width, height)`. It requires the separate
`window.size` grant and accepts only whole logical client-area dimensions:
width 320 through 3840 and height 240 through 2160. The result is only
`{ status: "applied" }`. The host derives its framed native size at the current
DPI of the one session-bound window. There is no window argument, target,
position, outer bounds, monitor, DPI readback, state readback, resize event,
constraint, or native handle. See `docs/WINDOW_SIZE.md`.

### Session-owned secondary views

Protocol 1.25 adds `openWindow(title, document)`, `closeWindow(windowId)`,
`replaceUiDocumentInWindow(windowId, document)`, and `readWindowUiEvents()`.
`openWindow` requires both `window.open` and `ui.document.write`; it returns an
opaque `windowId` only after the host created and registered a secondary view.
There can be at most three secondary views. The SDK cannot supply native size,
position, monitor, parentage, style, handle, or an identity from another
session.

`closeWindow` requires `window.close` and accepts only a secondary identity
returned by `openWindow`; `main` remains the session anchor and ends through
`closeSession()`. Its result confirms host acceptance, not destruction or a
person's observation. `replaceUiDocumentInWindow` requires the existing
`ui.document.write` grant and accepts `main` or a currently issued secondary
identity, always using the strict v1 document format. `readWindowUiEvents`
requires `ui.events.read` and returns only revision-checked semantic actions
tagged with their `windowId`; order across different views has no desktop-time
meaning. See `docs/MULTI_WINDOW.md`.

### Secondary scroll documents

Protocol 1.27 adds `openWindowV2(title, document)` and
`replaceUiDocumentV2InWindow(windowId, document)`. Each accepts only an exact
version-2 scroll document. They retain the existing `window.open` and
`ui.document.write` grants, but provide no scroll position, event, callback,
or observation. Scrolling remains local to the host-owned view. See
`docs/SCROLLING.md` and Decision 0102.

### Semantic live-status documents

Protocol 1.26 adds `replaceUiDocumentV3(document)`, `openWindowV3(title,
document)`, and `replaceUiDocumentV3InWindow(windowId, document)`. Each accepts
only an exact version-3 document; the replacement methods retain the existing
`ui.document.write` grant and opening a secondary view additionally requires
`window.open`. A response confirms only accepted document state. It never
reports whether Windows announced a visible status or whether anyone heard it.
See `docs/UI_LIVE_ANNOUNCEMENTS.md`.

### Native session menus

`replaceMenu(menus)` continues to require the separate `menu.write` grant and
returns only a host-owned revision. Protocol 1.24 lets an item include an
optional `shortcut`: exactly `Ctrl+<key>` or `Ctrl+Shift+<key>`, where `<key>`
is one uppercase ASCII letter or digit. A shortcut describes only a local
semantic menu action; it is not a native virtual-key code, global registration,
input listener, callback, target, handle, or shortcut readback. The host may
deliver one current enabled action through the existing granted
`readUiEvents()` route, with the same revision revalidation as a menu click.
See `docs/MENUS.md`.

## Windows development transport

`@anodrel/windows-transport` is a separate development-only Node-core adapter
for the direct Windows bootstrap and named-pipe diagnostic path. It is not a
production content host and is never embedded by the native Windows host. The
sample's `native-client.ts` demonstrates that path; its launch commands live in
`docs/DEVELOPMENT_DIAGNOSTICS.md`.

## Compatibility and boundaries

The SDK is versioned with the protocol. Adding an application-visible method,
payload field, response field, error, or event requires a documented protocol
compatibility change and contract tests before the client exposes it. Moving
the SDK's internal source files cannot change its package-root exports or
request behavior.

No SDK method grants authority. The native host derives identity and
capabilities, checks the exact grant immediately before an operation, and owns
all Windows objects and service calls. A response that says a request was
accepted is only the evidence that each operation's protocol contract promises;
for example, notification acceptance does not mean it was seen, and a session
close acceptance does not mean a window is already destroyed.
