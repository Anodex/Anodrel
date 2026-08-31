# Decision 0142: Read release bytes from fixed resources in the current installer image

**Status:** Accepted

**Date:** 2026-08-31

## Context

The release manifest and bundle must be covered by the installer signature. A
sidecar JSON file, arbitrary package directory, command-line payload path, or
downloaded archive would allow authority data to change independently from the
installer executable.

Windows PE resources are part of the executable image Authenticode signs. They
can be read directly through Kernel32 without a file path, archive dependency,
or a second copy of the payload. The reader still must not mistake resource
loading for signature verification; its output is only signed authority after
the installer self-signature check is added.

## Decision

Use two `RT_RCDATA` resources in the current process executable only:

| Identifier | Bytes |
| --- | --- |
| `0xA141` | strict UTF-8 `anodrel.release.v1` manifest |
| `0xA142` | exact `anodrel.bundle.v1` payload |

The Windows installer obtains its current module with `GetModuleHandleW(NULL)`,
then uses `FindResourceW`, `SizeofResource`, `LoadResource`, and `LockResource`.
It accepts neither a module name nor a resource identifier from command-line,
package, protocol, environment, or application input.

The resource slice lives only while the process executable remains loaded. The
reader first parses the manifest, then verifies the payload's signed length and
digest, and finally runs the owned bundle decoder. It does not perform file I/O,
signature verification, extraction, registry mutation, or process launch.

## Consequences

Positive:

- Release authority bytes are selected from the executing image, not a mutable
  sidecar location.
- The reader uses direct Kernel32 calls and borrowed data with no archive code.
- Fixed identifiers give the owned release builder one unambiguous target.

Tradeoffs:

- An installer image without both resources cannot proceed, which is intended.
- This reader alone does not prove the current image is signed or trusted.
- The release builder must later create the PE resource section before the
production installer is signed.

## Revisit conditions

Revisit if the installer becomes a DLL, if a Windows package identity replaces
the executable resource envelope, or if resource size limits require streaming.
