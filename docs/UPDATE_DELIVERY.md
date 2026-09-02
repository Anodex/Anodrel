# Windows update delivery

**Status:** Direct staging and locked image acceptance are implemented.
Catalogue discovery, user-visible update choice, elevation handoff composition,
installation, recovery proof, and automatic scheduling remain separate work.

## Purpose

`anodrel-windows-update-download` is a private native-host adapter that turns
one already CMS-verified, locally eligible catalogue candidate into one fresh,
bounded installer file. It is neither a protocol operation nor a general
download API. An application cannot select an endpoint, path, cache location,
or start it.

## Required sequence

~~~text
attached CMS catalogue, exact publisher
             |
             v
machine policy + installed Authenticode + newer version
             |
             v
one direct HTTPS GET to the signed image location
             |
             v
fresh cache file, streamed SHA-256 and exact byte count
             |
             v
locked Authenticode and exact-release acceptance
             |
             v
direct UAC handoff, then the installer update transaction
~~~

The preflight reloads the catalogue-selected installed application only after
the catalogue's CMS signature has verified. It reloads fixed machine policy,
requires the installed executable to pass Windows Authenticode, requires its
signer to match policy and catalogue, derives the installed version from its
canonical Anodrel release directory, and requires the catalogue version to be
strictly newer. It returns an opaque prepared value; a parsed but unsigned
catalogue cannot reach the downloader.

## Transfer and file rules

The caller supplies only an updater-owned, absolute existing cache directory.
The adapter canonicalizes it, rejects a link-like cache root, and creates one
previously absent regular `.exe` file within it. No catalogue, application,
protocol, command-line, or environment value chooses that directory or file
name.

The shared [direct HTTPS transport](HTTPS_TRANSPORT.md) makes one direct,
no-proxy, no-redirect, no-cookie, no-credential HTTPS `GET` to the exact
catalogue origin and path. It requires status `200`, streams no more than the
catalogue's declared length in 64 KiB chunks, and never retains the image in
memory. The update adapter writes and hashes each chunk as it arrives, flushes
the file, then checks the final byte count and SHA-256 against the opaque
catalogue descriptor.

Any preflight, transfer, write, synchronization, size, or digest failure
removes only that newly created cache file. A successful `DownloadedInstaller`
also deletes its private file on drop unless a later updater uses it while it
remains alive. The downloaded file is still *not* accepted for installation.
The next native gate re-verifies its Authenticode, embedded release, and
signed-catalogue facts while holding it against writes; see
[update handoff](UPDATE_HANDOFF.md). The elevated installer then re-verifies
itself and runs its existing transaction.

No resume, redirect, proxy, cookie, authentication, headers, response
metadata, arbitrary URL, cache enumeration, background transfer, scheduling,
notification, elevation, process launch, certificate creation, or installation
is part of this adapter.

See [update catalogue](UPDATE_CATALOGUE.md), [Windows installer](WINDOWS_INSTALLER.md), and Decisions 0165 through 0167.
