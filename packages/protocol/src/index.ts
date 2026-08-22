/**
 * Versioned, transport-neutral messages shared by Anodrel clients and hosts.
 * Values crossing the boundary must be JSON-compatible.
 */

import { canonicalBase64UrlDecodedLength } from "./base64url.js";

export { encodeCanonicalBase64Url } from "./base64url.js";

export const PROTOCOL_VERSION = { major: 1, minor: 26 } as const;
export const MAX_REQUEST_ID_BYTES = 256;
export const MAX_OPERATION_BYTES = 128;
export const MAX_CANCELLATION_ID_BYTES = 256;
export const MAX_UI_DOCUMENT_REQUEST_BYTES = 24 * 1024;
export const MAX_CLIPBOARD_TEXT_REQUEST_BYTES = 24 * 1024;
export const MAX_EXTERNAL_LINK_REQUEST_BYTES = 2 * 1024;
/** Maximum UTF-8 bytes in the exact HTTPS text-fetch URL payload. */
export const MAX_NETWORK_FETCH_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_FILTERS = 8;
export const MAX_FILE_TEXT_RESPONSE_BYTES = 8 * 1024;
export const MAX_FILE_TEXT_WRITE_BYTES = 8 * 1024;
/** Maximum decoded bytes in one exact binary-output replacement. */
export const MAX_FILE_BINARY_WRITE_BYTES = 32 * 1024;
/** Maximum encoded JSON bytes in one complete native-menu replacement payload. */
export const MAX_MENU_REPLACE_REQUEST_BYTES = 16 * 1024;
export const MAX_MENUS = 8;
export const MAX_MENU_ITEMS = 16;
export const MAX_MENU_LABEL_BYTES = 32;
export const MAX_MENU_ITEM_LABEL_BYTES = 96;
export const MAX_MENU_ACTION_ID_BYTES = 64;
export const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES = 24 * 1024;
export const SELECTION_REFERENCE_BYTES = 22;
/** Exact characters in a host-created save reference. */
export const SAVE_REFERENCE_BYTES = 22;
/** Maximum UTF-8 bytes in an exact credential name (ASCII only). */
export const MAX_CREDENTIAL_NAME_BYTES = 64;
/** Maximum characters in the canonical hexadecimal representation of a secret. */
export const MAX_CREDENTIAL_SECRET_HEX_BYTES = 4_096;
/** Smallest logical client width accepted by `window.size.set`. */
export const MIN_WINDOW_CLIENT_WIDTH = 320;
/** Largest logical client width accepted by `window.size.set`. */
export const MAX_WINDOW_CLIENT_WIDTH = 3_840;
/** Smallest logical client height accepted by `window.size.set`. */
export const MIN_WINDOW_CLIENT_HEIGHT = 240;
/** Largest logical client height accepted by `window.size.set`. */
export const MAX_WINDOW_CLIENT_HEIGHT = 2_160;

export interface ProtocolVersion {
  readonly major: number;
  readonly minor: number;
}

/** Capabilities are granted by the host policy, never by rendered application content. */
export type Capability =
  | "diagnostics.read"
  | "ui.document.write"
  | "ui.events.read"
  | "session.close"
  | "clipboard.read"
  | "clipboard.write"
  | "external.open"
  | "network.fetch"
  | "dialog.open_file"
  | "dialog.save_file"
  | "file.read_text"
  | "file.write_text"
  | "file.write_binary"
  | "storage.state.read"
  | "storage.state.replace"
  | "storage.state.clear"
  | "credential.read"
  | "credential.write"
  | "credential.delete"
  | "notification.show"
  | "window.title"
  | "ui.fields.read"
  | "window.state"
  | "window.focus"
  | "window.fullscreen"
  | "window.size"
  | "window.open"
  | "window.close"
  | "menu.write";

export type EmptyPayload = Record<string, never>;

/** The complete set of presentation states an application may request. */
export type WindowState = "minimized" | "maximized" | "restored";

/** The only reversible fullscreen modes an application may request. */
export type WindowFullscreenMode = "fullscreen" | "windowed";

/**
 * An opaque identity for one view in the current authenticated UI session.
 *
 * `main` names the session's primary view. The host issues secondary values as
 * canonical `window-<n>` strings and never treats either form as a native
 * handle, a global name, or a cross-session lookup key.
 */
export type SessionWindowId = "main" | SecondarySessionWindowId;

/** An opaque secondary view identity returned only by `window.open`. */
export type SecondarySessionWindowId = `window-${number}`;

/** One ASCII key permitted in a canonical local native-menu shortcut. */
export type NativeMenuShortcutKey =
  | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L"
  | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X"
  | "Y" | "Z" | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";

/** A canonical local shortcut for one semantic native-menu command. */
export type NativeMenuShortcut =
  | `Ctrl+${NativeMenuShortcutKey}`
  | `Ctrl+Shift+${NativeMenuShortcutKey}`;

/** One enabled or disabled semantic command in a native session menu. */
export interface NativeMenuItem {
  readonly id: string;
  readonly label: string;
  readonly enabled: boolean;
  /** Optional Protocol 1.24 local semantic shortcut. */
  readonly shortcut?: NativeMenuShortcut;
}

/** One top-level native session menu with its complete ordered item set. */
export interface NativeSessionMenu {
  readonly label: string;
  readonly items: readonly NativeMenuItem[];
}

export interface PlatformOperationMap {
  "platform.ping": {
    readonly payload: { readonly sentAt: string };
    readonly result: { readonly receivedAt: string; readonly hostName: string };
  };
  "platform.capabilities": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly applicationId: string;
      readonly grantedCapabilities: readonly Capability[];
    };
  };
  "platform.health": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly status: "ready";
      readonly hostName: string;
      readonly protocolVersion: ProtocolVersion;
    };
  };
  "diagnostics.entries.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly entries: readonly {
        readonly sequence: string;
        readonly level: "info";
        readonly component: string;
        readonly event: string;
      }[];
    };
  };
  "credential.read": {
    readonly payload: { readonly name: string };
    readonly result:
      | { readonly status: "found"; readonly secret: string }
      | { readonly status: "not_found" };
  };
  "credential.write": {
    readonly payload: { readonly name: string; readonly secret: string };
    readonly result: { readonly status: "written" };
  };
  "credential.delete": {
    readonly payload: { readonly name: string };
    readonly result: { readonly status: "deleted" } | { readonly status: "not_found" };
  };
  /**
   * Shows one bounded notification.
   *
   * The result reports only that the host handed the values over. It never
   * describes what the user experienced: whether notifications are silenced,
   * a focus mode is active, or this application is muted is not observable.
   */
  "notification.show": {
    readonly payload: { readonly title: string; readonly body: string };
    readonly result: { readonly status: "shown" };
  };
  /**
   * Proposes the title of this session's own window.
   *
   * A proposal, not an assignment. The host validates it and composes the
   * displayed caption with an application-name suffix that the proposal cannot
   * suppress or forge, so a title can say what is being shown and never change
   * what the application is.
   *
   * There is no window target: the host resolves the window from the
   * authenticated session. The result reports acceptance only — the composed
   * caption is deliberately not returned.
   */
  "window.title.set": {
    readonly payload: { readonly title: string };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Requests one standard presentation state for this session's own window.
   *
   * The host resolves the window from the authenticated session. There is no
   * target, native handle, geometry, state readback, or change event; success
   * means only that the host UI thread accepted the closed action.
   */
  "window.state.set": {
    readonly payload: { readonly state: WindowState };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Asks Windows to foreground this session's one host-owned window.
   *
   * It has no target, native handle, retry policy, focus readback, or event.
   * Windows decides whether the request is accepted; success reports only that
   * Windows accepted it, not what a person saw or used afterward.
   */
  "window.focus.request": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "requested" };
  };
  /**
   * Chooses reversible borderless fullscreen for this session's own window.
   *
   * The host retains native style and placement facts privately. There is no
   * target, monitor, geometry, display-mode control, state readback, or event;
   * success means only that the host UI thread accepted the closed action.
   */
  "window.fullscreen.set": {
    readonly payload: { readonly mode: WindowFullscreenMode };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Requests a bounded logical client size for this session's own window.
   *
   * The host derives the native framed rectangle at its current DPI. There is
   * no target, position, monitor, DPI, bounds readback, or geometry event;
   * success means only that the host UI thread accepted the request.
   */
  "window.size.set": {
    readonly payload: { readonly width: number; readonly height: number };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Opens one independently revised secondary view in this session.
   *
   * The host chooses all native presentation details. The returned ID is
   * opaque and session-scoped; it cannot be converted to a handle, a desktop
   * position, or a way to enumerate other views.
   */
  "window.open": {
    readonly payload: { readonly title: string; readonly document: string };
    readonly result: { readonly windowId: SecondarySessionWindowId };
  };
  /** Opens one exact v3 document in a bounded secondary session view. */
  "window.open.v3": {
    readonly payload: { readonly title: string; readonly document: string };
    readonly result: { readonly windowId: SecondarySessionWindowId };
  };
  /** Requests a close for one previously issued secondary view. */
  "window.close": {
    readonly payload: { readonly windowId: SecondarySessionWindowId };
    readonly result: { readonly status: "requested" };
  };
  /**
   * Replaces this authenticated session's complete native menu model.
   *
   * There is no native command number, accelerator, target, callback, or
   * handle. A successful revision is host-owned and opaque to the SDK.
   */
  "menu.replace": {
    readonly payload: { readonly menus: readonly NativeSessionMenu[] };
    readonly result: { readonly revision: string };
  };
  /**
   * Reads every field value on this session's own current surface.
   *
   * A snapshot, not a stream. The payload is empty and there is no selector:
   * a caller able to narrow a read to one field could repeat it until what
   * someone was typing had been reconstructed. Returning the whole surface
   * makes every read cost the same, so reading often gains nothing.
   *
   * The result carries values only — no caret, selection, timestamp, or
   * edited flag, because those describe the typing rather than the value.
   */
  "ui.fields.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly fields: ReadonlyArray<{ readonly id: string; readonly value: string }>;
    };
  };
  "ui.document.replace": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.document.replace.v2": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.document.replace.v3": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  /** Replaces the strict v1 document of one known session view. */
  "ui.document.replace.window": {
    readonly payload: { readonly windowId: SessionWindowId; readonly document: string };
    readonly result: { readonly revision: string };
  };
  /** Replaces one known session view with an exact v3 document. */
  "ui.document.replace.window.v3": {
    readonly payload: { readonly windowId: SessionWindowId; readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.events.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly events: readonly UiInteractionEvent[];
      readonly dropped: number;
      readonly discarded: number;
    };
  };
  /** Reads bounded semantic events from each current session view. */
  "ui.events.read.window": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly events: readonly WindowUiInteractionEvent[];
      readonly dropped: number;
      readonly discarded: number;
    };
  };
  "session.close": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "accepted" };
  };
  "clipboard.read": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "text"; readonly text: string }
      | { readonly status: "no_text" };
  };
  "clipboard.write": {
    readonly payload: { readonly text: string };
    readonly result: { readonly status: "written" };
  };
  "external.open": {
    readonly payload: { readonly url: string };
    readonly result: { readonly status: "opened" };
  };
  /**
   * Fetches one bounded UTF-8 response from a host-authorized HTTPS origin.
   *
   * There is deliberately no method, body, header, cookie, credential,
   * redirect, proxy, timeout, client-certificate, callback, or native-handle
   * field. A non-2xx status remains a successful protocol result when its
   * bounded text body is representable.
   */
  "network.fetch_text": {
    readonly payload: { readonly url: string };
    readonly result: { readonly statusCode: number; readonly text: string };
  };
  "dialog.open_file": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string }
      | { readonly status: "cancelled" };
  };
  "dialog.save_file": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "saved"; readonly path: string }
      | { readonly status: "cancelled" };
  };
  "dialog.open_file.v2": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string; readonly selectionReference: string }
      | { readonly status: "cancelled" };
  };
  "file.read_text": {
    readonly payload: { readonly selectionReference: string };
    readonly result: { readonly status: "text"; readonly text: string };
  };
  "dialog.save_file.v2": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string; readonly saveReference: string }
      | { readonly status: "cancelled" };
  };
  "file.write_text": {
    readonly payload: { readonly saveReference: string; readonly text: string };
    readonly result: { readonly status: "written" };
  };
  "file.write_binary": {
    readonly payload: { readonly saveReference: string; readonly bytesBase64Url: string };
    readonly result: { readonly status: "written" };
  };
  "storage.state.read": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "snapshot"; readonly snapshot: string }
      | { readonly status: "absent" };
  };
  "storage.state.replace": {
    readonly payload: { readonly snapshot: string };
    readonly result: { readonly status: "replaced" };
  };
  "storage.state.clear": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "cleared" };
  };
}

export type PlatformOperation = keyof PlatformOperationMap;
export type PayloadFor<TOperation extends PlatformOperation> =
  PlatformOperationMap[TOperation]["payload"];
export type ResultFor<TOperation extends PlatformOperation> =
  PlatformOperationMap[TOperation]["result"];

/**
 * The request constructed by the client SDK. The transport adapter binds it to
 * an authenticated application session before it reaches a host.
 */
export interface RequestEnvelope<TOperation extends PlatformOperation = PlatformOperation> {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "request";
  readonly requestId: string;
  readonly operation: TOperation;
  readonly payload: PayloadFor<TOperation>;
  readonly cancellationId?: string;
}

/**
 * The host-authenticated context attached by the transport. A client must not
 * be able to supply or elevate these values.
 */
export interface CapabilityContext {
  readonly applicationId: string;
  readonly sessionId: string;
  readonly grantedCapabilities: readonly Capability[];
}

export type HostRequestEnvelope<TOperation extends PlatformOperation = PlatformOperation> =
  RequestEnvelope<TOperation> & {
    readonly capabilityContext: CapabilityContext;
  };

/** The shape used at a raw host boundary before the operation is recognized. */
export interface WireRequestEnvelope {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "request";
  readonly requestId: string;
  readonly operation: string;
  readonly payload: unknown;
  readonly cancellationId?: string;
}

export interface CancellationEnvelope {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "cancel";
  readonly cancellationId: string;
}

export interface ResponseDiagnostics {
  /** A safe-to-expose host label; it must not contain paths, secrets, or raw errors. */
  readonly hostName: string;
}

export type ProtocolErrorCode =
  | "capability.denied"
  | "operation.unsupported"
  | "protocol.version_unsupported"
  | "request.cancelled"
  | "request.invalid"
  | "request.payload_invalid"
  | "clipboard.unavailable"
  | "clipboard.text_invalid"
  | "clipboard.text_too_large"
  | "external.unavailable"
  | "network.unavailable"
  | "network.response_invalid"
  | "dialog.unavailable"
  | "file.unavailable"
  | "file.text_invalid"
  | "file.text_too_large"
  | "file.binary_too_large"
  | "storage.unavailable"
  | "storage.snapshot_invalid"
  | "storage.snapshot_too_large"
  | "diagnostics.unavailable"
  | "credential.unavailable"
  | "credential.access_denied"
  | "credential.stored_secret_invalid"
  | "notification.unavailable"
  | "notification.busy"
  | "notification.text_invalid"
  | "window.unavailable"
  | "window.busy"
  | "window.title_invalid"
  | "ui.fields.unavailable"
  | "menu.unavailable";

export interface ProtocolError {
  readonly code: ProtocolErrorCode;
  readonly message: string;
  readonly retryable: boolean;
  readonly details?: Readonly<Record<string, string | number | boolean>>;
}

export interface SuccessResponseEnvelope<TOperation extends PlatformOperation = PlatformOperation> {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "response";
  readonly requestId: string;
  readonly status: "success";
  readonly result: ResultFor<TOperation>;
  readonly diagnostics: ResponseDiagnostics;
}

export interface FailureResponseEnvelope {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "response";
  readonly requestId: string;
  readonly status: "failure";
  readonly error: ProtocolError;
  readonly diagnostics: ResponseDiagnostics;
}

export type ResponseEnvelope<TOperation extends PlatformOperation = PlatformOperation> =
  | SuccessResponseEnvelope<TOperation>
  | FailureResponseEnvelope;

export interface EventEnvelope<TPayload = unknown> {
  readonly protocolVersion: ProtocolVersion;
  readonly kind: "event";
  readonly eventName: string;
  readonly source: string;
  readonly schemaVersion: ProtocolVersion;
  readonly payload: TPayload;
}

/** A current, enabled semantic UI action observed by a native host. */
export interface UiActionInvokedEvent
  extends EventEnvelope<{ readonly revision: string; readonly action: string }> {
  readonly eventName: "ui.action.invoked";
  readonly source: "native.ui";
  readonly schemaVersion: { readonly major: 1; readonly minor: 0 };
}

export function createRequest<TOperation extends PlatformOperation>(
  requestId: string,
  operation: TOperation,
  payload: PayloadFor<TOperation>,
  cancellationId?: string,
): RequestEnvelope<TOperation> {
  return {
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId,
    operation,
    payload,
    ...(cancellationId === undefined ? {} : { cancellationId }),
  };
}

export function isSupportedProtocolVersion(version: ProtocolVersion): boolean {
  return (
    version.major === PROTOCOL_VERSION.major &&
    version.minor <= PROTOCOL_VERSION.minor
  );
}

export function protocolVersionToString(version: ProtocolVersion): string {
  return `${version.major}.${version.minor}`;
}

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

function isValidatedHttpsUrl(value: string): boolean {
  if (value.length === 0 || !/^[\x21-\x7e]+$/.test(value) || value.includes("\\")) {
    return false;
  }
  const match = /^https:\/\/([^/?#]+)(?:[/?#].*)?$/.exec(value);
  if (match === null) {
    return false;
  }
  const authority = match[1];
  if (authority === undefined) {
    return false;
  }
  if (authority.includes("@")) {
    return false;
  }
  const separator = authority.lastIndexOf(":");
  const host = separator === -1 ? authority : authority.slice(0, separator);
  const port = separator === -1 ? undefined : authority.slice(separator + 1);
  if (
    !isDnsStyleHost(host) ||
    (port !== undefined && (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65_535))
  ) {
    return false;
  }
  return true;
}

function isValidatedHttpsTextFetchUrl(value: string): boolean {
  if (
    value.length === 0 ||
    !/^[A-Za-z0-9\-._~:/?@!$&'()*+,;=%]+$/.test(value) ||
    value.includes("\\") ||
    value.includes("#") ||
    !value.startsWith("https://") ||
    /%(?![0-9A-Fa-f]{2})/.test(value)
  ) {
    return false;
  }
  const match = /^https:\/\/([^/?]+)(?:[/?].*)?$/.exec(value);
  if (match === null) {
    return false;
  }
  const authority = match[1];
  if (authority === undefined || authority.includes("@")) {
    return false;
  }
  const separator = authority.lastIndexOf(":");
  const host = separator === -1 ? authority : authority.slice(0, separator);
  const port = separator === -1 ? undefined : authority.slice(separator + 1);
  return (
    isDnsStyleHost(host) &&
    !isIpv4Literal(host) &&
    (port === undefined || (/^\d+$/.test(port) && Number(port) >= 1 && Number(port) <= 65_535))
  );
}

function isIpv4Literal(value: string): boolean {
  const labels = value.split(".");
  return (
    labels.length === 4 &&
    labels.every((label) => /^\d+$/.test(label) && Number(label) >= 0 && Number(label) <= 255)
  );
}

function isDnsStyleHost(value: string): boolean {
  return (
    value.length > 0 &&
    value.length <= 253 &&
    !value.endsWith(".") &&
    value.split(".").every(
      (label) =>
        label.length > 0 &&
        label.length <= 63 &&
        /^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/.test(label),
    )
  );
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isProtocolVersion(value: unknown): value is ProtocolVersion {
  if (!isRecord(value)) {
    return false;
  }

  const { major, minor } = value;
  return (
    typeof major === "number" &&
    typeof minor === "number" &&
    Number.isSafeInteger(major) &&
    Number.isSafeInteger(minor) &&
    major >= 0 &&
    minor >= 0
  );
}

function isLimitedIdentifier(value: string, maximumBytes: number): boolean {
  return value.length > 0 && new TextEncoder().encode(value).byteLength <= maximumBytes;
}
