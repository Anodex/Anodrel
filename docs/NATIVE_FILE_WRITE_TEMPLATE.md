# Native file-write template

**Status:** Implemented. Its isolated release build and authenticated invited-
pipe test pass. The real Windows save-dialog and created-file walkthrough is
still a manual acceptance check.

## Purpose

`anodrel-native-app-tool init-file-write` creates one deliberately small Rust
development project for the existing retained selected-output text boundary.
It is a test and learning surface, not a product starter, general file API,
installer, package identity, signing system, or release route.

## Fixed authority

The matching Windows route is:

~~~text
anodrel-windows-host --native-file-write-template-client <client.exe>
~~~

It issues exactly four grants:

- `ui.document.write`
- `dialog.save_file`
- `file.write_text`
- `session.close`

The generated program supplies a fixed document, the fixed `Text documents` /
`txt` filter, and one fixed short UTF-8 line. It handles selected and cancelled
results, writes only through the opaque one-use reference returned by the host,
then asks only to close its own session. It cannot ask for a path, filename,
initial directory, reference, native handle, output text, binary data, offset,
append mode, stream, progress, readback, atomicity, durability, event reader,
network access, identity, installer, or signing behavior.

## Generate, build, and run

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-native-app-tool -- init-file-write .\scratch\file-write-template anodrel-file-write-template "Anodrel Native File Write Template"
cd .\scratch\file-write-template
cargo build --release
$clientPath = (Resolve-Path ".\target\release\anodrel-file-write-template.exe").Path
$hostManifest = (Resolve-Path "..\..\native\Cargo.toml").Path
cargo run --release --manifest-path $hostManifest -p anodrel-windows-host -- --native-file-write-template-client $clientPath
~~~

The simpler repository-root helper is `start-file-write-template.bat`.

When the host opens its window, choose a fresh `.txt` destination in the
host-owned save dialog. After the window closes, that file contains exactly:

~~~text
Hello from Anodrel's retained native file-write template.
~~~

Run the helper again and cancel the dialog. Cancellation closes cleanly and
must not leave a newly created destination behind.

## Boundary and verification

The displayed selected path is data only. The host captures the Windows output
object before returning an opaque reference; a one-time text write consumes
that retained object. This first boundary is synchronous but not atomic: an
error after mutation begins can leave partial output. See `docs/FILE_WRITE.md`
and Decision 0079 for the complete security and cleanup contract.

The generated-child integration test proves the private invitation,
authentication, fixed filter, opaque-reference handover, exact text write, and
self-close through an in-memory host service. It intentionally cannot prove a
real Windows dialog or filesystem result; the walkthrough above is the manual
desktop acceptance check.
