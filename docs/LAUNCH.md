# Installed application records v1

**Status:** Foundation contract. This defines the policy input for a future
Windows process-launch operation. It does not authorize a launch or define a
public application capability.

## Purpose and boundary

An executable and its package directory are mutable application input. An
`anodrel.application.json` file inside that directory cannot authorize its own
publisher or executable. Before a Windows host may start a product process, it
needs a separate policy record selected from a trusted host policy directory.

An installed application record binds one validated application ID to:

- a canonical package root;
- a canonical `.exe` below that root and its SHA-256 digest; and
- one approved leaf signing-certificate SHA-256 fingerprint.

The record must reside outside the package directory. A future Windows policy
store selects the policy directory and ensures only an authorized installer or
administrator can change it. Applications, package content, protocol messages,
command-line arguments, and rendered UI never choose a record or policy
directory.

This parser verifies record shape, package identity, executable containment,
and executable digest. It does **not** call Windows Authenticode, start a
process, create a pipe, or expose a record to an application or renderer.

## Record format

A record is strict UTF-8 JSON no larger than **16 KiB**. Version 1.0 accepts
exactly the fields below; unknown, missing, duplicate, and wrongly typed fields
are rejected.

~~~json
{
  "recordVersion": { "major": 1, "minor": 0 },
  "applicationId": "org.anodrel.sample",
  "packageRoot": "C:\\Program Files\\Anodrel\\Sample",
  "executable": {
    "path": "bin/anodrel-sample.exe",
    "sha256": "64 lowercase hexadecimal characters"
  },
  "publisher": {
    "leafCertificateSha256": "64 lowercase hexadecimal characters"
  }
}
~~~

| Field | Rule |
| --- | --- |
| `recordVersion` | Object with numeric `major: 1` and `minor: 0`. |
| `applicationId` | Uses the same 3â€“128 character identity grammar as the validated package manifest and exactly equals its `applicationId`. |
| `packageRoot` | Absolute local directory path. Its canonical value is private host data and is never rendered. |
| `executable.path` | Relative forward-slash-separated package path. It cannot contain roots, drives, `.` or `..`, or backslashes, and must end in `.exe` (case-insensitive). The canonical result remains inside `packageRoot`. |
| `executable.sha256` | Lowercase hexadecimal SHA-256 of raw executable bytes. Files above **128 MiB** are rejected. |
| `publisher.leafCertificateSha256` | Lowercase hexadecimal SHA-256 fingerprint expected from the accepted embedded Authenticode leaf certificate. It is internal comparison data, never display text. |

The package root must contain `anodrel.application.json`. The parser loads it
with normal containment and content-digest checks before accepting the record's
identity binding.

## Windows machine-policy store

The first trusted policy source is read-only and machine-wide:

~~~text
HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>
    record    REG_SZ
~~~

The direct Windows adapter always opens the 64-bit registry view with only
`KEY_QUERY_VALUE` access. It accepts no current-user store, registry
virtualization fallback, environment-variable override, package-supplied path,
or application-supplied record. The key component must already be a valid
application ID, and the JSON field must exactly match it before package
validation begins.

`record` is UTF-16 `REG_SZ` JSON. It must contain one trailing NUL and no
embedded NUL, fit within 32 KiB of UTF-16 registry data, convert to valid Rust
text, and then meet the 16 KiB JSON record limit above. The adapter reads only;
it has no create, write, delete, registry enumeration, or installer API.

Windows normally limits writes below this `HKEY_LOCAL_MACHINE` location to
administrative installation or system management. A later installation service
must document how it provisions records and validates the key's access-control
policy. The read adapter trusts this machine-policy boundary but does not claim
that an administrator is untrusted.

The portable validator has a dedicated trusted-record entry point for this
operating-system source. It performs every package and executable check from
this document but does not accept a filesystem policy root, because the record
has already come from the machine registry. That entry point is for a native
policy adapter; it is not a public application API.

## Compatibility and failures

Records are exact at version 1.0 because they influence future process
authority. A compatible extension requires a new minor version, documentation,
and tests before acceptance. A breaking change requires a new major version.

The parser fails closed if the record is outside the selected policy root,
inside the package root, malformed, oversized, mismatched with the package, or
names an executable that is missing, too large, escapes the package, or does
not match its declared digest. Failure categories never include a raw path,
certificate subject, fingerprint, argument, bootstrap invitation, or native
error. The Windows adapter also fails closed for a missing registry key,
non-string or malformed registry value, malformed UTF-16, access denial, or a
record that changes while being read.

## Future launch sequence

The record parser is only the first input. Before `CreateProcessW`, the future
Windows launch service must repeat containment and digest checks, call the
Windows Authenticode adapter, compare its leaf fingerprint to the record,
create and track the child without shell interpretation, and only then deliver
the private bootstrap invitation. This work runs off the UI thread. A failure
leaves no child process and keeps the Startup Lab tile planned.

~~~text
trusted Windows policy directory
        |
        v
strict installed application record
        |
        +--> canonical package + validated application identity
        +--> canonical executable + SHA-256 digest
        `--> approved signer fingerprint
                    |
                    v
        future Authenticode comparison and tracked process launch
~~~

See `docs/SIGNING.md`, `docs/APPLICATIONS.md`, and Decision 0018.
