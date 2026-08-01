/**
 * Versioned, transport-neutral messages shared by Anodrel clients and hosts.
 * Values crossing the boundary must be JSON-compatible.
 */

export const PROTOCOL_VERSION = { major: 1, minor: 4 } as const;
export const MAX_REQUEST_ID_BYTES = 256;
export const MAX_OPERATION_BYTES = 128;
export const MAX_CANCELLATION_ID_BYTES = 256;
export const MAX_UI_DOCUMENT_REQUEST_BYTES = 24 * 1024;

export interface ProtocolVersion {
  readonly major: number;
  readonly minor: number;
}

/** Capabilities are granted by the host policy, never by rendered application content. */
export type Capability =
  | "diagnostics.read"
  | "ui.document.write"
  | "ui.events.read"
  | "session.close";

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
  | "request.payload_invalid";

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
