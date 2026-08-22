import {
  PROTOCOL_VERSION,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_STORAGE_SNAPSHOT_REQUEST_BYTES,
  MAX_UI_DOCUMENT_REQUEST_BYTES,
  isCancellationEnvelope,
  isClipboardWritePayload,
  isCredentialReadPayload,
  isCredentialWritePayload,
  isMenuReplacePayload,
  isNotificationShowPayload,
  isWindowFullscreenSetPayload,
  isWindowSizeSetPayload,
  isWindowStateSetPayload,
  isWindowTitleSetPayload,
  isExternalOpenPayload,
  isNetworkFetchTextPayload,
  isRecord,
  isFileDialogOpenPayload,
  classifyFileBinaryWritePayload,
  isFileBinaryWritePayloadShape,
  isFileTextReadPayload,
  isFileTextWritePayload,
  isEmptyPayload,
  isUiDocumentReplaceWindowPayload,
  isUiDocumentReplacePayload,
  isWindowClosePayload,
  isWindowOpenPayload,
  isWindowTitleProposal,
  isPingPayload,
  isStorageStateReplacePayload,
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
  type SecondarySessionWindowId,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

export interface MockHostOptions {
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

    return {
      send: async <TOperation extends PlatformOperation>(
        request: RequestEnvelope<TOperation>,
      ) =>
        this.handle(request, sessionId, cancelled, sessionWindows, menu) as Promise<ResponseEnvelope<TOperation>>,
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

    return this.dispatch(request, sessionId, sessionWindows, menu);
  }

  private dispatch(
    request: WireRequestEnvelope,
    sessionId: string,
    sessionWindows: UiWindowState,
    menu: MenuState,
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

      case "diagnostics.entries.read":
        if (request.protocolVersion.minor < 11) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "diagnostics.entries.read requires protocol 1.11 or later.",
          );
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "diagnostics.entries.read does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "diagnostics.read")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "diagnostics.entries.read requires the diagnostics.read capability.",
            { capability: "diagnostics.read" },
          );
        }
        return this.success("diagnostics.entries.read", request.requestId, {
          entries: [
            {
              sequence: "1",
              level: "info",
              component: "core",
              event: "Internal platform.health check completed.",
            },
          ],
        });

      case "credential.read":
        if (request.protocolVersion.minor < 12) {
          return this.failure(request.requestId, "operation.unsupported", "credential.read requires protocol 1.12 or later.");
        }
        if (!isCredentialReadPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "credential.read requires one exact credential name.");
        }
        if (!this.hasCapability(sessionId, "credential.read")) {
          return this.failure(request.requestId, "capability.denied", "credential.read requires the credential.read capability.", { capability: "credential.read" });
        }
        {
          const secret = this.credentials.get(request.payload.name);
          return this.success("credential.read", request.requestId, secret === undefined
            ? { status: "not_found" }
            : { status: "found", secret });
        }

      case "credential.write":
        if (request.protocolVersion.minor < 12) {
          return this.failure(request.requestId, "operation.unsupported", "credential.write requires protocol 1.12 or later.");
        }
        if (!isCredentialWritePayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "credential.write requires one exact credential name and canonical secret.");
        }
        if (!this.hasCapability(sessionId, "credential.write")) {
          return this.failure(request.requestId, "capability.denied", "credential.write requires the credential.write capability.", { capability: "credential.write" });
        }
        this.credentials.set(request.payload.name, request.payload.secret);
        return this.success("credential.write", request.requestId, { status: "written" });

      case "credential.delete":
        if (request.protocolVersion.minor < 12) {
          return this.failure(request.requestId, "operation.unsupported", "credential.delete requires protocol 1.12 or later.");
        }
        if (!isCredentialReadPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "credential.delete requires one exact credential name.");
        }
        if (!this.hasCapability(sessionId, "credential.delete")) {
          return this.failure(request.requestId, "capability.denied", "credential.delete requires the credential.delete capability.", { capability: "credential.delete" });
        }
        return this.success("credential.delete", request.requestId, {
          status: this.credentials.delete(request.payload.name) ? "deleted" : "not_found",
        });

      case "notification.show":
        if (request.protocolVersion.minor < 13) {
          return this.failure(request.requestId, "operation.unsupported", "notification.show requires protocol 1.13 or later.");
        }
        if (!isNotificationShowPayload(request.payload)) {
          // The failure never echoes the offending text back: a refusal must
          // not become a way to have the host repeat content.
          return this.failure(request.requestId, "request.payload_invalid", "notification.show requires one title and one body string.");
        }
        if (!this.hasCapability(sessionId, "notification.show")) {
          return this.failure(request.requestId, "capability.denied", "notification.show requires the notification.show capability.", { capability: "notification.show" });
        }
        // Handed over, never seen: the mock reports acceptance and nothing
        // about what a user would have experienced.
        return this.success("notification.show", request.requestId, { status: "shown" });

      case "window.title.set":
        if (request.protocolVersion.minor < 14) {
          return this.failure(request.requestId, "operation.unsupported", "window.title.set requires protocol 1.14 or later.");
        }
        if (!isWindowTitleSetPayload(request.payload)) {
          // Never echoes the proposal: text refused for being unsafe to display
          // must not be repeated in a failure that reaches a log.
          return this.failure(request.requestId, "request.payload_invalid", "window.title.set requires one title string.");
        }
        if (!this.hasCapability(sessionId, "window.title")) {
          return this.failure(request.requestId, "capability.denied", "window.title.set requires the window.title capability.", { capability: "window.title" });
        }
        // Acceptance only. A real host composes the caption with its validated
        // application-name suffix and does not report what it became.
        return this.success("window.title.set", request.requestId, { status: "applied" });

      case "window.state.set":
        if (request.protocolVersion.minor < 16) {
          return this.failure(request.requestId, "operation.unsupported", "window.state.set requires protocol 1.16 or later.");
        }
        if (!isWindowStateSetPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "window.state.set requires one closed state string.");
        }
        if (!this.hasCapability(sessionId, "window.state")) {
          return this.failure(request.requestId, "capability.denied", "window.state.set requires the window.state capability.", { capability: "window.state" });
        }
        // The mock deliberately reports acceptance only. It has no native
        // window and cannot reveal a resulting state, handle, or geometry.
        return this.success("window.state.set", request.requestId, { status: "applied" });

      case "window.focus.request":
        if (request.protocolVersion.minor < 20) {
          return this.failure(request.requestId, "operation.unsupported", "window.focus.request requires protocol 1.20 or later.");
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "window.focus.request accepts no payload fields.");
        }
        if (!this.hasCapability(sessionId, "window.focus")) {
          return this.failure(request.requestId, "capability.denied", "window.focus.request requires the window.focus capability.", { capability: "window.focus" });
        }
        // The mock has no native focus state. It reports only that the host
        // accepted the request, matching the intentionally one-way contract.
        return this.success("window.focus.request", request.requestId, { status: "requested" });

      case "window.fullscreen.set":
        if (request.protocolVersion.minor < 21) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "window.fullscreen.set requires protocol 1.21 or later.",
          );
        }
        if (!isWindowFullscreenSetPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "window.fullscreen.set requires one closed mode string.",
          );
        }
        if (!this.hasCapability(sessionId, "window.fullscreen")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "window.fullscreen.set requires the window.fullscreen capability.",
            { capability: "window.fullscreen" },
          );
        }
        // The mock deliberately stores no fullscreen state. It reports only
        // that the host accepted one closed request, matching the one-way
        // contract rather than simulating monitor or geometry observation.
        return this.success("window.fullscreen.set", request.requestId, { status: "applied" });

      case "window.size.set":
        if (request.protocolVersion.minor < 23) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "window.size.set requires protocol 1.23 or later.",
          );
        }
        if (!isWindowSizeSetPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "window.size.set requires one bounded logical width and height.",
          );
        }
        if (!this.hasCapability(sessionId, "window.size")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "window.size.set requires the window.size capability.",
            { capability: "window.size" },
          );
        }
        // The mock has no native surface. It reports acceptance only, never a
        // current size, position, monitor, DPI, or resulting outer rectangle.
        return this.success("window.size.set", request.requestId, { status: "applied" });

      case "window.open":
        if (request.protocolVersion.minor < 25) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "window.open requires protocol 1.25 or later.",
          );
        }
        if (
          !isRecord(request.payload) ||
          Object.keys(request.payload).length !== 2 ||
          typeof request.payload.title !== "string" ||
          typeof request.payload.document !== "string" ||
          new TextEncoder().encode(request.payload.document).byteLength > MAX_UI_DOCUMENT_REQUEST_BYTES
        ) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "window.open requires one exact title and bounded document.",
          );
        }
        if (!isWindowOpenPayload(request.payload) || !isWindowTitleProposal(request.payload.title)) {
          return this.failure(
            request.requestId,
            "window.title_invalid",
            "window.open title is invalid.",
          );
        }
        if (!this.hasCapability(sessionId, "window.open")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "window.open requires the window.open capability.",
            { capability: "window.open" },
          );
        }
        if (!this.hasCapability(sessionId, "ui.document.write")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "window.open requires the ui.document.write capability.",
            { capability: "ui.document.write" },
          );
        }
        if (sessionWindows.revisions.size >= 4 || sessionWindows.nextSecondaryId > 65_535) {
          return this.failure(
            request.requestId,
            "window.unavailable",
            "the session window group is unavailable.",
          );
        }
        {
          const windowId = `window-${sessionWindows.nextSecondaryId}` as SecondarySessionWindowId;
          sessionWindows.nextSecondaryId += 1;
          sessionWindows.revisions.set(windowId, 1);
          return this.success("window.open", request.requestId, { windowId });
        }

      case "window.close":
        if (request.protocolVersion.minor < 25) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "window.close requires protocol 1.25 or later.",
          );
        }
        if (!isWindowClosePayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "window.close requires one canonical secondary windowId.",
          );
        }
        if (!this.hasCapability(sessionId, "window.close")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "window.close requires the window.close capability.",
            { capability: "window.close" },
          );
        }
        if (!sessionWindows.revisions.delete(request.payload.windowId)) {
          return this.failure(
            request.requestId,
            "window.unavailable",
            "the requested session window is unavailable.",
          );
        }
        return this.success("window.close", request.requestId, { status: "requested" });

      case "menu.replace":
        if (request.protocolVersion.minor < 18) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "menu.replace requires protocol 1.18 or later.",
          );
        }
        if (!isMenuReplacePayload(request.payload, request.protocolVersion.minor >= 24)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "menu.replace requires one exact bounded complete menu model.",
          );
        }
        if (!this.hasCapability(sessionId, "menu.write")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "menu.replace requires the menu.write capability.",
            { capability: "menu.write" },
          );
        }
        menu.revision += 1;
        return this.success("menu.replace", request.requestId, {
          revision: menu.revision.toString(),
        });

      case "ui.fields.read":
        if (request.protocolVersion.minor < 15) {
          return this.failure(request.requestId, "operation.unsupported", "ui.fields.read requires protocol 1.15 or later.");
        }
        // No selector, deliberately: one would let a caller narrow a read and
        // repeat it until the typing had been reconstructed.
        if (!isEmptyPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "ui.fields.read accepts no payload fields.");
        }
        if (!this.hasCapability(sessionId, "ui.fields.read")) {
          return this.failure(request.requestId, "capability.denied", "ui.fields.read requires the ui.fields.read capability.", { capability: "ui.fields.read" });
        }
        // The mock has no surface, which is the same answer a real host gives
        // when it has none: one code, so an application cannot tell the cases
        // apart by asking.
        return this.failure(request.requestId, "ui.fields.unavailable", "no field values are available.");

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
        const revision = (sessionWindows.revisions.get("main") ?? 0) + 1;
        sessionWindows.revisions.set("main", revision);
        return this.success(request.operation, request.requestId, {
          revision: revision.toString(),
        });

      case "ui.document.replace.window":
        if (request.protocolVersion.minor < 25) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "ui.document.replace.window requires protocol 1.25 or later.",
          );
        }
        if (!isUiDocumentReplaceWindowPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "ui.document.replace.window requires one canonical windowId and bounded document.",
          );
        }
        if (!this.hasCapability(sessionId, "ui.document.write")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "ui.document.replace.window requires the ui.document.write capability.",
            { capability: "ui.document.write" },
          );
        }
        {
          const revision = sessionWindows.revisions.get(request.payload.windowId);
          if (revision === undefined) {
            return this.failure(
              request.requestId,
              "window.unavailable",
              "the requested session window is unavailable.",
            );
          }
          const next = revision + 1;
          sessionWindows.revisions.set(request.payload.windowId, next);
          return this.success("ui.document.replace.window", request.requestId, {
            revision: next.toString(),
          });
        }

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

      case "ui.events.read.window":
        if (request.protocolVersion.minor < 25) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "ui.events.read.window requires protocol 1.25 or later.",
          );
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "ui.events.read.window does not accept a payload.",
          );
        }
        if (!this.hasCapability(sessionId, "ui.events.read")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "ui.events.read.window requires the ui.events.read capability.",
            { capability: "ui.events.read" },
          );
        }
        return this.success("ui.events.read.window", request.requestId, {
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

      case "network.fetch_text":
        if (request.protocolVersion.minor < 19) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "network.fetch_text requires protocol 1.19 or later.",
          );
        }
        if (!isNetworkFetchTextPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "network.fetch_text requires one exact bounded HTTPS URL.",
          );
        }
        if (!this.hasCapability(sessionId, "network.fetch")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "network.fetch_text requires the network.fetch capability.",
            { capability: "network.fetch" },
          );
        }
        if (this.networkTextResponse === undefined) {
          return this.failure(
            request.requestId,
            "network.unavailable",
            "network text fetch is unavailable.",
          );
        }
        return this.success("network.fetch_text", request.requestId, this.networkTextResponse);

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

      case "dialog.save_file.v2":
        if (request.protocolVersion.minor < 17) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "dialog.save_file.v2 requires protocol 1.17 or later.",
          );
        }
        if (!isFileDialogOpenPayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "dialog.save_file.v2 requires strict bounded filters.",
          );
        }
        if (!this.hasCapability(sessionId, "dialog.save_file")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "dialog.save_file.v2 requires the dialog.save_file capability.",
            { capability: "dialog.save_file" },
          );
        }
        return this.success("dialog.save_file.v2", request.requestId, { status: "cancelled" });

      case "file.write_text":
        if (request.protocolVersion.minor < 17) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "file.write_text requires protocol 1.17 or later.",
          );
        }
        if (!isFileTextWritePayload(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "file.write_text requires one exact save reference and bounded text.",
          );
        }
        if (!this.hasCapability(sessionId, "file.write_text")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "file.write_text requires the file.write_text capability.",
            { capability: "file.write_text" },
          );
        }
        return this.failure(
          request.requestId,
          "file.unavailable",
          "selected output is unavailable.",
        );

      case "file.write_binary": {
        if (request.protocolVersion.minor < 22) {
          return this.failure(
            request.requestId,
            "operation.unsupported",
            "file.write_binary requires protocol 1.22 or later.",
          );
        }
        if (!isFileBinaryWritePayloadShape(request.payload)) {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "file.write_binary requires one exact save reference and base64url data.",
          );
        }
        if (!this.hasCapability(sessionId, "file.write_binary")) {
          return this.failure(
            request.requestId,
            "capability.denied",
            "file.write_binary requires the file.write_binary capability.",
            { capability: "file.write_binary" },
          );
        }
        const binaryStatus = classifyFileBinaryWritePayload(request.payload);
        if (binaryStatus === "invalid") {
          return this.failure(
            request.requestId,
            "request.payload_invalid",
            "file.write_binary requires canonical base64url data.",
          );
        }
        if (binaryStatus === "too_large") {
          return this.failure(
            request.requestId,
            "file.binary_too_large",
            "selected output binary data is too large.",
          );
        }
        return this.failure(
          request.requestId,
          "file.unavailable",
          "selected output is unavailable.",
        );
      }

      case "storage.state.read":
        if (request.protocolVersion.minor < 10) {
          return this.failure(request.requestId, "operation.unsupported", "storage.state.read requires protocol 1.10 or later.");
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "storage.state.read requires an empty payload.");
        }
        if (!this.hasCapability(sessionId, "storage.state.read")) {
          return this.failure(request.requestId, "capability.denied", "storage.state.read requires the storage.state.read capability.", { capability: "storage.state.read" });
        }
        return this.success("storage.state.read", request.requestId, this.storageSnapshot === undefined ? { status: "absent" } : { status: "snapshot", snapshot: this.storageSnapshot });

      case "storage.state.replace":
        if (request.protocolVersion.minor < 10) {
          return this.failure(request.requestId, "operation.unsupported", "storage.state.replace requires protocol 1.10 or later.");
        }
        if (!isStorageStateReplacePayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "storage.state.replace requires one bounded snapshot.");
        }
        if (!this.hasCapability(sessionId, "storage.state.replace")) {
          return this.failure(request.requestId, "capability.denied", "storage.state.replace requires the storage.state.replace capability.", { capability: "storage.state.replace" });
        }
        this.storageSnapshot = request.payload.snapshot;
        return this.success("storage.state.replace", request.requestId, { status: "replaced" });

      case "storage.state.clear":
        if (request.protocolVersion.minor < 10) {
          return this.failure(request.requestId, "operation.unsupported", "storage.state.clear requires protocol 1.10 or later.");
        }
        if (!isEmptyPayload(request.payload)) {
          return this.failure(request.requestId, "request.payload_invalid", "storage.state.clear requires an empty payload.");
        }
        if (!this.hasCapability(sessionId, "storage.state.clear")) {
          return this.failure(request.requestId, "capability.denied", "storage.state.clear requires the storage.state.clear capability.", { capability: "storage.state.clear" });
        }
        this.storageSnapshot = undefined;
        return this.success("storage.state.clear", request.requestId, { status: "cleared" });

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

interface UiWindowState {
  nextSecondaryId: number;
  readonly revisions: Map<string, number>;
}

function createUiWindowState(): UiWindowState {
  return {
    nextSecondaryId: 1,
    revisions: new Map([["main", 0]]),
  };
}

interface MenuState {
  revision: number;
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
