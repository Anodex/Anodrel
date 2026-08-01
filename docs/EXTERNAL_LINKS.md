# Anodrel external-link foundation

**Status:** Portable HTTPS link validation, the direct Windows opener, and the
capability-checked Protocol 1.6 surface are implemented.

## Boundary

Anodrel's first external-link service opens one validated HTTPS address through
the operating system's ordinary associated handler. It does not expose an
executable, command line, shell, browser selection, profile, referrer, working
directory, file path, custom scheme, network request, or handler result.

The portable boundary is deliberately small:

~~~text
ExternalLink::parse(url) -> ExternalLink | ExternalLinkInputError
ExternalLinkService::open(link) -> success | ExternalLinkOpenError
~~~

## Validation and limits

- An address is ASCII and at most **2,048 bytes**.
- It begins exactly with `https://`.
- Its authority has one ASCII DNS-style hostname and an optional numeric port
  from 1 through 65,535.
- User information, backslashes, whitespace, controls, empty labels, invalid
  labels, and malformed ports are rejected before a native call.
- The path, query, and fragment remain opaque validated URL text; Anodrel does
  not fetch, resolve, rewrite, or log them.

## Windows mapping

The Windows adapter uses the direct Shell32 `ShellExecuteW` API with no verb,
parameters, working directory, or retained process handle. Windows chooses the
user's associated HTTPS handler. The adapter returns only `Unavailable` when
the system cannot accept the request; it does not expose a handler name or
native status code.

## Security and privacy

An external link can lead a user away from the application. Protocol 1.6 maps
one exact `{ "url": string }` payload to `external.open`, which requires the
separate `external.open` host-issued capability. It is never treated as a file
path, shell command, executable, callback, navigation instruction, or native
handle. Its full text must not enter diagnostics, errors, persistent host state,
or logs. The host checks that capability immediately before handing the
previously validated value to the operating-system adapter.

## Deferred

HTTP, custom schemes, file links, application links, handler selection,
embedded browsing, confirmation UI, request metadata, callback delivery,
history, and non-Windows adapters require separate contracts and reviews.
