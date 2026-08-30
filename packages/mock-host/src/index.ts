import {
  PROTOCOL_VERSION,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_STORAGE_SNAPSHOT_REQUEST_BYTES,
  isCancellationEnvelope,
  isCredentialWritePayload,
  isSupportedProtocolVersion,
  isWireRequestEnvelope,
  protocolVersionToString,
  type Capability,
  type CancellationEnvelope,
  type PlatformOperation,
  type ProtocolError,
  type ProtocolErrorCode,
  type RequestEnvelope,
  type ResponseDiagnostics,
  type ResponseEnvelope,
  type ResultFor,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import { dispatchMockOperation } from "./operations/dispatch.js";
import type { MockOperationContext } from "./operations/context.js";
import {
  createUiWindowState,
  extractRequestId,
  isWellFormedUnicode,
  type ContextMenuState,
  type MenuState,
  type UiWindowState,
} from "./state.js";export interface MockHostOptions {
  readonly applicationId: string;
  readonly grantedCapabilities?: readonly Capability[];
  readonly hostName?: string;
  readonly now?: () => Date;
  /** Initial bounded text returned by a granted clipboard read. */
  readonly clipboardText?: string;
  /** Initial bounded application state returned by a granted storage read. */
  readonly storageSnapshot?: string;
  /** Initial exact credentials, keyed by the documented credential-name grammar. */
  readonly credentials?: Readonly<Record<string, string>>;
  /** Fixed protocol-safe response for a granted HTTPS text-fetch test. */
  readonly networkTextResponse?: Readonly<{ statusCode: number; text: string }>;
}

/**
 * Structurally compatible with the SDK transport, without making the mock host
 * depend on the SDK module at runtime or build time.
 */
export interface MockHostTransport {
  send<TOperation extends PlatformOperation>(
    request: RequestEnvelope<TOperation>,
  ): Promise<ResponseEnvelope<TOperation>>;
  cancel(cancellation: CancellationEnvelope): Promise<void>;
}

/**
 * A deterministic host for application and contract tests. It deliberately
 * derives capability context from host configuration rather than request data.
 * It validates the bounded replacement payload but does not duplicate the
 * native strict UI document decoder; that decoder remains the authority at the
 * authenticated host boundary.
 */
export class MockHost {
  private readonly applicationId: string;
  private readonly grantedCapabilities: readonly Capability[];
  private readonly hostName: string;
  private readonly now: () => Date;
  private clipboardText: string | undefined;
  private storageSnapshot: string | undefined;
  private readonly credentials = new Map<string, string>();
  private readonly networkTextResponse: Readonly<{ statusCode: number; text: string }> | undefined;
  private sessionCount = 0;

  constructor(options: MockHostOptions) {
    if (options.applicationId.trim().length === 0) {
      throw new Error("MockHost requires a non-empty applicationId.");
    }
    if (
      options.clipboardText !== undefined &&
      new TextEncoder().encode(options.clipboardText).byteLength > MAX_CLIPBOARD_TEXT_REQUEST_BYTES
    ) {
      throw new Error("MockHost clipboard text exceeds the protocol size limit.");
    }
    if (
      options.storageSnapshot !== undefined &&
      new TextEncoder().encode(options.storageSnapshot).byteLength > MAX_STORAGE_SNAPSHOT_REQUEST_BYTES
    ) {
      throw new Error("MockHost storage snapshot exceeds the protocol size limit.");
    }
    for (const [name, secret] of Object.entries(options.credentials ?? {})) {
      if (!isCredentialWritePayload({ name, secret })) {
        throw new Error("MockHost credentials must use a valid name and canonical secret encoding.");
      }
      this.credentials.set(name, secret);
    }
    if (
      options.networkTextResponse !== undefined &&
      (!Number.isInteger(options.networkTextResponse.statusCode) ||
        options.networkTextResponse.statusCode < 100 ||
        options.networkTextResponse.statusCode > 599 ||
        !isWellFormedUnicode(options.networkTextResponse.text) ||
        new TextEncoder().encode(options.networkTextResponse.text).byteLength > 32 * 1024)
    ) {
      throw new Error("MockHost network text response must be protocol-representable.");
    }

    this.applicationId = options.applicationId;
    this.grantedCapabilities = [...(options.grantedCapabilities ?? [])];
    this.hostName = options.hostName ?? "anodrel-mock-host";
    this.now = options.now ?? (() => new Date());
    this.clipboardText = options.clipboardText;
    this.storageSnapshot = options.storageSnapshot;
    this.networkTextResponse = options.networkTextResponse;
  }

  createTransport(sessionId = `mock-session-${++this.sessionCount}`): MockHostTransport {
    const cancelled = new Set<string>();
    const sessionWindows = createUiWindowState();
    const menu = { revision: 0 };
    const contextMenu = { revision: 0 };

    return {
      send: async <TOperation extends PlatformOperation>(
        request: RequestEnvelope<TOperation>,
      ) =>
        this.handle(request, sessionId, cancelled, sessionWindows, menu, contextMenu) as Promise<ResponseEnvelope<TOperation>>,
      cancel: async (cancellation: CancellationEnvelope) => {
        if (!isCancellationEnvelope(cancellation)) {
          throw new Error("MockHost received an invalid cancellation envelope.");
        }
        if (!isSupportedProtocolVersion(cancellation.protocolVersion)) {
          throw new Error(
            `MockHost does not support protocol ${protocolVersionToString(cancellation.protocolVersion)}.`,
          );
        }
        cancelled.add(cancellation.cancellationId);
      },
    };
  }

  async handle(
    request: unknown,
    sessionId = `direct-session-${++this.sessionCount}`,
    cancelled: ReadonlySet<string> = new Set(),
    sessionWindows: UiWindowState = createUiWindowState(),
    menu: MenuState = { revision: 0 },
    contextMenu: ContextMenuState = { revision: 0 },
  ): Promise<ResponseEnvelope> {
    const requestId = extractRequestId(request);

    if (!isWireRequestEnvelope(request)) {
      return this.failure(requestId, "request.invalid", "Request envelope is malformed.");
    }

    if (!isSupportedProtocolVersion(request.protocolVersion)) {
      return this.failure(
        request.requestId,
        "protocol.version_unsupported",
        `Protocol ${protocolVersionToString(request.protocolVersion)} is not supported.`,
      );
    }

    if (request.cancellationId !== undefined && cancelled.has(request.cancellationId)) {
      return this.failure(
        request.requestId,
        "request.cancelled",
        "Request was cancelled before the host began processing it.",
      );
    }

    return this.dispatch(request, sessionId, sessionWindows, menu, contextMenu);
  }

  private dispatch(
    request: WireRequestEnvelope,
    sessionId: string,
    sessionWindows: UiWindowState,
    menu: MenuState,
    contextMenu: ContextMenuState,
  ): ResponseEnvelope {
    const context: MockOperationContext = {
      applicationId: this.applicationId,
      grantedCapabilities: this.grantedCapabilities,
      hostName: this.hostName,
      now: this.now,
      clipboardText: this.clipboardText,
      storageSnapshot: this.storageSnapshot,
      credentials: this.credentials,
      networkTextResponse: this.networkTextResponse,
      hasCapability: (operationSessionId, capability) => this.hasCapability(operationSessionId, capability),
      success: <TOperation extends PlatformOperation>(
        operation: TOperation,
        requestId: string,
        result: ResultFor<TOperation>,
      ): ResponseEnvelope<TOperation> => this.success(operation, requestId, result),
      failure: (requestId, code, message, details) => this.failure(requestId, code, message, details),
    };
    const response = dispatchMockOperation(
      context,
      request,
      sessionId,
      sessionWindows,
      menu,
      contextMenu,
    );
    this.clipboardText = context.clipboardText;
    this.storageSnapshot = context.storageSnapshot;
    return response;
  }
  private hasCapability(_sessionId: string, capability: Capability): boolean {
    // The mock receives session identity through its transport closure. A native
    // host must bind the same identity to an authenticated application session.
    return this.grantedCapabilities.includes(capability);
  }

  private success<TOperation extends PlatformOperation>(
    _operation: TOperation,
    requestId: string,
    result: ResultFor<TOperation>,
  ): ResponseEnvelope<TOperation> {
    return {
      protocolVersion: PROTOCOL_VERSION,
      kind: "response",
      requestId,
      status: "success",
      result,
      diagnostics: this.diagnostics(),
    };
  }

  private failure(
    requestId: string,
    code: ProtocolErrorCode,
    message: string,
    details?: ProtocolError["details"],
  ): ResponseEnvelope {
    return {
      protocolVersion: PROTOCOL_VERSION,
      kind: "response",
      requestId,
      status: "failure",
      error: { code, message, retryable: false, ...(details === undefined ? {} : { details }) },
      diagnostics: this.diagnostics(),
    };
  }

  private diagnostics(): ResponseDiagnostics {
    return { hostName: this.hostName };
  }
}
