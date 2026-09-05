# Windows product-update progress

**Status:** Native product progress is host-owned. It is not an application
capability or a general download display.

## What a person sees

After selecting **Check for Anodrel updates**, the verified product window
shows only fixed Anodrel status in its native caption:

- `Checking for Anodrel updates` while signed discovery is in progress;
- `Downloading Anodrel update — N%` while the selected image is streamed; and
- `Installing Anodrel update` after the image is ready for its fixed
  verification, elevation, and policy-proof stages.

The existing validated product caption remains after this fixed prefix. If the
application later proposes a title, Anodrel recomposes the suffix exactly as
normal before it redraws the same update prefix. The application is never told
that an update is active and cannot alter the status.

When Windows has created the product window's taskbar button, Anodrel also
uses the direct Windows taskbar progress API. It shows activity during checking
and installation and a determinate bar during download. Windows may suppress
that bar in high-contrast mode or when its taskbar service is unavailable; the
native caption remains the reliable visible representation.

## Exact progress rule

The total is the byte length in the already CMS-verified update catalogue. A
completed byte count advances only after the private image file accepted a
write. The host rounds that signed pair down to a whole percentage, retains it
monotonically in `0..=100`, and updates presentation only when the percentage
or phase changes.

No HTTP `Content-Length`, speed, remaining time, image path, catalogue URL,
certificate, installer output, Windows status, progress event, callback,
application field, protocol response, or SDK method is exposed. A blocked or
failed transfer reports only the existing fixed terminal result.

## Lifecycle and performance

The transfer worker writes and records progress. The product window's existing
low-frequency timer only loads that small state, compares it with the previous
visible state, and makes a native presentation call when it changes. It never
waits for a network read, file write, image verification, UAC process, or
policy check. A 576 MiB image can produce thousands of private chunks but at
most 101 percentage presentations.

The taskbar API is called only after the system's `TaskbarButtonCreated`
message. Its COM object is short-lived on the product window's UI thread; it
is not shared with, or called by, a worker. Terminal results and destruction
clear any taskbar progress and restore the ordinary validated window caption.

The feature has no pause, cancellation, retry, bandwidth control, background
transfer, scheduling, automatic restart, or settings route. See Decision 0200
and [product updates](PRODUCT_UPDATES.md).
