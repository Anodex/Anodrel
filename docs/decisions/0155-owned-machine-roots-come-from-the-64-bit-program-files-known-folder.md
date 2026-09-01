# Decision 0155: Owned machine roots come from the 64-bit Program Files known folder

**Status:** Accepted

**Date:** 2026-08-31

## Context

The installer needs an application root before it can privately stage and
promote a signed release. Accepting a destination from a command line,
environment variable, release manifest, or application would make the machine
package location attacker-selectable. A 32-bit process's generic Program Files
known folder can select the x86 location on 64-bit Windows, which is not the
machine installation target for Anodrel's 64-bit host.

## Decision

The first-party 64-bit installer obtains `FOLDERID_ProgramFilesX64` through
`SHGetKnownFolderPath`, frees the returned shell allocation, and builds only
the fixed hierarchy:

~~~text
<Program Files>\Anodrel\Applications\<signed application ID>
~~~

Every existing or newly created component is required to be a normal directory
and not a reparse point. The installer checks the application identity again
before forming its final component. A 32-bit installer returns an unsupported
architecture error rather than silently using an x86 Program Files location.

## Consequences

- No install root is configurable through any user, application, or release
  input; only the already validated signed identity names the application node.
- Private staging and final version directories are siblings on one local
  machine-owned volume.
- The fixed Program Files location supplies the normal machine-install ACL
  boundary. Local administrators remain trusted machine operators and are not
  defended against as untrusted filesystem writers.
- The path selector has no release extraction, policy write, launch, trust, or
  network side effect beyond creating its fixed missing directories.

## Revisit conditions

Revisit for an approved per-user installation model, an enterprise deployment
broker, a Windows package identity, or an explicitly supported 32-bit host.
