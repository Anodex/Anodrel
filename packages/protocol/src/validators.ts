//! Runtime validation for every bounded protocol payload.
//!
//! These guards preserve the versioned protocol boundary: callers receive only
//! values within the documented limits before they cross into the native host.

import {
  MAX_CANCELLATION_ID_BYTES,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_CREDENTIAL_NAME_BYTES,
  MAX_CREDENTIAL_SECRET_HEX_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MAX_FILE_BINARY_WRITE_BYTES,
  MAX_FILE_DIALOG_FILTERS,
  MAX_FILE_DIALOG_REQUEST_BYTES,
  MAX_FILE_TEXT_RESPONSE_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_MENU_ACTION_ID_BYTES,
  MAX_MENU_ITEM_LABEL_BYTES,
  MAX_MENU_ITEMS,
  MAX_MENU_LABEL_BYTES,
  MAX_MENUS,
  MAX_MENU_REPLACE_REQUEST_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MAX_OPERATION_BYTES,
  MAX_REQUEST_ID_BYTES,
  SAVE_REFERENCE_BYTES,
  SELECTION_REFERENCE_BYTES,
  MAX_STORAGE_SNAPSHOT_REQUEST_BYTES,
  MAX_UI_DOCUMENT_REQUEST_BYTES,
  MAX_WINDOW_CLIENT_HEIGHT,
  MAX_WINDOW_CLIENT_WIDTH,
  MIN_WINDOW_CLIENT_HEIGHT,
  MIN_WINDOW_CLIENT_WIDTH,
  PROTOCOL_VERSION,
} from "./index.js";
import { canonicalBase64UrlDecodedLength } from "./base64url.js";
import { isLimitedIdentifier, isProtocolVersion, isRecord, isValidatedHttpsTextFetchUrl, isValidatedHttpsUrl } from "./validators/primitives.js";
import type {
  CancellationEnvelope,
  EmptyPayload,
  NativeMenuShortcut,
  PayloadFor,
  ProtocolVersion,
  SecondarySessionWindowId,
  SessionWindowId,
  WireRequestEnvelope,
  EventEnvelope,
  UiActionInvokedEvent,
} from "./index.js";
export function isWireRequestEnvelope(value: unknown): value is WireRequestEnvelope {
  if (!isRecord(value) || value.kind !== "request") {
    return false;
  }

  return (
    isProtocolVersion(value.protocolVersion) &&
    typeof value.requestId === "string" &&
    isLimitedIdentifier(value.requestId, MAX_REQUEST_ID_BYTES) &&
    typeof value.operation === "string" &&
    isLimitedIdentifier(value.operation, MAX_OPERATION_BYTES) &&
    (value.cancellationId === undefined ||
      (typeof value.cancellationId === "string" &&
        isLimitedIdentifier(value.cancellationId, MAX_CANCELLATION_ID_BYTES)))
  );
}

export function isCancellationEnvelope(value: unknown): value is CancellationEnvelope {
  return (
    isRecord(value) &&
    value.kind === "cancel" &&
    isProtocolVersion(value.protocolVersion) &&
    typeof value.cancellationId === "string" &&
    isLimitedIdentifier(value.cancellationId, MAX_CANCELLATION_ID_BYTES)
  );
}

export function isPingPayload(value: unknown): value is PayloadFor<"platform.ping"> {
  return isRecord(value) && typeof value.sentAt === "string";
}

export function isEmptyPayload(value: unknown): value is EmptyPayload {
  return isRecord(value) && Object.keys(value).length === 0;
}

/** Validates the bounded outer payload for an authenticated UI document update. */
export function isUiDocumentReplacePayload(
  value: unknown,
): value is PayloadFor<"ui.document.replace"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.document === "string" &&
    new TextEncoder().encode(value.document).byteLength <= MAX_UI_DOCUMENT_REQUEST_BYTES
  );
}

/** Validates the exact bounded text payload for a clipboard write. */
export function isClipboardWritePayload(
  value: unknown,
): value is PayloadFor<"clipboard.write"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.text === "string" &&
    new TextEncoder().encode(value.text).byteLength <= MAX_CLIPBOARD_TEXT_REQUEST_BYTES
  );
}

/** Validates the exact bounded URL payload for an external HTTPS handoff. */
export function isExternalOpenPayload(
  value: unknown,
): value is PayloadFor<"external.open"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.url === "string" &&
    new TextEncoder().encode(value.url).byteLength <= MAX_EXTERNAL_LINK_REQUEST_BYTES &&
    isValidatedHttpsUrl(value.url)
  );
}

/** Validates strict structured filters for one host-owned file picker. */
export function isFileDialogOpenPayload(
  value: unknown,
): value is PayloadFor<"dialog.open_file"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    Array.isArray(value.filters) &&
    value.filters.length > 0 &&
    value.filters.length <= MAX_FILE_DIALOG_FILTERS &&
    new TextEncoder().encode(JSON.stringify(value)).byteLength <= MAX_FILE_DIALOG_REQUEST_BYTES &&
    value.filters.every(isFileDialogFilter)
  );
}

/** Validates one exact opaque selection reference for a bounded file text read. */
export function isFileTextReadPayload(
  value: unknown,
): value is PayloadFor<"file.read_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.selectionReference === "string" &&
    value.selectionReference.length === SELECTION_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.selectionReference)
  );
}

/** Validates an exact protocol-safe whole application-state replacement. */
export function isStorageStateReplacePayload(
  value: unknown,
): value is PayloadFor<"storage.state.replace"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.snapshot === "string" &&
    new TextEncoder().encode(value.snapshot).byteLength <= MAX_STORAGE_SNAPSHOT_REQUEST_BYTES
  );
}

/** Validates one exact, non-enumerable credential name. */
export function isCredentialReadPayload(
  value: unknown,
): value is PayloadFor<"credential.read"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    isCredentialName(value.name)
  );
}

/** Validates one credential name and its canonical bounded secret encoding. */
export function isCredentialWritePayload(
  value: unknown,
): value is PayloadFor<"credential.write"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isCredentialName(value.name) &&
    isCanonicalCredentialSecret(value.secret)
  );
}

/** Maximum UTF-16 code units in a notification title. */
export const MAX_NOTIFICATION_TITLE_UTF16_UNITS = 63;

/** Maximum UTF-16 code units in a notification body. */
export const MAX_NOTIFICATION_BODY_UTF16_UNITS = 255;

/**
 * Returns whether a value is exactly one notification title and body.
 *
 * An extra field is a mismatch rather than something to ignore, so a future
 * urgency, icon, or action field cannot be smuggled past protocol 1.13.
 */
export function isNotificationShowPayload(
  value: unknown,
): value is PayloadFor<"notification.show"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isNotificationText(value.title, MAX_NOTIFICATION_TITLE_UTF16_UNITS, false) &&
    isNotificationText(value.body, MAX_NOTIFICATION_BODY_UTF16_UNITS, true)
  );
}

/**
 * Returns whether a value is bounded notification text.
 *
 * Length is measured in UTF-16 code units because that is what the host's
 * native buffers count. Control characters are rejected so text cannot forge a
 * second message or misrepresent its source; a body may carry line feeds
 * because the target surface renders them as breaks.
 */
function isNotificationText(value: unknown, maximumUnits: number, allowLineFeed: boolean): boolean {
  if (typeof value !== "string" || value.length === 0 || value.length > maximumUnits) {
    return false;
  }
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (allowLineFeed && code === 0x0a) {
      continue;
    }
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) {
      return false;
    }
  }
  return true;
}

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

/** Validates the exact bounded HTTPS text-fetch payload for Protocol 1.19. */
export function isNetworkFetchTextPayload(
  value: unknown,
): value is PayloadFor<"network.fetch_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.url === "string" &&
    new TextEncoder().encode(value.url).byteLength <= MAX_NETWORK_FETCH_REQUEST_BYTES &&
    isValidatedHttpsTextFetchUrl(value.url)
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

/** Validates one exact opaque save reference and bounded UTF-8 output text. */
export function isFileTextWritePayload(
  value: unknown,
): value is PayloadFor<"file.write_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.saveReference === "string" &&
    value.saveReference.length === SAVE_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.saveReference) &&
    typeof value.text === "string" &&
    new TextEncoder().encode(value.text).byteLength <= MAX_FILE_TEXT_WRITE_BYTES
  );
}

/** The bounded validation outcome for one binary-output protocol payload. */
export type FileBinaryWritePayloadStatus = "valid" | "invalid" | "too_large";

/** Validates the exact non-decoding shape of a binary-output request. */
export function isFileBinaryWritePayloadShape(
  value: unknown,
): value is { readonly saveReference: string; readonly bytesBase64Url: string } {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.saveReference === "string" &&
    value.saveReference.length === SAVE_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.saveReference) &&
    typeof value.bytesBase64Url === "string"
  );
}

/**
 * Classifies one exact binary-output payload without decoding it.
 *
 * This shares the Rust core's strict canonical base64url grammar so the mock
 * can preserve the same invalid-versus-too-large protocol outcome.
 */
export function classifyFileBinaryWritePayload(value: unknown): FileBinaryWritePayloadStatus {
  if (!isFileBinaryWritePayloadShape(value)) {
    return "invalid";
  }
  const decodedLength = canonicalBase64UrlDecodedLength(value.bytesBase64Url);
  if (decodedLength === undefined) {
    return "invalid";
  }
  return decodedLength > MAX_FILE_BINARY_WRITE_BYTES ? "too_large" : "valid";
}

/** Validates one exact canonical bounded binary-output payload. */
export function isFileBinaryWritePayload(
  value: unknown,
): value is PayloadFor<"file.write_binary"> {
  return classifyFileBinaryWritePayload(value) === "valid";
}

/** Validates one exact bounded native-session menu replacement. */
export function isMenuReplacePayload(
  value: unknown,
  shortcutsAllowed = true,
): value is PayloadFor<"menu.replace"> {
  if (
    !isRecord(value) ||
    Object.keys(value).length !== 1 ||
    !Array.isArray(value.menus) ||
    value.menus.length === 0 ||
    value.menus.length > MAX_MENUS ||
    !hasAtMostEncodedJsonBytes(value, MAX_MENU_REPLACE_REQUEST_BYTES)
  ) {
    return false;
  }

  const actionIds = new Set<string>();
  const shortcuts = new Set<string>();
  return value.menus.every((menu) => isNativeSessionMenu(menu, actionIds, shortcuts, shortcutsAllowed));
}

function isNativeSessionMenu(
  value: unknown,
  actionIds: Set<string>,
  shortcuts: Set<string>,
  shortcutsAllowed: boolean,
): boolean {
  if (
    !isRecord(value) ||
    Object.keys(value).length !== 2 ||
    !isMenuText(value.label, MAX_MENU_LABEL_BYTES) ||
    !Array.isArray(value.items) ||
    value.items.length === 0 ||
    value.items.length > MAX_MENU_ITEMS
  ) {
    return false;
  }

  for (const item of value.items) {
    if (!isRecord(item)) {
      return false;
    }
    const hasShortcut = Object.prototype.hasOwnProperty.call(item, "shortcut");
    if (
      Object.keys(item).length !== 3 + Number(hasShortcut) ||
      !isMenuActionId(item.id) ||
      !isMenuText(item.label, MAX_MENU_ITEM_LABEL_BYTES) ||
      typeof item.enabled !== "boolean" ||
      actionIds.has(item.id)
    ) {
      return false;
    }
    if (hasShortcut) {
      const shortcut = item.shortcut;
      if (!shortcutsAllowed || !isMenuShortcut(shortcut) || shortcuts.has(shortcut)) {
        return false;
      }
      shortcuts.add(shortcut);
    }
    actionIds.add(item.id);
  }
  return true;
}

function isMenuShortcut(value: unknown): value is NativeMenuShortcut {
  return typeof value === "string" && /^Ctrl\+(?:Shift\+)?[A-Z0-9]$/.test(value);
}

function isMenuActionId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_MENU_ACTION_ID_BYTES &&
    /^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$/.test(value)
  );
}

function isMenuText(value: unknown, maximumBytes: number): value is string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    !isWellFormedUnicode(value) ||
    new TextEncoder().encode(value).byteLength > maximumBytes
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

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (index + 1 >= value.length) {
        return false;
      }
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        return false;
      }
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return false;
    }
  }
  return true;
}

function hasAtMostEncodedJsonBytes(value: unknown, maximumBytes: number): boolean {
  try {
    return new TextEncoder().encode(JSON.stringify(value)).byteLength <= maximumBytes;
  } catch {
    return false;
  }
}

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

/** Returns whether a value is the stable protocol credential-name grammar. */
export function isCredentialName(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_CREDENTIAL_NAME_BYTES &&
    /^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?$/.test(value)
  );
}

/** Returns whether a value is a non-empty, lowercase, even-length secret hex string. */
export function isCanonicalCredentialSecret(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_CREDENTIAL_SECRET_HEX_BYTES &&
    value.length % 2 === 0 &&
    /^[0-9a-f]+$/.test(value)
  );
}

function isFileDialogFilter(value: unknown): boolean {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.label === "string" &&
    value.label.length > 0 &&
    value.label.length <= 64 &&
    /^[\x20-\x7e]+$/.test(value.label) &&
    Array.isArray(value.extensions) &&
    value.extensions.length > 0 &&
    value.extensions.length <= 8 &&
    value.extensions.every(
      (extension) =>
        typeof extension === "string" && extension.length > 0 && extension.length <= 16 && /^[a-z0-9]+$/.test(extension),
    )
  );
}

