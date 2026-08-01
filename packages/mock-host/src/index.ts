import {
  PROTOCOL_VERSION,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  isCancellationEnvelope,
  isClipboardWritePayload,
  isExternalOpenPayload,
  isFileDialogOpenPayload,
  isFileTextReadPayload,
  isEmptyPayload,
  isUiDocumentReplacePayload,
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
  /** Initial bounded text returned by a granted clipboard read. */
  readonly clipboardText?: string;
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

    this.applicationId = options.applicationId;
    this.grantedCapabilities = [...(options.grantedCapabilities ?? [])];
    this.hostName = options.hostName ?? "anodrel-mock-host";
    this.now = options.now ?? (() => new Date());
    this.clipboardText = options.clipboardText;
  }

  createTransport(sessionId = `mock-session-${++this.sessionCount}`): MockHostTransport {
    const cancelled = new Set<string>();
    const uiDocument = { revision: 0 };

    return {
      send: async <TOperation extends PlatformOperation>(
        request: RequestEnvelope<TOperation>,
      ) =>
        this.handle(request, sessionId, cancelled, uiDocument) as Promise<ResponseEnvelope<TOperation>>,
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
    uiDocument: UiDocumentState = { revision: 0 },
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

    return this.dispatch(request, sessionId, uiDocument);
  }

  private dispatch(
    request: WireRequestEnvelope,
    sessionId: string,
    uiDocument: UiDocumentState,
  ): ResponseEnvelope {
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

      case "ui.document.replace":
      case "ui.document.replace.v2":
        if (request.protocolVersion.minor < (request.operation === "ui.document.replace.v2" ? 4 : 1)) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            `${request.operation} requires a supported protocol version.`,
          );
        }
        if (!isUiDocumentReplacePayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "ui.document.replace requires one bounded document string.",
          );
        }
        if (!this.hasCapability(sessionId, "ui.document.write")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "ui.document.replace requires the ui.document.write capability.",
            { capability: "ui.document.write" },
          );
        }
        uiDocument.revision += 1;
        return this.success(request.operation, request.requestId, {
          revision: uiDocument.revision.toString(),
        });

      case "ui.events.read":
        if (request.protocolVersion.minor < 2) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "ui.events.read requires protocol 1.2 or later.",
          );
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "ui.events.read does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "ui.events.read")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "ui.events.read requires the ui.events.read capability.",
            { capability: "ui.events.read" },
          );
        }
        return this.success("ui.events.read", request.requestId, {
          events: [],
          dropped: 0,
          discarded: 0,
        });

      case "session.close":
        if (request.protocolVersion.minor < 3) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "session.close requires protocol 1.3 or later.",
          );
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "session.close does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "session.close")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "session.close requires the session.close capability.",
            { capability: "session.close" },
          );
        }
        return this.success("session.close", request.requestId, { status: "accepted" });

      case "clipboard.read":
        if (request.protocolVersion.minor < 5) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "clipboard.read requires protocol 1.5 or later.",
          );
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "clipboard.read does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "clipboard.read")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "clipboard.read requires the clipboard.read capability.",
            { capability: "clipboard.read" },
          );
        }
        return this.success(
          "clipboard.read",
          request.requestId,
          this.clipboardText === undefined
            ? { status: "no_text" }
            : { status: "text", text: this.clipboardText },
        );

      case "clipboard.write":
        if (request.protocolVersion.minor < 5) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "clipboard.write requires protocol 1.5 or later.",
          );
        }
        if (!isClipboardWritePayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "clipboard.write requires one bounded text string.",
          );
        }
        if (!this.hasCapability(sessionId, "clipboard.write")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "clipboard.write requires the clipboard.write capability.",
            { capability: "clipboard.write" },
          );
        }
        this.clipboardText = request.payload.text;
        return this.success("clipboard.write", request.requestId, { status: "written" });

      case "external.open":
        if (request.protocolVersion.minor < 6) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "external.open requires protocol 1.6 or later.",
          );
        }
        if (!isExternalOpenPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "external.open requires one bounded URL string.",
          );
        }
        if (!this.hasCapability(sessionId, "external.open")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "external.open requires the external.open capability.",
            { capability: "external.open" },
          );
        }
        return this.success("external.open", request.requestId, { status: "opened" });

      case "dialog.open_file":
        if (request.protocolVersion.minor < 7) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "dialog.open_file requires protocol 1.7 or later.",
          );
        }
        if (!isFileDialogOpenPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "dialog.open_file requires strict bounded filters.",
          );
        }
        if (!this.hasCapability(sessionId, "dialog.open_file")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "dialog.open_file requires the dialog.open_file capability.",
            { capability: "dialog.open_file" },
          );
        }
        return this.success("dialog.open_file", request.requestId, { status: "cancelled" });

      case "dialog.save_file":
        if (request.protocolVersion.minor < 8) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "dialog.save_file requires protocol 1.8 or later.",
          );
        }
        if (!isFileDialogOpenPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "dialog.save_file requires strict bounded filters.",
          );
        }
        if (!this.hasCapability(sessionId, "dialog.save_file")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "dialog.save_file requires the dialog.save_file capability.",
            { capability: "dialog.save_file" },
          );
        }
        return this.success("dialog.save_file", request.requestId, { status: "cancelled" });

      case "dialog.open_file.v2":
        if (request.protocolVersion.minor < 9) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "dialog.open_file.v2 requires protocol 1.9 or later.",
          );
        }
        if (!isFileDialogOpenPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "dialog.open_file.v2 requires strict bounded filters.",
          );
        }
        if (!this.hasCapability(sessionId, "dialog.open_file")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "dialog.open_file.v2 requires the dialog.open_file capability.",
            { capability: "dialog.open_file" },
          );
        }
        return this.success("dialog.open_file.v2", request.requestId, { status: "cancelled" });

      case "file.read_text":
        if (request.protocolVersion.minor < 9) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "file.read_text requires protocol 1.9 or later.",
          );
        }
        if (!isFileTextReadPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "file.read_text requires one exact selection reference.",
          );
        }
        if (!this.hasCapability(sessionId, "file.read_text")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "file.read_text requires the file.read_text capability.",
            { capability: "file.read_text" },
          );
        }
        return this.failure(
          request.requestId,
          "file.unavailable",
          "selected file is unavailable.",
        );

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

interface UiDocumentState {
  revision: number;
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
