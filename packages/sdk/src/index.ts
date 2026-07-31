import {
  PROTOCOL_VERSION,
  createRequest,
  type CancellationEnvelope,
  type PayloadFor,
  type PlatformOperation,
  type ProtocolError,
  type RequestEnvelope,
  type ResponseEnvelope,
  type ResultFor,
} from "@anodrel/protocol";

export interface RequestIdFactory {
  next(): string;
}

export interface PlatformTransport {
  send<TOperation extends PlatformOperation>(
    request: RequestEnvelope<TOperation>,
  ): Promise<ResponseEnvelope<TOperation>>;
  cancel(cancellation: CancellationEnvelope): Promise<void>;
}

export class PlatformRemoteError extends Error {
  readonly code: ProtocolError["code"];
  readonly retryable: boolean;
  readonly details: ProtocolError["details"];

  constructor(error: ProtocolError) {
    super(error.message);
    this.name = "PlatformRemoteError";
    this.code = error.code;
    this.retryable = error.retryable;
    this.details = error.details;
  }
}

export class PlatformProtocolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PlatformProtocolError";
  }
}

export class PlatformClient {
  constructor(
    private readonly transport: PlatformTransport,
    private readonly requestIds: RequestIdFactory = new UuidRequestIdFactory(),
  ) {}

  ping(sentAt: string): Promise<ResultFor<"platform.ping">> {
    return this.request("platform.ping", { sentAt });
  }

  getCapabilities(): Promise<ResultFor<"platform.capabilities">> {
    return this.request("platform.capabilities", {});
  }

  getHealth(): Promise<ResultFor<"platform.health">> {
    return this.request("platform.health", {});
  }

  async cancel(cancellationId: string): Promise<void> {
    await this.transport.cancel({
      protocolVersion: PROTOCOL_VERSION,
      kind: "cancel",
      cancellationId,
    });
  }

  private async request<TOperation extends PlatformOperation>(
    operation: TOperation,
    payload: PayloadFor<TOperation>,
    cancellationId?: string,
  ): Promise<ResultFor<TOperation>> {
    const request = createRequest(
      this.requestIds.next(),
      operation,
      payload,
      cancellationId,
    );
    const response = await this.transport.send(request);

    if (response.requestId !== request.requestId) {
      throw new PlatformProtocolError(
        `Host response ID ${response.requestId} did not match request ID ${request.requestId}.`,
      );
    }

    if (response.status === "failure") {
      throw new PlatformRemoteError(response.error);
    }

    return response.result;
  }
}

class UuidRequestIdFactory implements RequestIdFactory {
  next(): string {
    const generator = globalThis.crypto?.randomUUID;
    if (generator === undefined) {
      throw new PlatformProtocolError(
        "This runtime cannot generate a cryptographically strong request ID.",
      );
    }

    return generator.call(globalThis.crypto);
  }
}
