# Decision 0021: Derive application directories from host-validated identity

**Status:** Accepted

**Date:** 2026-07-31

## Context

Future Anodrel storage, credential, logging, and update services need stable
per-application locations. Letting a package, request, environment variable,
or child process choose an absolute path would permit cross-application
collisions and path traversal. Creating a general filesystem capability before
its permissions and recovery behavior are designed would be premature.

## Decision

Add a portable `anodrel-paths` crate that derives an application root plus
`data`, `cache`, and `logs` locations only from an absolute operating-system
root and the shared validated application-ID grammar. It performs no I/O and
does not expose raw paths in debug formatting.

Add `anodrel-windows-paths`, a direct Windows adapter that obtains the current
user's Local AppData root with `SHGetKnownFolderPath` and frees Windows-owned
memory with `CoTaskMemFree`. It returns the portable directory value but does
not create or inspect any directory, select another user's profile, accept a
path from application input, or expose a public protocol operation.

The stable Windows namespace is:

~~~text
%LOCALAPPDATA%\Anodrel\Applications\<applicationId>\{data,cache,logs}
~~~

## Consequences

Positive:

- directory layout is deterministic, auditable, and isolated by the validated
  application identity;
- portable layout rules can be reused by macOS and Linux adapters without
  leaking Windows APIs into the platform layer;
- path lookup has no filesystem mutation or ambient process working-directory
  dependency.

Tradeoffs:

- no application can use these locations until a separate, permissioned
  storage or logging service is documented;
- changing a location later requires a data migration rather than an implicit
  path remap;
- the Windows adapter is current-user only and is not an installer or service
  account path policy.

## Revisit conditions

Revisit when Anodrel defines a documented storage capability, a per-machine
service path policy, roaming-data semantics, or a cross-platform migration
format for existing application data.
