# Decision 0031: Windows UI preview is an explicit bounded developer tool

**Status:** Accepted

**Date:** 2026-08-01

## Context

The UI Lab proves a compiled-in document, while `anodrel-ui-document` proves
the external format in isolation. A developer needs a visible path that proves
the decoder, layout, native input, and renderer together before an
authenticated application session is introduced. Treating a command-line file
as a production package or a client session would incorrectly grant it
application identity and native authority.

## Decision

The Windows host adds `--ui-preview <path>`. It is available only through an
explicit local operator command. The host reads one regular file with a 64 KiB
limit, strictly decodes it before window construction, and renders it through
the existing direct native UI view. Its interaction remains local and semantic.

The command loads no manifest, assets, executable, URL, package, policy,
credential, or second file. It opens no pipe and exposes no protocol or
application event channel. The selected file carries no application identity,
permission, or capability.

## Consequences

- developers can visually validate a real external UI document without a
  browser, webview, framework runtime, or third-party dependency;
- invalid input fails before a native window is created; and
- the authenticated application-session boundary remains explicit rather than
  being accidentally replaced by a command-line diagnostic.

## Revisit conditions

Revisit before accepting documents through a package, pipe, protocol request,
network connection, watch service, drag-and-drop target, or any caller other
than an explicit local developer command.
