# Anodrel development native UI template

**Status:** The portable typed client, first-party new-directory generator, and
explicit fixed-grant Windows development host route are implemented. The
generator's isolated build and real authenticated generated-child session tests
pass. Manual native-window action verification remains open. This is a Windows
development template, not a product packaging or application-identity format.

## Purpose

The template lets a new Rust executable render one strict Anodrel UI
document, receive semantic actions from that document, and request that its own
host-controlled session close. It removes private bootstrap records, wire frames,
raw request JSON, and host orchestration from project source while preserving
the platform's explicit capability boundary.

It does not make a generated executable trusted. The operator selects it
explicitly for a local development command; the host does not validate its
identity. A signed installed application remains the separate product-session
path in `docs/PRODUCT_SESSIONS.md`.

## Typed client contract

`anodrel-ui-client` wraps one already-authenticated `anodrel-client` session.
Its implemented initial Protocol 1.3 surface is exactly this:

| Operation | Input | Typed result | Required host grant |
| --- | --- | --- | --- |
| `replace_document_v1` | one exact `anodrel.ui.document.v1` string | `DocumentRevision` | `ui.document.write` |
| `read_actions` | none | `UiActionBatch` | `ui.events.read` |
| `close` | none | accepted close request | `session.close` |

`DocumentRevision` is a validated nonzero decimal document revision. The client
does not convert it through a floating-point number. `UiAction` contains only
that revision and the action's semantic ID. `UiActionBatch` contains at most 32
actions plus the exact bounded `dropped` and `discarded` counters returned by
the host. A batch containing an unexpected event shape or event name is a
closed client error, never a raw fallback value.

The facade creates its own opaque request IDs and sends the minimum compatible
Protocol 1.3 version. It exposes no method that names an arbitrary operation,
selects a protocol version, supplies a capability list, accesses a native
handle, launches a process, or reads a raw response. There is no background
event receiver: an application deliberately calls `read_actions` when it wants
one bounded pull.

The application still explicitly opens the one invited Windows pipe through
`anodrel-windows-client` after reading its standard-input invitation. That
adapter can open only the invitation's exact pipe and owns no host policy or
window API. See `docs/NATIVE_CLIENT.md`.

The regular template has no `menu.write` grant. The separately implemented
typed menu extension is reserved for the native menu template's explicit
generator command and host route; it does not silently broaden this project's
authority. See `docs/NATIVE_MENU_TEMPLATE.md`.

## Generated project contract

`anodrel-native-app-tool init` accepts a destination, a Cargo-compatible
project slug, and a display label. It refuses an existing destination and
writes only a new project directory containing:

~~~text
my-native-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel dependency path is relative to the local checkout from which the
tool was run. The destination's parent directory must already exist. The tool
writes no absolute path, machine setting, certificate, installed record,
signature, capability declaration, package, or generated secret. The example
program has one compiled-in v1 document and one action. It does not load
application code, content, configuration, or a document from a path, URL,
environment variable, or command argument.

From the Anodrel checkout, create a project with:

~~~powershell
cargo run --release --manifest-path native\Cargo.toml -p anodrel-native-app-tool -- init .\my-native-app my-native-app "My Native App"
~~~

The generated project compiles in isolation with `cargo build --release`. Its
README gives the exact checkout-relative host command to open it in a
development session.

The display label is project text only. It is not an application ID, a trusted
publisher name, a host window title, or a machine-policy value.

## Development host session

`--native-template-client <client.exe>` creates one host-controlled native window
and grants its one authenticated session only:

- `ui.document.write`;
- `ui.events.read`; and
- `session.close`.

It never accepts project-supplied grants. The host owns the window title,
mailboxes, semantic hit testing, process handle, pipe worker, and cleanup. A
child exits early, closes its session, or times out only through the host's
bounded lifecycle; it cannot leave a worker or child running in the background.

## Compatibility and verification

This template introduces no protocol version or wire format. It uses existing
Protocol 1.3 `ui.document.replace`, `ui.events.read`, and `session.close`
operations, and existing `ANBI` bootstrap v1 plus `ANDR` wire v1. Its proof
includes typed-client tests and a real Windows pipe test: the existing compiled
native UI diagnostic now consumes this facade in its end-to-end test. The
generator's test creates a new project, checks that its dependency paths remain
relative, and runs an isolated release `cargo build` against the generated
manifest.
The generator's real-pipe integration test builds a generated executable,
delivers an invitation, verifies its first document, supplies only its fixed
semantic action, then verifies self-close and clean exit. The Windows host's
shared fixed-grant lifecycle has a unit test. The remaining verification is the
documented manual native-window action. See Decision 0082.
