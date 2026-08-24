//! Validation and event types for one session-local native window.

import {
  MAX_UI_DOCUMENT_REQUEST_BYTES,
  MAX_WINDOW_CLIENT_HEIGHT,
  MAX_WINDOW_CLIENT_WIDTH,
  MIN_WINDOW_CLIENT_HEIGHT,
  MIN_WINDOW_CLIENT_WIDTH,
} from "../index.js";
import type {
  EventEnvelope,
  PayloadFor,
  SecondarySessionWindowId,
  SessionWindowId,
  UiActionInvokedEvent,
} from "../index.js";
import { isRecord } from "./primitives.js";

/** Maximum UTF-16 code units an application may propose for its window title. */
export const MAX_WINDOW_TITLE_UTF16_UNITS = 96;

/**
 * Returns whether a value is exactly one window-title proposal.
 *
 * An extra field is a mismatch rather than something to ignore, so a future
 * window target, identifier, position, or size cannot be smuggled past protocol
 * 1.14 — which is what keeps this capability impossible to aim at another
 * window.
 *
 * Every control character is rejected, with no exception for a line feed. A
 * title is a label rendered on one line, so a newline could split one window's
 * title into what reads as two, or push the visible text away from the host's
 * application-name suffix.
 */
export function isWindowTitleSetPayload(
  value: unknown,
): value is PayloadFor<"window.title.set"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    isWindowTitleProposal(value.title)
  );
}

/** Validates the exact Protocol 1.25 secondary-window creation payload. */
export function isWindowOpenPayload(value: unknown): value is PayloadFor<"window.open"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isWindowTitleProposal(value.title) &&
    typeof value.document === "string" &&
    new TextEncoder().encode(value.document).byteLength <= MAX_UI_DOCUMENT_REQUEST_BYTES
  );
}

/** Validates one exact currently issued secondary-view close request shape. */
export function isWindowClosePayload(value: unknown): value is PayloadFor<"window.close"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    isCanonicalSecondaryWindowId(value.windowId)
  );
}

/** Validates one exact strict-v1 update targeted at a known session view. */
export function isUiDocumentReplaceWindowPayload(
  value: unknown,
): value is PayloadFor<"ui.document.replace.window"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isCanonicalSessionWindowId(value.windowId) &&
    typeof value.document === "string" &&
    new TextEncoder().encode(value.document).byteLength <= MAX_UI_DOCUMENT_REQUEST_BYTES
  );
}

/** One current enabled semantic command selected from a native session menu. */
export interface MenuActionInvokedEvent
  extends EventEnvelope<{ readonly menuRevision: string; readonly action: string }> {
  readonly eventName: "menu.action.invoked";
  readonly source: "native.menu";
  readonly schemaVersion: { readonly major: 1; readonly minor: 18 };
}

/** A semantic interaction delivered by the bounded `ui.events.read` result. */
export type UiInteractionEvent = UiActionInvokedEvent | MenuActionInvokedEvent;

/** A UI or primary-menu action tagged with the logical view that produced it. */
export type WindowUiInteractionEvent =
  | (UiActionInvokedEvent & { readonly windowId: SessionWindowId })
  | (MenuActionInvokedEvent & { readonly windowId: SessionWindowId });

/**
 * Returns whether a value is exactly one closed session-window state request.
 *
 * An extra field is a mismatch rather than something to ignore, so a future
 * target, identifier, geometry, focus option, or native command cannot be
 * smuggled past Protocol 1.16.
 */
export function isWindowStateSetPayload(
  value: unknown,
): value is PayloadFor<"window.state.set"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    (value.state === "minimized" || value.state === "maximized" || value.state === "restored")
  );
}

/**
 * Returns whether a value is exactly one closed reversible fullscreen request.
 *
 * An extra field is a mismatch rather than a future monitor, display-mode,
 * geometry, style, z-order, or native-command escape hatch. See Decision 0086.
 */
export function isWindowFullscreenSetPayload(
  value: unknown,
): value is PayloadFor<"window.fullscreen.set"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    (value.mode === "fullscreen" || value.mode === "windowed")
  );
}

/**
 * Returns whether a value is exactly one bounded logical client-size request.
 *
 * Extra fields are a mismatch rather than a future route to a target,
 * position, monitor, DPI, geometry readback, or native rectangle. Fractions
 * are invalid because logical client pixels are whole 96-DPI units.
 */
export function isWindowSizeSetPayload(
  value: unknown,
): value is PayloadFor<"window.size.set"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.width === "number" &&
    Number.isSafeInteger(value.width) &&
    value.width >= MIN_WINDOW_CLIENT_WIDTH &&
    value.width <= MAX_WINDOW_CLIENT_WIDTH &&
    typeof value.height === "number" &&
    Number.isSafeInteger(value.height) &&
    value.height >= MIN_WINDOW_CLIENT_HEIGHT &&
    value.height <= MAX_WINDOW_CLIENT_HEIGHT
  );
}

/** Returns whether a value is a bounded, single-line window-title proposal. */
export function isWindowTitleProposal(value: unknown): value is string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > MAX_WINDOW_TITLE_UTF16_UNITS
  ) {
    return false;
  }
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) {
      return false;
    }
  }
  return true;
}

/** Returns whether a value is one exact session-local logical view identity. */
export function isCanonicalSessionWindowId(value: unknown): value is SessionWindowId {
  return value === "main" || isCanonicalSecondaryWindowId(value);
}

/** Returns whether a value is one exact session-local secondary identity. */
export function isCanonicalSecondaryWindowId(value: unknown): value is SecondarySessionWindowId {
  if (typeof value !== "string" || !/^window-[1-9][0-9]{0,4}$/.test(value)) {
    return false;
  }
  const suffix = Number(value.slice("window-".length));
  return Number.isSafeInteger(suffix) && suffix <= 65_535;
}
