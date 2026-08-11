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

  readDiagnosticEntries(): Promise<ResultFor<"diagnostics.entries.read">> {
    return this.request("diagnostics.entries.read", {});
  }

  readCredential(name: string): Promise<ResultFor<"credential.read">> {
    return this.request("credential.read", { name });
  }

  writeCredential(name: string, secret: string): Promise<ResultFor<"credential.write">> {
    return this.request("credential.write", { name, secret });
  }

  deleteCredential(name: string): Promise<ResultFor<"credential.delete">> {
    return this.request("credential.delete", { name });
  }

  /**
   * Shows one bounded notification.
   *
   * A successful result means the host handed the values to the operating
   * system, not that anyone saw them. There is deliberately no way to learn
   * whether the user has notifications silenced or this application muted.
   */
  showNotification(title: string, body: string): Promise<ResultFor<"notification.show">> {
    return this.request("notification.show", { title, body });
  }

  /**
   * Proposes the title of this session's own window.
   *
   * A proposal, not an assignment. The host composes the displayed caption by
   * appending the application's validated display name, so `Report.pdf` becomes
   * `Report.pdf — Your Application`. That suffix cannot be suppressed or forged,
   * which is what stops a title claiming to be another application.
   *
   * There is no window argument, and none is coming: the host resolves the
   * window from the authenticated session.
   */
  setWindowTitle(title: string): Promise<ResultFor<"window.title.set">> {
    return this.request("window.title.set", { title });
  }

  replaceUiDocument(document: string): Promise<ResultFor<"ui.document.replace">> {
    return this.request("ui.document.replace", { document });
  }

  replaceUiDocumentV2(document: string): Promise<ResultFor<"ui.document.replace.v2">> {
    return this.request("ui.document.replace.v2", { document });
  }

  /**
   * Reads every field value on this session's own current surface.
   *
   * A snapshot taken when you ask, not a stream. There is no way to name a
   * field and no change event, so this cannot be used to follow what someone
   * is typing — call it when a person has finished, such as on a submit
   * action. See `docs/UI_FIELDS.md`.
   */
  readUiFields(): Promise<ResultFor<"ui.fields.read">> {
    return this.request("ui.fields.read", {});
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

  saveFileDialog(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.save_file">> {
    return this.request("dialog.save_file", { filters });
  }

  openFileDialogWithReference(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.open_file.v2">> {
    return this.request("dialog.open_file.v2", { filters });
  }

  readSelectedFileText(selectionReference: string): Promise<ResultFor<"file.read_text">> {
    return this.request("file.read_text", { selectionReference });
  }

  readStorageState(): Promise<ResultFor<"storage.state.read">> {
    return this.request("storage.state.read", {});
  }

  replaceStorageState(snapshot: string): Promise<ResultFor<"storage.state.replace">> {
    return this.request("storage.state.replace", { snapshot });
  }

  clearStorageState(): Promise<ResultFor<"storage.state.clear">> {
    return this.request("storage.state.clear", {});
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
