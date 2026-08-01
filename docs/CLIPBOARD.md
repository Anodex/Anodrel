# Anodrel clipboard foundation

**Status:** Portable text values and the direct Windows adapter are implemented;
the capability-checked protocol surface remains deferred.

## Boundary

Anodrel's first clipboard service carries plain UTF-8 text only. The platform
maps it to the operating system's ordinary Unicode-text clipboard format. It
does not expose raw clipboard formats, HTML, images, file lists, delayed
rendering, change notifications, ownership callbacks, native handles, or other
applications' metadata.

The portable service interface is deliberately small:

~~~text
ClipboardService
  read_text() -> ClipboardRead
  write_text(text) -> success | ClipboardError
~~~

`ClipboardRead` distinguishes an empty Unicode-text clipboard from a clipboard
that has no supported text representation. It never returns a native error,
format identifier, source application name, or raw handle.

## Limits and validation

- Text is valid UTF-8 and has at most **64 KiB** when encoded.
- Writing rejects invalid or over-limit text before opening the clipboard.
- Reading rejects an over-limit operating-system value without returning a
  partial result.
- The service makes one bounded attempt; contention is reported as a stable
  unavailable category, with no retry loop or blocking wait.

## Authority and protocol

Clipboard reading and writing are separate host-issued capabilities:
`clipboard.read` and `clipboard.write`. A protocol operation must check its
capability immediately before touching the operating system, use a bounded
string payload/result, and return only safe error categories. No current
protocol operation grants clipboard authority.

## Windows mapping

The Windows adapter uses only User32 and Kernel32 APIs. It opens the clipboard
for the host's current native window, reads or writes `CF_UNICODETEXT`, and
uses a movable global-memory allocation only during the documented ownership
transfer to Windows. It frees memory itself only when that transfer fails.
The adapter closes the clipboard on every path and never retains the native
handle after a call.

The direct adapter is host-only. It accepts the current host window only as an
opaque transient owner value, never exposes it to an application, and maps all
native failures to `Unavailable`, `StoredTextInvalid`, or `StoredTextTooLarge`.

## Security and privacy

Clipboard data is user-controlled and must not enter logs, diagnostics,
exception text, persistent host state, or application capability context.
Writing text replaces the system's current clipboard contents only after the
full bounded value has been validated and copied. An application cannot request
another application, window, format, or clipboard-history entry.

## Deferred

Clipboard events, history, rich text, images, files, custom formats, primary
selection, drag-and-drop integration, persistent ownership, and consent UI are
outside this foundation.
