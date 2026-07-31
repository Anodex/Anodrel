# Native adapters

This folder contains operating-system transport and service adapters. Each
adapter owns a narrow direct OS API boundary and depends on portable Anodrel
modules for protocol, policy, and framing.

- `windows-pipe` is the one-client authenticated Windows named-pipe adapter.
- `windows-bootstrap` owns restricted child-handle inheritance and one-time
  private invitation delivery. It does not make application trust decisions.

Adapters must not place application behavior, public protocol definitions, or
raw OS calls into the platform core.
