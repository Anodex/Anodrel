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

anodrel-windows-instance -> Kernel32 / User32

anodrel-windows-policy -> anodrel-application
                       `-> Advapi32

anodrel-json -> anodrel-application -> anodrel-windows-host
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
- `adapters/windows-instance` owns the bounded current-session mutex,
  readiness event, and no-data activation request for one package identity.
- `adapters/windows-policy` reads one bounded installed-application record
  from the fixed 64-bit `HKEY_LOCAL_MACHINE` registry location with query-only
  access. It validates that record through `crates/application` but cannot
  provision policy, verify a signature, or launch a process.
- `hosts/windows` isolates raw Win32 FFI for a window class, message loop, and
  handle-keyed view registry, client-area drawing, and final-window shutdown.

The initial window displays a host-created `platform.health` response. With
`--application <manifest>`, it can also display the documented, digest-verified
plain-text application package. It has no webview, script runtime, navigation,
native bridge, or privileged platform service. Its named-pipe adapter and
bootstrap launcher can deliver a private invitation to a child process. The
Windows host's development-only sample launches the compiled Node client to
prove the real authenticated health path. Publisher trust, executable launch,
and a capability bridge still require separate threat-model work.

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

Verify from the repository root:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-bootstrap
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --showcase apps/sample/anodrel.application.json
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --window-lab
~~~

The final two commands are manual Windows smoke checks: confirm that an
**Anodrel Windows host** window shows a `platform.health` success response, and
that the sample package window identifies `org.anodrel.sample` and displays
verified text. The Startup Lab command must show the branded native surface,
verified identity, and three foundation cards. Close each normally. Windows is
required; WebView2 is not.
