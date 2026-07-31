import {
  PROTOCOL_VERSION,
  isCancellationEnvelope,
  isEmptyPayload,
  isPingPayload,
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

export interface MockHostOptions {
  readonly applicationId: string;
  readonly grantedCapabilities?: readonly Capability[];
  readonly hostName?: string;
  readonly now?: () => Date;
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
 */
export class MockHost {
  private readonly applicationId: string;
  private readonly grantedCapabilities: readonly Capability[];
  private readonly hostName: string;
  private readonly now: () => Date;
  private sessionCount = 0;

  constructor(options: MockHostOptions) {
    if (options.applicationId.trim().length === 0) {
      throw new Error("MockHost requires a non-empty applicationId.");
    }

    this.applicationId = options.applicationId;
    this.grantedCapabilities = [...(options.grantedCapabilities ?? [])];
    this.hostName = options.hostName ?? "anodrel-mock-host";
    this.now = options.now ?? (() => new Date());
  }

  createTransport(sessionId = `mock-session-${++this.sessionCount}`): MockHostTransport {
    const cancelled = new Set<string>();

    return {
      send: async <TOperation extends PlatformOperation>(
        request: RequestEnvelope<TOperation>,
      ) =>
        this.handle(request, sessionId, cancelled) as Promise<ResponseEnvelope<TOperation>>,
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

    return this.dispatch(request, sessionId);
  }

  private dispatch(request: WireRequestEnvelope, sessionId: string): ResponseEnvelope {
    switch (request.operation) {
      case "platform.ping":
        if (!isPingPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "platform.ping requires a sentAt string.",
          );
        }
        return this.success("platform.ping", request.requestId, {
          receivedAt: this.now().toISOString(),
          hostName: this.hostName,
        });

      case "platform.capabilities":
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "platform.capabilities does not accept a payload.",
          );
        }
        return this.success("platform.capabilities", request.requestId, {
          applicationId: this.applicationId,
          grantedCapabilities: [...this.grantedCapabilities],
        });

      case "platform.health":
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "platform.health does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "diagnostics.read")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "platform.health requires the diagnostics.read capability.",
            { capability: "diagnostics.read" },
          );
        }
        return this.success("platform.health", request.requestId, {
          status: "ready",
          hostName: this.hostName,
          protocolVersion: PROTOCOL_VERSION,
        });

      default:
        return this.failure(
          request.requestId,
          "operation.unsupported",
          `Operation ${request.operation} is not supported by this host.`,
        );
    }
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

function extractRequestId(value: unknown): string {
  if (
    typeof value === "object" &&
    value !== null &&
    "requestId" in value &&
    typeof value.requestId === "string" &&
    value.requestId.length > 0
  ) {
    return value.requestId;
  }

  return "invalid-request";
}
