# Native binary file-write template

**Status:** Implemented. Its isolated release build and authenticated invited-
pipe test pass. The real Windows save-dialog and created-byte walkthrough is
still a manual acceptance check.

## Purpose

`anodrel-native-app-tool init-file-binary-write` creates one deliberately small
Rust development project for the existing retained selected-output binary
boundary. It is a test and learning surface, not a product starter, general
file API, installer, package identity, signing system, or release route.

## Fixed authority

The matching Windows route is:

~~~text
anodrel-windows-host --native-file-binary-write-template-client <client.exe>
~~~

It issues exactly four grants:

- `ui.document.write`
- `dialog.save_file`
- `file.write_binary`
- `session.close`

The generated program supplies a fixed document, the fixed `Binary files` /
`bin` filter, and one canonical base64url value for the fixed bytes
`41 6E 6F 64 72 65 6C 00 FF`. It handles selected and cancelled results, writes
only through the opaque one-use reference returned by the host, then asks only
to close its own session. It cannot ask for a path, filename, initial directory,
reference, native handle, MIME type, alternate encoding, binary input, offset,
append mode, stream, progress, readback, atomicity, durability, event reader,
network access, identity, installer, or signing behavior.

## Generate, build, and run

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-native-app-tool -- init-file-binary-write .\scratch\binary-write-template anodrel-binary-write-template "Anodrel Native Binary File Write Template"
cd .\scratch\binary-write-template
cargo build --release
$clientPath = (Resolve-Path ".\target\release\anodrel-binary-write-template.exe").Path
$hostManifest = (Resolve-Path "..\..\native\Cargo.toml").Path
cargo run --release --manifest-path $hostManifest -p anodrel-windows-host -- --native-file-binary-write-template-client $clientPath
~~~

The simpler repository-root helper is `start-file-binary-write-template.bat`.

When the host opens its window, choose a fresh `.bin` destination in the
host-owned save dialog. After the window closes, that file contains exactly:

~~~text
41 6E 6F 64 72 65 6C 00 FF
~~~

Run the helper again and cancel the dialog. Cancellation closes cleanly and
must not leave a newly created destination behind.

## Boundary and verification

The displayed selected path is data only. The host captures the Windows output
object before returning an opaque reference; a one-time binary write consumes
that retained object. This first boundary is synchronous but not atomic: an
error after mutation begins can leave partial output. See
`docs/FILE_BINARY_WRITE.md` and Decision 0087 for the complete security and
cleanup contract.

The generated-child integration test proves the private invitation,
authentication, fixed filter, opaque-reference handover, exact byte write, and
self-close through an in-memory host service. It intentionally cannot prove a
real Windows dialog or filesystem result; the walkthrough above is the manual
desktop acceptance check.
