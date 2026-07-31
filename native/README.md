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
~~~

- `crates/json` is Anodrel's strict JSON codec for protocol messages.
- `crates/protocol` validates the documented public envelope and builds safe
  responses.
- `crates/core` applies host-owned capability policy and message-size limits.
- `crates/wire` owns the bounded, versioned byte-stream frame format.
- `crates/transport` converts complete frames to policy-bound core responses.
- `crates/bootstrap` owns the bounded private child-invitation record and
  redacts its token from diagnostics.
- `adapters/windows-pipe` creates a logon-SID-restricted, one-client named pipe
  and authenticates its first frame with a CNG-generated credential; it can
  convert that invitation directly into the bootstrap record.
- `adapters/windows-bootstrap` launches one caller-selected child with an exact
  inherited handle list, provides the bootstrap record on standard input, then
  closes the parent endpoint. Its test fixture is test-only source, not a
  shipped runtime component.
- `hosts/windows` isolates raw Win32 FFI for a window class, message loop, and
  client-area drawing.

The initial window displays a host-created `platform.health` response. It has
no webview, external content loader, or privileged platform service. Its owned
named-pipe adapter and bootstrap launcher can deliver a private invitation to a
child process. The Windows host's development-only sample launches the compiled
Node client to prove the real authenticated health path. Application identity,
controlled content loading, and a rendered content host still require separate
threat-model work before implementation.

Verify from the repository root:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-bootstrap
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
~~~

The last command is the manual Windows smoke check: confirm that an **Anodrel
Windows host** window opens, shows a `platform.health` success response, and
closes normally. It requires Windows but does not require WebView2.
