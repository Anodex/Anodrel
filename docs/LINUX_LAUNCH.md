# Linux development child launch

**Status:** Direct development-only launcher. It is not Linux application
identity, package validation, a product launcher, or a desktop host.

## Boundary

`anodrel-linux-bootstrap` starts one host-selected absolute executable and
delivers one already-created ANLI invitation to its standard input. It accepts
no arguments, shell text, environment input, working-directory input, token,
endpoint, output route, or application request.

~~~text
host-selected absolute executable
  -> direct fork + execve with an empty environment
  -> child standard input receives one ANLI frame then end-of-file
  -> child output and error go to /dev/null
~~~

The executable path is mechanics, not trust. The adapter checks only that it is
absolute and contains no current- or parent-directory component. A future
Linux package and identity policy must select and validate an executable before
calling it.

## Lifecycle

The returned process value is opaque. A host may wait for a bounded time or
send the fixed termination signal; dropping the value does not kill the child.
This preserves the separation between launch mechanics and host session
lifetime. A failed bootstrap write kills and reaps the child immediately, so an
uninvited child cannot remain alive.

The child has no inherited Anodrel file descriptor beyond standard input. The
launcher gives it an empty environment and redirects standard output/error to
`/dev/null`. It captures no output and exposes no process ID or signal choice.

## Verification

The Linux compiled-client integration test starts a real abstract socket,
launches the fixed probe through this adapter, proves the ANLI round trip and
`platform.health` exchange, and observes a clean exit. Run:

~~~powershell
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-native-linux-client-sample'
~~~

This does not verify a Linux visible window, application identity, package,
policy, installer, update route, or general command execution.
