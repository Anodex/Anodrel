# Anodrel native host

This is the first production-path native workspace. Its shipped dependency
graph contains only Anodrel crates, the Rust standard library, and direct
Windows APIs. It has no third-party runtime library.

~~~text
anodrel-json -> anodrel-protocol -> anodrel-core -> anodrel-windows-host
                                               ^                 |
                                               |                 +-> User32 / Kernel32
                                anodrel-wire -> anodrel-transport -> anodrel-bootstrap
                                                     ^                    ^
                                                     |                    |
                                      anodrel-windows-pipe -> Win32 / CNG |
                                                          anodrel-windows-bootstrap -> Kernel32

anodrel-client -> anodrel-bootstrap / anodrel-wire / anodrel-json
anodrel-windows-client -> anodrel-bootstrap / Kernel32

anodrel-windows-instance -> Kernel32 / User32

anodrel-windows-policy -> anodrel-application
                       `-> Advapi32

anodrel-session-policy -> anodrel-application / anodrel-core

anodrel-windows-registered-session -> anodrel-windows-policy / anodrel-windows-pipe

anodrel-perf-lab -> anodrel-wire / anodrel-transport / anodrel-core

anodrel-windows-launch -> anodrel-windows-policy / anodrel-windows-signature
                        -> anodrel-windows-bootstrap / Kernel32

anodrel-paths -> anodrel-application
anodrel-windows-paths -> anodrel-paths -> Shell32 / Ole32

anodrel-credentials -> anodrel-application
anodrel-windows-credentials -> anodrel-credentials -> Advapi32

anodrel-external-links -> Shell32
anodrel-windows-external-links -> anodrel-external-links -> Shell32

anodrel-json -> anodrel-application -> anodrel-windows-host

anodrel-ui -> future renderer / input adapter
anodrel-ui-document -> anodrel-json / anodrel-ui
anodrel-ui-session -> anodrel-ui-document / anodrel-ui
~~~

- `crates/json` is Anodrel's strict JSON codec for protocol messages.
- `crates/protocol` validates the documented public envelope and builds safe
  responses.
- `crates/core` applies host capability policy and message-size limits.
- `crates/wire` owns the bounded, versioned byte-stream frame format.
- `crates/transport` converts complete frames to policy-bound core responses.
- `crates/bootstrap` owns the bounded private child-invitation record and
  redacts its token from diagnostics.
- `crates/application` validates the strict application manifest, canonical
  package containment, built-in SHA-256 content digest, and bounded no-script text
  surface data. It does not authenticate a publisher or launch an executable.
- `adapters/windows-pipe` creates a logon-SID-restricted, one-client named pipe
  and authenticates its first frame with a CNG-generated credential; it can
  convert that invitation directly into the bootstrap record.
- `adapters/windows-bootstrap` launches one caller-selected child with an exact
  inherited handle list, provides the bootstrap record on standard input, then
  closes the parent endpoint. Its test fixture is test-only source, not a
  shipped runtime component.
- `crates/client` owns one portable authenticated framed child conversation;
  it has no endpoint, operating-system, policy, or capability authority.
- `adapters/windows-client` opens only the exact invitation-selected pipe with
  direct Kernel32 data I/O and owns that client handle through RAII.
  `anodrel-product-fixture` uses this pair rather than carrying a second client
  implementation.
- `tools/native-client-sample` is a fixed compiled development probe: it reads
  one invitation, authenticates, validates `platform.health`, and exits. It is
  neither a public SDK template nor a product application.
- `adapters/windows-instance` owns the bounded current-session mutex,
  readiness event, and no-data activation request for one package identity.
- `adapters/windows-policy` reads one bounded installed-application record
  from the fixed 64-bit `HKEY_LOCAL_MACHINE` registry location with query-only
  access. It validates that record through `crates/application` and can derive
  one host session policy through `crates/session-policy`, but cannot provision
  policy, verify a signature, launch a process, or create a pipe.
- `crates/session-policy` maps only a validated installed record's application
  ID and machine-selected capabilities into one `anodrel-core` host policy. It
  does not select policy storage, launch a process, create a transport, or
  accept grants from an application-facing value.
- `adapters/windows-registered-session` derives that host policy from the
  fixed Windows machine store and creates one owner-restricted named-pipe
  endpoint. It leaves process launch, invitation delivery, and worker-thread
  pipe service to their dedicated adapters.
- `tools/perf-lab` is a development-only first-party release benchmark. It
  measures fixed 1 KiB and 64 KiB in-process requests or a temporary real
  Windows named-pipe loopback through wire, authenticated transport, and core;
  it does not claim an application-runtime comparison.
- `adapters/windows-launch` is the host-only registered-process service. It
  locks the policy-approved executable, revalidates its digest and signer,
  launches no shell or application arguments, delivers one private bootstrap
  invitation, and terminates its tracked child during host shutdown.
- `tools/windows-installer` is the first owned distribution foundation. Its
  current read-only `validate` command parses the strict embedded release
  manifest, checks its complete bounded payload before the owned bundle decoder,
  and renders a record that the existing application validator accepts. Its
  direct Kernel32 resource reader accepts only two fixed current-image resources;
  it cannot install, uninstall, write machine policy, unpack a payload, or add a
  signing dependency.
- `crates/release-bundle` encodes and parses bounded, uncompressed release files
  with a per-file SHA-256 check. Its decoder borrows checked file contents from
  the signed payload and performs no filesystem or Windows API operation.
- `crates/paths` derives fixed per-application `data`, `cache`, and `logs`
  locations without filesystem I/O from a validated identity and an absolute
  operating-system root.
- `adapters/windows-paths` reads only the current user's Local AppData known
  folder through Shell32, then delegates the directory layout to `crates/paths`.
  It does not create, read, enumerate, or expose a directory to an application.
- `crates/credentials` owns validated credential names, exact per-application
  generic-credential targets, and bounded opaque secret values. It has no
  operating-system calls or public application protocol.
- `crates/ui` owns a bounded in-memory native UI document, semantic appearance
  roles, deterministic layout, clipping, semantic action hit testing, visible
  accessibility snapshot, and portable focus traversal. It has no renderer,
  package, protocol, scripting, operating-system dependency, or native
  authority.
- `crates/ui-document` strictly decodes and deterministically encodes the
  documented `anodrel.ui.document.v1` data format. It cannot render a document,
  accept a host session, invoke an action, or make an operating-system call.
- `crates/ui-session` owns one revision-bound current UI document and validates
  semantic actions against that exact document revision. It cannot authenticate
  a caller, render, queue an event, send a message, or make an operating-system
  call.
- `adapters/windows-credentials` reads, writes, and deletes only the exact
  generic Credential Manager target derived from a validated identity. It
  cannot enumerate credentials or expose a secret, target, or raw Windows
  status to diagnostics or an application.
- `crates/external-links` validates one bounded ASCII HTTPS link with a strict
  DNS-style authority. It has no native handoff, protocol, shell, or network
  operation.
- `adapters/windows-external-links` hands one validated HTTPS link to the
  ordinary Windows association through Shell32 with no verb, arguments,
  directory, process handle, or raw native error exposure.
- `hosts/windows` isolates raw Win32 FFI for a window class, message loop, and
  handle-keyed view registry, client-area drawing, and final-window shutdown.

The initial window displays a host-created `platform.health` response. With
`--application <manifest>`, it can also display the documented, digest-verified
plain-text application package. It has no webview, script runtime, navigation,
or general native bridge. Its named-pipe adapter and bootstrap launcher can
deliver a private invitation to a child process. The Windows host's
development-only diagnostics launch both the compiled native health probe and
the Node client: the former proves private bootstrap and authentication without
a runtime dependency, and the latter exercises the bounded service seams.
The registered interactive session implements the product capability bridge;
production publisher trust, packaging, installation, and updates remain
separate release work. The host-only registered launch service is separate from
the visual surface until a signed application record is provisioned.

`--showcase <manifest>` opens the Anodrel Startup Lab. Before any window is
created, it loads the supplied package, performs the internal protocol health
check, and completes one temporary internal named-pipe authentication and health
loopback. The direct GDI screen then displays only safe application identity and
foundation status. It has no webview, renderer bridge, public pipe client, or
application launch.

`--application` claims one current-session primary instance from the validated
package identity. A second invocation sends no data and makes only a bounded
best-effort activation request to the primary window. Startup Lab uses a
separate diagnostic scope. See `docs/INSTANCE_LIFECYCLE.md`.

`--window-lab` opens two static native windows to verify the multi-window
registry. Closing one leaves the other open; closing the final window exits the
message loop. It is a lifecycle diagnostic, not a public window API.

`--ui-lab` opens a fixed host-owned screen built through `anodrel-ui`. The
screen's JSON is compiled into the host and decoded through
`anodrel-ui-document`; it is not loaded from an application or external source.
Its semantic appearance roles, action hit tests, and focus state are interpreted
by the Windows renderer; actions show only their semantic IDs in the same
screen. Tab and Shift+Tab move the test focus ring; Enter reports the focused
action's ID. These actions do not create a session or grant a capability. See
`docs/UI.md`.

`--ui-preview <document.json>` is an explicit developer command that reads one
bounded regular JSON file, validates it through `anodrel-ui-document`, and
renders it through the same native UI view. It does not load a package or asset,
create a protocol session, or grant an action authority. See `docs/UI_PREVIEW.md`.

Verify from the repository root:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-bootstrap
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-launch
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-installer
cargo test --manifest-path native/Cargo.toml -p anodrel-release-bundle
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-paths
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-credentials
cargo test --manifest-path native/Cargo.toml -p anodrel-ui
cargo test --manifest-path native/Cargo.toml -p anodrel-ui-document
cargo test --manifest-path native/Cargo.toml -p anodrel-ui-session
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --iterations 5000
cargo run --release --manifest-path native/Cargo.toml -p anodrel-perf-lab -- --windows-pipe --iterations 5000
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --showcase apps/sample/anodrel.application.json
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-lab
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --ui-lab
~~~

The Windows host commands are manual smoke checks: the default host window must
show a `platform.health` success response; the sample package window must
identify `org.anodrel.sample` and display verified text; Startup Lab must show
the branded native surface, verified identity, and its foundation cards; and UI
Lab must update only its semantic-event reading when an action is clicked.
Close each normally. Windows is required; WebView2 is not.
