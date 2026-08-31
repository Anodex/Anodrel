# Windows installer contract

**Status:** First-party release-manifest foundation. No production installer or
production signing identity is shipped yet.

## Purpose

The Windows installer will turn one signed Anodrel release into a machine
record the existing product host can safely launch. It is a small native
executable built from Anodrel modules and direct Windows APIs; it does not use
an installer framework, scripting host, webview, or runtime dependency.

The installer is not a new application capability. An installed application
cannot invoke it, choose a package directory, alter its own policy, or request
an update.

## Release envelope

The installer contains two private resources in its Authenticode-signed image:

~~~text
signed anodrel-windows-installer.exe
├── anodrel.release.v1 manifest
└── anodrel payload bytes
~~~

The manifest is not distributed beside the executable. Windows trust evaluation
must accept the installer before its embedded bytes become installation input.
The payload descriptor is checked before extraction; the owned bundle codec
defines its individual file records without compression.

The installer reads those two bytes from fixed `RT_RCDATA` resources in its own
loaded executable: manifest identifier `0xA141` and payload identifier `0xA142`.
It never accepts a sidecar path or resource selector. Resource loading alone is
not signature verification; the later signed-installer activation requires both
the self-signature check and the manifest/payload verification chain.

That activation checks the current installer executable with Windows
Authenticode, loads its fixed resources, and requires its accepted leaf
fingerprint to equal the manifest publisher fingerprint. A valid bundle under
an unsigned or differently signed executable is not an installable release.

`anodrel-release-image` builds those resources into a new unsigned installer
image with direct Windows APIs, then reloads the output to verify the exact
bytes. The production signing step follows assembly because resource changes
invalidate existing executable signatures.

## `anodrel.release.v1` manifest

The strict UTF-8 JSON manifest is at most **16 KiB**. Version 1.0 accepts only
the exact fields below. Unknown, missing, duplicate, or wrongly typed fields
are rejected.

~~~json
{
  "formatVersion": { "major": 1, "minor": 0 },
  "applicationId": "org.example.product",
  "packageVersion": { "major": 1, "minor": 0, "patch": 0 },
  "executable": {
    "path": "bin/example-product.exe",
    "sha256": "64 lowercase hexadecimal characters"
  },
  "publisher": {
    "leafCertificateSha256": "64 lowercase hexadecimal characters"
  },
  "capabilities": ["ui.document.write", "session.close"],
  "networkOrigins": [],
  "payload": {
    "byteLength": 123456,
    "sha256": "64 lowercase hexadecimal characters"
  }
}
~~~

| Field | Rule |
| --- | --- |
| `formatVersion` | Exactly `{ "major": 1, "minor": 0 }`. A later version must be a documented strict superset. |
| `applicationId` | Existing 3–128 character Anodrel application identity. |
| `packageVersion` | Three non-negative integers from 0 through 65,535; it identifies a staged release directory and is not protocol compatibility. |
| `executable.path` | Relative, forward-slash-separated contained `.exe` path; no roots, drives, `.` or `..`. |
| `executable.sha256` | Lowercase SHA-256 of the extracted executable. |
| `publisher.leafCertificateSha256` | Lowercase SHA-256 leaf fingerprint the installer and extracted executable must both match. |
| `capabilities` | Unique installed-record grant names. The installer renders them into a version-1.19 machine record, then asks the existing validator to accept it. |
| `networkOrigins` | Exact host/port policy. It is empty unless `capabilities` includes `network.fetch`; the existing installed-record validator is authoritative. |
| `payload` | Exact uncompressed byte length, at least 1 and at most 512 MiB, plus a lowercase SHA-256 digest. |

The manifest contains no filesystem root, command line, registry location,
download URL, certificate subject, token, user data, or child argument.

## Current implementation

`native/tools/windows-installer` now implements the read-only `validate`
foundation. It bounds and parses one release manifest, validates its executable
and payload descriptors, then requires the complete payload to match before its
owned per-file bundle decoder runs. It canonicalizes permitted network origins
and renders the existing version-1.19 installed-record shape for later host-side
validation. The direct Windows resource reader selects only the two fixed
current-image resources and fails closed when they are absent. Its contract tests
prove that the rendered record passes the same `anodrel-application` validator
the Windows host reads. The activation gate now asks Windows to verify the
current installer image before it reads those resources, and rejects an unsigned
test image through the real Authenticode path.

The command-line tool has no `install` or `uninstall` command yet. It cannot
write a production package directory or the registry, create machine trust, or
launch an application. Its library now has the private staged-extraction
boundary below, but no command can select a staging parent or invoke it. The
later signer check, promotion, policy publication, recovery, and uninstall
operations remain separate boundaries.

## Staged extraction contract

The owned staged-extraction module accepts only an internal absolute staging
parent selected by the elevated installer. It creates one new private,
unpublished staging directory below that parent and never writes into a version
directory or a path supplied by a release, command line, or application.

For each already checked bundle entry it derives the relative Windows path from
the canonical `/` components, rejects device names, trailing dots or spaces,
reserved path characters, and overlong output paths, then creates a new regular
file. It syncs and rehashes that file before continuing. Existing files,
directories, links, and registry values are never reused as staging input.

After every bundle file is present, the installer renders the version-1.19
record for the staging root and runs the existing installed-record validator.
That independently checks the application manifest, content digest, executable
containment, executable digest, application identity, capabilities, and network
policy. The later signer check, atomic promotion, registry publication,
recovery, and uninstall boundaries remain separate.

## Planned machine installation

Version 1 is a machine installation. The installer owns the destination under
`Program Files\Anodrel\Applications\<applicationId>\<packageVersion>` and the
existing registry value:

~~~text
HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>
    record    REG_SZ
~~~

It accepts neither as a command-line argument. The registry remains the product
host's only policy source.

1. Verify the installer executable with Windows Authenticode, then verify that
   its accepted leaf fingerprint matches the embedded manifest.
2. Hash the embedded payload, extract it into a new private staging directory,
   and reject any file outside the destination tree.
3. Check the extracted application package, executable path, executable digest,
   and executable Authenticode fingerprint. It must match the installer.
4. Compose the proposed version-1.19 installed record and validate it through
   `anodrel-application` before any registry write.
5. Rename the verified staging directory to its version directory, atomically
   publish the one registry `record` value, and retain the prior complete
   directory until a later cleanup pass.

The host therefore follows a current valid record throughout an update. A crash
before the registry write leaves the previous record intact; a crash after it
leaves a complete new version directory selected by that record. A later
installer command will recover only Anodrel-owned stale staging directories.

## Commands and exclusions

The future installer has only `install`, `uninstall`, and `verify` commands.
`install` and `uninstall` need elevation; `verify` is read-only. All commands
select the embedded identity only. They do not accept an arbitrary executable,
package root, registry path, policy, capability, certificate, or network URL.

Initial release work deliberately excludes automatic download, background
updates, key rotation, shortcuts, file associations, service installation, and
notifications. Each adds a separate user-visible or trust boundary.

## Production decision still required

This contract does not select a certificate authority or hold a private key.
Before a public release, the product owner must choose the production signing
certificate, its custody and renewal process, and whether that identity is
compatible with the desired Windows distribution channel.

See [Windows release readiness](WINDOWS_RELEASE.md), [signing foundation](SIGNING.md),
[installed application records](LAUNCH.md), and Decisions 0017–0020 and 0140.
