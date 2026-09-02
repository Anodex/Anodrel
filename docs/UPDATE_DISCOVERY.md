# Windows update catalogue discovery

**Status:** The signed installed-policy contract now composes with the native
cache, consent, handoff, and postcondition boundaries. A user-visible host
action and a signed end-to-end acceptance run remain separate work.

## Purpose

An updater must not obtain a catalogue URL from an application, command line,
environment, registry value outside Anodrel policy, or a mutable release
directory. A release manifest at version 1.1 may instead contain one exact
`updateCatalogue` HTTPS location. During installation the existing signed
installer writes that location into the fixed machine record at version 1.20.

Older release-manifest and machine-record versions remain valid but have no
catalogue-discovery authority. They cannot enter the update retrieval path.

## Exact location

The field has only this shape:

~~~json
"updateCatalogue": {
  "origin": { "host": "updates.example.test", "port": 443 },
  "path": "/anodrel/catalogues/stable.p7s"
}
~~~

The origin uses Anodrel's existing exact DNS hostname and non-zero HTTPS port
grammar. The request path is an ASCII absolute path of at most 512 bytes,
ending in lowercase `.p7s`, with letters, digits, `/`, `-`, `_`, `.`, and `~`
only. It has no query, fragment, encoded character, repeated slash, backslash,
or `.`/`..` component.

## Retrieval sequence

~~~text
host-selected installed application identity
              |
              v
fixed machine record with signed update-catalogue location
              |
              v
installed executable Authenticode and record-publisher comparison
              |
              v
one direct bounded HTTPS CMS download and exact-publisher verification
              |
              v
existing newer-candidate preflight and private image staging
~~~

The installed application identity must come from native updater composition,
such as the verified application identity already selected for that host. It is
never accepted from an application protocol request. Before transfer, the
adapter verifies the selected installed executable with Windows Authenticode and
requires that signer to match its machine record. It retrieves at most 128 KiB
from exactly the installed location through the shared direct HTTPS transport,
then verifies the CMS against that same signer. The returned candidate still
must pass the current identity, publisher, and newer-version preflight before
image download.

No fallback endpoint, redirect, cache, proxy, application network grant,
background task, schedule, notification, elevation, launch, or install route
is introduced here.

See [update catalogue](UPDATE_CATALOGUE.md), [update delivery](UPDATE_DELIVERY.md), and Decisions 0167 and 0168.
