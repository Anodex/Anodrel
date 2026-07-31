# Anodrel native host

This is the first production-path native workspace. Its shipped dependency
graph contains only Anodrel crates, the Rust standard library, and direct
Windows APIs. It has no third-party runtime library.

~~~text
anodrel-json -> anodrel-protocol -> anodrel-core -> anodrel-windows-host
                                               ^                 |
                                               |                 +-> User32 / Kernel32
                                anodrel-wire -> anodrel-transport
                                                     ^
                                                     |
                                      anodrel-windows-pipe -> Win32 / CNG
~~~

- `crates/json` is Anodrel's strict JSON codec for protocol messages.
- `crates/protocol` validates the documented public envelope and builds safe
  responses.
- `crates/core` applies host-owned capability policy and message-size limits.
- `crates/wire` owns the bounded, versioned byte-stream frame format.
- `crates/transport` converts complete frames to policy-bound core responses.
- `adapters/windows-pipe` creates a logon-SID-restricted, one-client named pipe
  and authenticates its first frame with a CNG-generated credential.
- `hosts/windows` isolates raw Win32 FFI for a window class, message loop, and
  client-area drawing.

The initial window displays a host-created `platform.health` response. It has
no webview, external content loader, or privileged platform service. Its owned
named-pipe adapter is ready for private host-to-application invitation delivery;
that bootstrap and application content host require separate threat-model work
before implementation.

Verify from the repository root:

~~~text
cargo fmt --manifest-path native/Cargo.toml --all --check
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path native/Cargo.toml
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host
~~~

The last command is the manual Windows smoke check: confirm that an **Anodrel
Windows host** window opens, shows a `platform.health` success response, and
closes normally. It requires Windows but does not require WebView2.
