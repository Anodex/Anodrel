/**
 * Versioned, transport-neutral messages shared by Anodrel clients and hosts.
 * Values crossing the boundary must be JSON-compatible.
 */

export const PROTOCOL_VERSION = { major: 1, minor: 13 } as const;
export const MAX_REQUEST_ID_BYTES = 256;
export const MAX_OPERATION_BYTES = 128;
export const MAX_CANCELLATION_ID_BYTES = 256;
export const MAX_UI_DOCUMENT_REQUEST_BYTES = 24 * 1024;
export const MAX_CLIPBOARD_TEXT_REQUEST_BYTES = 24 * 1024;
export const MAX_EXTERNAL_LINK_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_FILTERS = 8;
export const MAX_FILE_TEXT_RESPONSE_BYTES = 8 * 1024;
export const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES = 24 * 1024;
export const SELECTION_REFERENCE_BYTES = 22;
/** Maximum UTF-8 bytes in an exact credential name (ASCII only). */
export const MAX_CREDENTIAL_NAME_BYTES = 64;
/** Maximum characters in the canonical hexadecimal representation of a secret. */
export const MAX_CREDENTIAL_SECRET_HEX_BYTES = 4_096;

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
  | "dialog.open_file"
  | "dialog.save_file"
  | "file.read_text"
  | "storage.state.read"
  | "storage.state.replace"
  | "storage.state.clear"
  | "credential.read"
  | "credential.write"
  | "credential.delete"
  | "notification.show";

export type EmptyPayload = Record<string, never>;

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
  "ui.document.replace": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.document.replace.v2": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.events.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly events: readonly UiActionInvokedEvent[];
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
  | "dialog.unavailable"
  | "file.unavailable"
  | "file.text_invalid"
  | "file.text_too_large"
  | "storage.unavailable"
  | "storage.snapshot_invalid"
  | "storage.snapshot_too_large"
  | "diagnostics.unavailable"
  | "credential.unavailable"
  | "credential.access_denied"
  | "credential.stored_secret_invalid"
  | "notification.unavailable"
  | "notification.busy"
  | "notification.text_invalid";

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
