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

  replaceUiDocument(document: string): Promise<ResultFor<"ui.document.replace">> {
    return this.request("ui.document.replace", { document });
  }

  replaceUiDocumentV2(document: string): Promise<ResultFor<"ui.document.replace.v2">> {
    return this.request("ui.document.replace.v2", { document });
  }

  readUiEvents(): Promise<ResultFor<"ui.events.read">> {
    return this.request("ui.events.read", {});
  }

  closeSession(): Promise<ResultFor<"session.close">> {
    return this.request("session.close", {});
  }

  readClipboardText(): Promise<ResultFor<"clipboard.read">> {
    return this.request("clipboard.read", {});
  }

  writeClipboardText(text: string): Promise<ResultFor<"clipboard.write">> {
    return this.request("clipboard.write", { text });
  }

  openExternalLink(url: string): Promise<ResultFor<"external.open">> {
    return this.request("external.open", { url });
  }

  openFileDialog(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.open_file">> {
    return this.request("dialog.open_file", { filters });
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
