# Linux development sessions

## Status

`anodrel-linux-development-session` is the first host-owned Linux lifecycle
adapter. It joins one private Linux pipe, one ANLI invitation, one exact
development child, and two host worker threads. It is not an application
desktop host, product session, package validator, or public SDK surface.

## Lifecycle

~~~text
host-selected policy + session ID + executable
    │
    ├── create one same-UID authenticated abstract socket
    ├── convert its invitation into one ANLI standard-input record
    ├── start one exact executable through direct fork + execve
    ├── run the pipe worker away from a future UI thread
    └── watch the opaque child exit away from a future UI thread
              │
              ▼
either worker ending requests one host-local close signal
              │
              ▼
close requests pipe stop + fixed child termination
              │
              ▼
host joins both workers before the session ends
~~~

The child and worker never outlive a session that is explicitly finished or
dropped. During shutdown the host first sends the launch adapter's fixed
termination signal. The exit watcher gives that request 250 ms, then uses the
adapter's fixed final signal if the child is still alive. Neither signal,
process identifier, exit code, endpoint, token, or error crosses into the
application protocol.

## Boundaries

The coordinator accepts only a `HostPolicy`, opaque session ID, and a
`LinuxBootstrapProgram` that has already accepted the host-selected absolute
path. It does not choose an executable from application data; accept an
argument, command, environment, output, working directory, restart policy, or
window binding; or expose a PID, handle, signal selector, child output, exit
code, timing, or callback.

The returned close signal is host-local and coalescing. A future Wayland host
must retain a running session while it owns its matching view, use that signal
to request the view close, and then finish the session. That composition does
not exist yet.

## Verification

The native Linux client integration suite launches the compiled fixed health
probe through the coordinator, observes the host-local close signal, and joins
the child watcher and authenticated pipe worker:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-native-linux-client-sample
~~~

The test proves the development lifecycle only. It does not create a Wayland
window, load an application document, validate an executable identity, or
prove packaging, installation, updates, accessibility, or a product launcher.

See Decision 0130, docs/LINUX_TRANSPORT.md, docs/LINUX_LAUNCH.md, and
docs/LINUX_WINDOWING.md.
