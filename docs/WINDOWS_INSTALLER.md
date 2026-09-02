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
the exact fields below. Version 1.1 adds exactly one `updateCatalogue` field.
Version 1.2 adds signed product display metadata while retaining that catalogue
field. Unknown, missing, duplicate, or wrongly typed fields are rejected.

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
| `formatVersion` | Exactly `{ "major": 1, "minor": 0 }`, 1.1, or 1.2. Versions 1.1 and 1.2 require `updateCatalogue`; 1.2 also requires `product`; version 1.0 has no discovery authority. |
| `applicationId` | Existing 3–128 character Anodrel application identity. |
| `packageVersion` | Three non-negative integers from 0 through 65,535; it identifies a staged release directory and is not protocol compatibility. |
| `executable.path` | Relative, forward-slash-separated contained `.exe` path; no roots, drives, `.` or `..`. |
| `executable.sha256` | Lowercase SHA-256 of the extracted executable. |
| `publisher.leafCertificateSha256` | Lowercase SHA-256 leaf fingerprint the installer and extracted executable must both match. |
| `capabilities` | Unique installed-record grant names. The installer renders them into a version-1.19, 1.20, or 1.21 machine record, then asks the existing validator to accept it. |
| `networkOrigins` | Exact host/port policy. It is empty unless `capabilities` includes `network.fetch`; the existing installed-record validator is authoritative. |
| `updateCatalogue` | Versions 1.1 and 1.2: one exact `origin` and canonical `.p7s` path for signed product-update metadata. It is unrelated to application `network.fetch` authority. See [update discovery](UPDATE_DISCOVERY.md). |
| `product` | Version 1.2 only: one signed `displayName` and `publisherName` for later host-owned Windows product surfaces. Neither value is a path, policy key, identity, or certificate selector. See [signed product display metadata](PRODUCT_METADATA.md). |
| `payload` | Exact uncompressed byte length, at least 1 and at most 512 MiB, plus a lowercase SHA-256 digest. |

The manifest contains no filesystem root, command line, registry location,
image download URL, certificate subject, token, user data, or child argument.
Version 1.1's bounded update-catalogue source is the sole documented network
location and it enters the fixed machine record only through installer gates.

## Current implementation

`native/tools/windows-installer` now implements the read-only `validate`
foundation. It bounds and parses one release manifest, validates its executable
and payload descriptors, then requires the complete payload to match before its
owned per-file bundle decoder runs. It canonicalizes permitted network origins
and renders version 1.19, 1.20, or 1.21 installed-record policy for later
host-side validation. Version 1.20 retains the signed release's optional strict
catalogue source, not an application-provided or arbitrary download route;
version 1.21 additionally retains signed product display metadata for a later
host-owned Windows surface. The direct Windows resource reader selects only the two fixed
current-image resources and fails closed when they are absent. Its contract tests
prove that the rendered record passes the same `anodrel-application` validator
the Windows host reads. The activation gate now asks Windows to verify the
current installer image before it reads those resources, and rejects an unsigned
test image through the real Authenticode path.

The library can then stage only a checked release under an installer-owned
parent, revalidate its package and rendered record, and call Windows
Authenticode for the staged executable before it is promotion-ready. An unsigned
staged executable fails closed in the direct test; the matching-signer positive
path remains an operator fixture until a signed resource-bearing installer and
application are available.

The owned promotion path moves only a prepared stage to its absent signed
version-directory sibling through direct `MoveFileExW`, with no copy or
replacement flags. Its direct tests prove both the successful move and that an
existing version stays untouched. The following direct Advapi32 boundary then
writes only its already validated record to the fixed 64-bit machine-policy
location; it is unit-tested for exact key/value and UTF-16 representation, but
its elevated signed-fixture path remains an operator check.

The installer core exposes only fixed `verify`, `install`, `update`, and
`uninstall` operations through its thin first-party `windows-installer-shell`.
Each machine-changing operation first asks Windows
whether its current process token is elevated and fails closed without trying
to self-elevate. It accepts no target, root, record, publisher, certificate,
or network argument. The separately reviewable staging, signer, promotion,
publication, recovery, uninstall, and update boundaries remain behind those
fixed commands.

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

After every bundle file is present, the installer renders the version-1.19,
1.20, or 1.21 record for the staging root and runs the existing
installed-record validator.
That independently checks the application manifest, content digest, executable
containment, executable digest, application identity, capabilities, and network
policy. The signer gate then verifies the staged executable through Windows
Authenticode and compares its opaque accepted leaf fingerprint to the same
embedded publisher value before it becomes promotion-ready. Atomic promotion,
registry publication, recovery, and uninstall remain separate.

## Promotion contract

A promotion-ready stage can become only its signed three-part version directory
under the same installer-owned application root. The destination must be
absent. The owned boundary uses `MoveFileExW` with neither copy nor replacement
flags, so it cannot overwrite an existing version or fall back to a
cross-volume copy-and-delete operation.

Promotion itself publishes no machine policy. If it fails, the stage stays
unpublished and its owner removes it. If it succeeds but a later registry
publication fails or the machine stops, the resulting complete version remains
unselected; the prior registry record is still the host's only launch policy.
Recovery may clean only such Anodrel-owned stale directories after a separate
decision.

## Machine-policy publication contract

Only an opaque promotion result can write the fixed 64-bit `record` `REG_SZ`
value below `HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>`.
The record was constructed from the signed release and validated against the
complete promoted package before publication. The writer exposes no hive, key,
value, record-text, or delete option to a command line or application.

If the Windows registry write fails, the existing record remains selected while
the new complete version is unselected. The later recovery boundary decides
whether that stale version can be safely removed; this publication step never
tries to roll back by deleting content.

## Recovery discovery contract

The owned recovery scanner accepts only an installer-selected absolute
application root. It discovers, but does not delete, normal child directories
with Anodrel's exact private staging name format. Version directories, files,
links, and unknown names are never candidates. A later direct deletion boundary
consumes only those discovered private directories and refuses reparse points
while walking them. It removes normal files then normal directories, never a
version directory or a caller-named path. A partial failed cleanup remains
unselected and may be retried.

## Uninstall preflight contract

Uninstall starts with the current signed installer release, then loads only its
fixed machine record and requires the installed executable's accepted signer to
match both that record and the release publisher. It accepts no identity, root,
registry, or executable argument and deletes nothing. Later record removal and
tree cleanup consume only this opaque verified result.

The next owned step removes only the fixed `record` value from that application's
64-bit machine key. It neither deletes the key nor any package file. The later
tree cleanup receives only its opaque policy-removed result.

Package cleanup consumes only that policy-removed result. It uses the same
direct normal-tree remover as private-stage recovery and refuses reparse points.
It does not remove application data or credentials; an incomplete cleanup stays
unselected for a later recovery route.

## Update-candidate preflight contract

Before later update retrieval or installation work can use a candidate, the
current signed installer release is compared to the fixed selected installed
record for the same embedded application identity. Windows must accept the
installed executable's Authenticode signature; that signer must match both the
installed record and the candidate release publisher. The selected package root
must end in the exact canonical owned version-directory form `major.minor.patch`.
The candidate version must be strictly newer.

This is a read-only anti-rollback and publisher-continuity gate. It has no
network, URL, file, registry, process, background-service, or user-interface
input. It does not prove the candidate application's embedded executable until
the separate staging signer gate runs, and it does not perform an update.

## Machine update transaction contract

The installer library composes an update only from a current candidate that
passed the preflight above. Before selecting a machine root or writing any
file, it activates its current signed release again and requires its identity,
package version, and publisher to match the candidate decision. It then uses
the same private preparation, staged-executable signer, no-overwrite promotion,
and fixed policy-publication boundaries as initial installation.

An update transaction accepts no parameter and creates no download, background
service, user-data directory, process, trust, shortcut, file association, or
network connection. The already-selected version directory is retained after
the fixed record points at the new complete version. Before it selects that new
record, it copies the current fixed record into one private `previous` policy
value. Publisher-key rotation and the later rollback command remain separate
decisions.

## Machine rollback transaction contract

The installer may roll back only to that one retained `previous` record. Its
preflight activates the current signed installer release, then validates both
the selected and retained records as complete application packages. Windows
must accept both executable signatures; each signer must match its own record
and the current installer publisher. Both package roots must be direct
canonical version children of the fixed machine application root, and the
retained version must be strictly lower than the selected version.

Only that opaque result can copy the fixed retained record back to `record`.
It accepts no version, path, identity, registry value, publisher, package, or
network input. It neither deletes a version directory nor starts a process;
the retained `previous` value stays in place until a later update replaces it.

## Machine installation transaction contract

The installer library composes one machine installation only from its current
signed embedded release. It first activates that signed release to obtain the
validated application identity and rejects any existing selected machine policy.
An existing installation must use the separately gated update path, preventing
this first-install route from rolling a selected release back. It then derives
the fixed machine root and activates the current release again as preparation
starts. Preparation privately stages and checks the package and executable,
promotion uses its existing no-overwrite same-volume rename, and publication
writes the existing fixed policy record.

The transaction accepts no parameter. It creates no trust, shortcut, file
association, service, process, network connection, updater, or user-data
directory. An unsigned current executable fails before Program Files selection.
If promotion succeeds but policy publication fails, the complete version stays
unselected while the prior policy remains authoritative; the transaction never
replaces a version or deletes existing content to hide that failure.

## Planned machine installation

Version 1 is a machine installation. The installer owns the destination under
`Program Files\Anodrel\Applications\<applicationId>\<packageVersion>` and the
existing registry value:

~~~text
HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>
    record    REG_SZ
~~~

The 64-bit installer derives `Program Files` from Windows'
`FOLDERID_ProgramFilesX64` known folder rather than an environment variable or
hard-coded drive. It creates and verifies only normal, non-reparse
`Anodrel\Applications\<applicationId>` directories below it, where the final
component is the signed manifest identity. A 32-bit installer is not a Version
1 distribution target because it cannot reliably select that 64-bit known
folder. Administrators remain inside the machine-trust boundary.

It accepts neither as a command-line argument. The registry remains the product
host's only policy source.

1. Verify the installer executable with Windows Authenticode, then verify that
   its accepted leaf fingerprint matches the embedded manifest.
2. Hash the embedded payload, extract it into a new private staging directory,
   and reject any file outside the destination tree.
3. Check the extracted application package, executable path, executable digest,
   and executable Authenticode fingerprint. It must match the installer.
4. Compose the proposed version-1.19, 1.20, or 1.21 installed record and validate it through
   `anodrel-application` before any registry write.
5. Rename the verified staging directory to its version directory, atomically
   publish the one registry `record` value, and retain the prior complete
   directory until a later cleanup pass.

The host therefore follows a current valid record throughout an update. A crash
before the registry write leaves the previous record intact; a crash after it
leaves a complete new version directory selected by that record. A later
installer command will recover only Anodrel-owned stale staging directories.

## Commands and exclusions

The installer has only `install`, `update`, `rollback`, `uninstall`, and
`verify` commands. `install`, `update`, `rollback`, and `uninstall` need
elevation; `verify` is read-only. All commands select the embedded identity
only. They do not accept an arbitrary executable, package root, registry path,
policy, capability, certificate, or network URL.

`install`, `update`, `rollback`, and `uninstall` need elevation; the installer
does not trigger a UAC prompt or relaunch itself. An operator explicitly starts
the signed executable from an elevated shell. `verify` is read-only and can
show that the current signed embedded release was accepted without writing
machine state. The development-only `validate-manifest <path>` command still
validates one sidecar manifest and cannot write machine state.

The installer now also exposes an opaque initial-install preflight and matching
postcondition proof for a later native consent/UAC surface; see
[initial-install acceptance](INSTALL_ACCEPTANCE.md). A direct native, no-by-default
confirmation may turn that prepared result into an opaque approval; see
[initial-install consent](INSTALL_CONSENT.md). Only that approval can enter a
fixed direct UAC handoff, whose zero exit still requires the postcondition
proof; see [initial-install handoff](INSTALL_HANDOFF.md). The current
no-argument command now composes those stages for an initial installation; see
[interactive initial-install flow](INSTALL_FLOW.md). Named machine-changing
commands still require an explicitly elevated shell.

Initial release work deliberately excludes automatic download, background
updates, key rotation, shortcuts, file associations, service installation, and
notifications. The signed catalogue source, CMS verification, and private image
staging foundations are implemented; elevation handoff and installation remain
separate trust boundaries.

## Production decision still required

This contract does not select a certificate authority or hold a private key.
Before a public release, the product owner must choose the production signing
certificate, its custody and renewal process, and whether that identity is
compatible with the desired Windows distribution channel.

See [Windows release readiness](WINDOWS_RELEASE.md), [signing foundation](SIGNING.md),
[installed application records](LAUNCH.md), [initial-install acceptance](INSTALL_ACCEPTANCE.md),
[initial-install consent](INSTALL_CONSENT.md), [initial-install handoff](INSTALL_HANDOFF.md),
[interactive initial-install flow](INSTALL_FLOW.md), and Decisions 0017–0020,
0140, and 0177–0181.
