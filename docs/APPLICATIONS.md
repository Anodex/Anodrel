# Anodrel application packages v1

**Status:** Foundation contract. The Windows host implements the local text surface described here. It does not yet launch an application process, run scripts, provide a webview, or expose a native bridge.

## Purpose and boundary

An application package gives the host one explicit, validated identity for the content it displays. The package is a local directory containing a manifest and one bounded text document. The host obtains the application ID only from the validated manifest; rendered text cannot declare or change it.

This is a content-identity boundary, not publisher authentication. A person who can replace both an unpackaged manifest and its content can create a new package with a different valid digest. Windows now has an isolated embedded-executable signature verifier, but it does not authenticate this mutable package or grant launch authority. Production package signing, publisher trust, executable containment and digest checks, updates, process launch, and permissions are intentionally deferred. The current development Node sample is not an application package and must not be treated as one.

## Layout

The conventional layout is:

~~~text
application-directory/
|- anodrel.application.json
`- content/
   `- main.txt
~~~

The Windows host accepts a manifest path supplied by its operator. It resolves that path and the content path to canonical local paths before reading either file. A content path that is absolute, has a drive or root prefix, contains `.` or `..`, or resolves outside the manifest directory is rejected. This also prevents content symlinks from escaping the package directory.

## Manifest format

A manifest is strict UTF-8 JSON no larger than **16 KiB**. Version 1.0 accepts exactly the fields below: unknown, missing, duplicate, and wrongly typed fields are rejected.

~~~json
{
  "manifestVersion": { "major": 1, "minor": 0 },
  "applicationId": "org.anodrel.sample",
  "displayName": "Anodrel Sample",
  "content": {
    "format": "anodrel.text.v1",
    "path": "content/main.txt",
    "sha256": "64 lowercase hexadecimal characters"
  }
}
~~~

| Field | Rule |
| --- | --- |
| `manifestVersion` | Object with numeric `major: 1` and `minor: 0`. |
| `applicationId` | 3–128 ASCII characters: lowercase letters, digits, `.`, `-`, or `_`; it must begin and end with a lowercase letter or digit. |
| `displayName` | Non-empty UTF-8 text, at most 80 bytes; control characters are rejected. |
| `content.format` | Exact string `anodrel.text.v1`. |
| `content.path` | A relative package path following the containment rules above. |
| `content.sha256` | The lowercase hexadecimal SHA-256 digest of the raw content bytes. |

The digest algorithm is implemented in Anodrel. It is a content integrity check, not a secret or a substitute for publisher signing.
Package distribution must preserve the declared raw content bytes. The sample
package marks its digest-verified text as `-text` in `.gitattributes` so line
ending conversion cannot change its digest during checkout.

## The text surface

`anodrel.text.v1` is intentionally small. It is UTF-8 plain text that the host draws directly in its own native window. It contains no HTML, JavaScript, CSS, URLs, links, navigation, resource fetches, forms, or application-to-host bridge.

Before drawing the host verifies the digest and rejects content larger than **8 KiB**, more than **4,096 Unicode scalar values**, more than **128 lines**, more than **160 scalar values on one line**, or any control character other than line feed (`U+000A`). The host adds its own application name and identity header; content does not control the native window title or host diagnostics.

The surface is suitable only for verifying package loading and content containment. It is not a general UI toolkit or a browser replacement.

## Compatibility and failure behavior

Manifest 1.0 is intentionally exact because it controls a security boundary. A compatible manifest extension requires a new minor version, documentation, and a test before a host accepts it. A semantic or structural breaking change requires a new major version. Version 1.0 hosts reject every version other than 1.0 rather than guessing how to interpret it.

The host fails closed before creating a content window when the manifest, package containment, text restrictions, or digest check fails. It does not fall back to unverified content. Safe diagnostics may identify the failed validation category, but must not include bootstrap credentials or raw application content.

## Current lifecycle

~~~text
operator-selected manifest
        |
        v
canonical package containment + strict manifest validation
        |
        v
bounded content read + SHA-256 verification
        |
        v
claim application-scoped host instance
        |
        +-- existing --> bounded activation request --> exit
        |
        `-- primary --> host-rendered Win32 text surface
~~~

`docs/INSTANCE_LIFECYCLE.md` defines the exact Windows coordination behavior.
No application code runs in this path. A later process-launch decision must
bind a verified executable and its session to this validated application ID; it
must not reuse the development bootstrap launcher as authority. The Windows
signature foundation in docs/SIGNING.md is only one input to that later
host-controlled policy.
