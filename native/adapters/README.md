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

Adapters must not place application behavior, public protocol definitions, or
raw OS calls into the platform core.
