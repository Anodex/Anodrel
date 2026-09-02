# Windows update catalogue contract

**Status:** Implemented portable validation and direct Windows attached-CMS
signature foundation. A catalogue is not yet retrieved, written to disk, run,
elevated, or installed.

## Purpose

`anodrel.update-catalogue.v1` describes one future signed Windows installer
candidate. It gives the later updater an exact application identity, publisher,
release version, HTTPS origin, request path, and installer-byte descriptor to
check before it can hand an image to the existing installer gates.

The catalogue parser is owned, portable Anodrel code. It performs no file,
network, certificate, registry, process, window, or elevation operation.

## Exact format

The catalogue is strict UTF-8 JSON of at most **16 KiB** with exactly these
fields:

~~~json
{
  "formatVersion": { "major": 1, "minor": 0 },
  "applicationId": "org.example.product",
  "packageVersion": { "major": 1, "minor": 0, "patch": 0 },
  "publisher": {
    "leafCertificateSha256": "64 lowercase hexadecimal characters"
  },
  "installer": {
    "origin": { "host": "updates.example.test", "port": 443 },
    "path": "/releases/1.0.0/anodrel-windows-installer.exe",
    "byteLength": 123456,
    "sha256": "64 lowercase hexadecimal characters"
  }
}
~~~

The version is exactly 1.0. The application ID and publisher have the existing
Anodrel grammars. The release version has three unsigned 16-bit components.
The origin is one validated DNS hostname and non-zero HTTPS port. The request
path is an ASCII absolute path of at most 512 bytes ending in lowercase `.exe`;
it accepts only letters, digits, `/`, `-`, `_`, `.`, and `~`, with no query,
fragment, backslash, encoding escape, or repeated slash. The installer is at
least one byte and at most **576 MiB**, and its SHA-256 is lowercase hex.

Unknown, duplicate, missing, incorrectly typed, or out-of-range fields fail
closed. The parser exposes comparison operations and only an opaque image-byte
match; it does not expose the expected image digest as a display value.

## Intended trust sequence

~~~text
separately signed catalogue
          |
          v
exact installed identity + publisher + strictly newer version
          |
          v
exact HTTPS request, bounded image byte check
          |
          v
existing Authenticode and installer update gates
~~~

The parser itself is **not** a trust decision. The direct Windows signature
adapter uses attached CMS with exactly one valid signer certificate, requires
that signer's SHA-256 fingerprint to equal the installed publisher, and only
then returns an opaque verified catalogue value. It proves a signature from
that pinned publisher but does not independently establish certificate-chain
trust, timestamp validity, installed identity, version freshness, or installer
trust.

The direct update-download adapter refuses redirects, cookies, proxy discovery,
automatic credentials, arbitrary URLs, and content outside the declared size
and digest. It admits only CMS-verified catalogues that also pass fixed
installed identity, publisher, and newer-version preflight, and stages one
fresh private file. The existing installer must still independently check the
downloaded image's Authenticode signature, embedded publisher, payload, staged
executable, forward version, and machine policy before update. See
[update delivery](UPDATE_DELIVERY.md) and Decision 0167.

## Owned signing command

`anodrel-update-catalogue-sign` turns one absolute strict catalogue file into
one new synchronized attached-CMS file. Its exact command is:

~~~text
anodrel-update-catalogue-sign sign <catalogue.json> <certificate-sha256> <new-catalogue.p7s>
~~~

It selects only the exact lowercase SHA-256 fingerprint from the current
user's `MY` certificate store, includes that certificate with one CMS signer,
and verifies the output against the same fingerprint before reporting success.
It never overwrites a file, modifies its input, creates or imports a
certificate, creates trust, retrieves an update, installs, launches, elevates,
or starts a background service. The signer has no timestamp; an approved
timestamp and certificate-renewal policy remain production decisions.

No automatic schedule, endpoint discovery, key rotation, timestamp policy,
user notification, background service, or network route is introduced here.
Those require their own documented product and operating-system boundaries.

See [Windows installer](WINDOWS_INSTALLER.md), [signing](SIGNING.md), and
Decisions 0164 and 0165.
