# Windows product launcher

**Status:** Windows product-launch contract. It is internal to Anodrel; it
does not add an SDK method, protocol operation, or application-controlled
process route.

## Purpose

An Anodrel application executable is an authenticated child. It cannot be a
Windows Start-menu target because the native host must first create its private
bootstrap invitation, authenticated pipe, and UI resource group. A product
release therefore includes one separate Anodrel Windows launcher: the host
executable that starts the verified child session.

## Signed descriptors

Release-manifest and release-plan version 1.4 add this exact object:

~~~json
"launcher": {
  "path": "bin/anodrel-windows-host.exe",
  "sha256": "64 lowercase hexadecimal characters"
}
~~~

The release plan supplies only `launcher.path`; `anodrel-release-manifest`
derives `launcher.sha256` from the matching checked bundle file. The path uses
the same strict contained `.exe` grammar as `executable.path`, must resolve to
a distinct regular file, and is never a command-line, URL, registry key, or
application-provided value.

The installer verifies both the selected application child and the launcher
against the release publisher before promotion. Record version 1.23 retains
both paths and digests. A record from version 1.22 or earlier has no launcher
and is not eligible for a Start-menu link.

## Fixed link and launch sequence

The all-users link under `Common Programs\\Anodrel` contains only:

| Shell Link value | Source |
| --- | --- |
| Target | selected record's verified launcher |
| Working directory | selected record's verified package root |
| Arguments | generated `--product-launch <selected application ID>` |
| Filename | selected record's signed `product.startMenuName` |

The launcher receives no record, package path, child path, capability,
bootstrap invitation, URL, or user-provided argument. It validates its own
canonical current executable against the selected record, locks and rehashes
that file, and checks its Authenticode publisher before it creates a window.
It then uses the existing product-session coordinator, which separately
re-reads and locks the selected child, verifies its digest and publisher, and
delivers the invitation over private standard input.

The self-check prevents a selected record from being served by a stale or
wrongly targeted host. It cannot authenticate code before that code executes;
the initial executable boundary remains the signed installer deployment into
the machine-owned Program Files tree and Windows executable trust controls.

## Migration and verification

Version 1.22's signed Start-menu filename remains useful metadata but no
longer authorizes a direct-child link. A post-policy synchronization with no
launcher removes an old link, including one created by the earlier writer.
When a selected v1.23 record replaces it, the new launcher link is persisted
before any stale filename is removed.

Automated coverage verifies strict format selection, digest derivation from the
bundle, record parsing and containment, rejection of a child-as-launcher,
generated fixed arguments, and direct Shell Link persistence. A signed
machine fixture is still required to prove Explorer starting the selected
launcher, followed by a real authenticated product window and clean child
shutdown. It is not run automatically because that fixture changes local
machine certificate trust. See [development diagnostics](DEVELOPMENT_DIAGNOSTICS.md).

See [installed records](LAUNCH.md), [product sessions](PRODUCT_SESSIONS.md),
[Start-menu registration](START_MENU.md), and Decision 0187.
