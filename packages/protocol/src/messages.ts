/** Versioned request, response, error, and semantic-event envelopes. */

import type {
  Capability,
  PayloadFor,
  PlatformOperation,
  ProtocolVersion,
  ResultFor,
} from "./index.js";

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
  | "folder.unavailable"
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
