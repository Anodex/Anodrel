# Decision 0200: Product-update progress stays native and bounded

**Status:** Accepted

**Date:** 2026-09-04

## Context

The installed-product update action safely discovers one signed candidate,
obtains native consent, streams its installer privately, requests fixed UAC
elevation, and proves the selected policy afterwards. The menu action becomes
disabled while that work runs, but a person cannot yet see whether a signed
transfer is making progress. A download byte count must not become a general
application event, network diagnostic, or source of unbounded paint work.

Windows supplies a direct taskbar progress API, but Windows hides that visual
indicator in high-contrast schemes and requires that a window receive its
`TaskbarButtonCreated` message before the API is called. A taskbar indicator
therefore cannot be the only user-facing representation.

## Decision

The verified primary product window owns one update-progress presentation. It
has two fixed host-controlled representations while an update attempt is live:

1. its native caption begins with a fixed Anodrel update status and retains
   the already composed, validated product caption as its suffix; and
2. after Windows has delivered `TaskbarButtonCreated`, its taskbar button
   receives one direct `ITaskbarList3` activity or determinate-progress state.

The update controller learns the total only from the CMS-verified candidate's
signed installer byte length. It records completed bytes only after a private
file write succeeds. The host converts that pair into a monotonic whole
percentage in `0..=100`, and changes the native caption or taskbar state only
when that visible state changes. It never uses HTTP content length, estimates a
speed or remaining time, or presents a byte count, endpoint, path, signer,
installer output, or operating-system failure.

Discovery and post-download verification/elevation are shown as fixed
indeterminate activity. The taskbar state is best effort and is cleared when
the attempt reaches any terminal result or its window ends. The caption remains
the direct high-contrast-safe progress representation. A later application
title proposal changes only the preserved validated base caption; it cannot
choose, suppress, or forge the native update status.

This stays outside the application protocol, SDK, UI document, menu model,
notification area, diagnostic ledger, and preferences. It does not add pause,
cancel, retry, scheduling, background transfer, automatic restart, release
notes, or application-visible progress.

## Consequences

- A person who started an update can see fixed native activity and signed-byte
  transfer progress without granting the application update authority.
- At most 101 determinate states are presented for one image, even though its
  private downloader may process many 64 KiB chunks. The UI thread performs
  only atomic reads, state comparison, and occasional native presentation.
- A taskbar reset or unavailable COM service reduces only that optional visual;
  it cannot change the update transaction or hide the native caption.
- Direct Windows COM is isolated in a small adapter and is never retained or
  invoked from the background transfer worker.

## Revisit conditions

Revisit for an owned client-area progress surface, accessibility events beyond
the native caption, pause or cancellation, a user-selected bandwidth policy,
time estimates, automatic updates, an update settings surface, another
platform, or production signed acceptance.
