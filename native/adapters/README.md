# Native adapters

This folder contains operating-system transport and service adapters. Each
adapter owns a narrow direct OS API boundary and depends on portable Anodrel
modules for protocol, policy, and framing.

- `windows-pipe` is the one-client authenticated Windows named-pipe adapter.
- `windows-bootstrap` owns restricted child-handle inheritance and one-time
  private invitation delivery. It does not make application trust decisions.
- `windows-policy` reads one installed application record from a fixed,
  machine-wide registry location with query-only access. It does not provision
  records or launch applications.
- `windows-launch` is the host-only process service that locks and rechecks a
  policy-approved executable, verifies its approved signer, launches it with
  no shell or application arguments, and tracks it for host shutdown.
- `windows-paths` reads the current user's Windows Local AppData known folder
  and derives host-only application directories through the portable path
  layout. It performs no filesystem mutation or public protocol operation.
- `windows-credentials` uses the current user's generic Windows Credential
  Manager store only through the exact target derived from an Anodrel identity
  and restricted credential name. It cannot enumerate credentials or expose a
  protocol operation.

Adapters must not place application behavior, public protocol definitions, or
raw OS calls into the platform core.
