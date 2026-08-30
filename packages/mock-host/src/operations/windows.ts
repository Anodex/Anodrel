import {
  MAX_UI_DOCUMENT_REQUEST_BYTES,
  isContextMenuReplacePayload,
  isEmptyPayload,
  isMenuReplacePayload,
  isRecord,
  isWindowClosePayload,
  isWindowFullscreenSetPayload,
  isWindowOpenPayload,
  isWindowSizeSetPayload,
  isWindowStateSetPayload,
  isWindowTitleProposal,
  isWindowTitleSetPayload,
  type ResponseEnvelope,
  type SecondarySessionWindowId,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import {
  secondaryDocumentOperationMinor,
  type ContextMenuState,
  type MenuState,
  type UiWindowState,
} from "../state.js";
import type { MockOperationContext } from "./context.js";

export function dispatchWindowOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
  sessionWindows: UiWindowState,
  menu: MenuState,
  contextMenu: ContextMenuState,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "window.title.set":
      if (request.protocolVersion.minor < 14) {
        return context.failure(request.requestId, "operation.unsupported", "window.title.set requires protocol 1.14 or later.");
      }
      if (!isWindowTitleSetPayload(request.payload)) {
        // Never echoes the proposal: text refused for being unsafe to display
        // must not be repeated in a failure that reaches a log.
        return context.failure(request.requestId, "request.payload_invalid", "window.title.set requires one title string.");
      }
      if (!context.hasCapability(sessionId, "window.title")) {
        return context.failure(request.requestId, "capability.denied", "window.title.set requires the window.title capability.", { capability: "window.title" });
      }
      // Acceptance only. A real host composes the caption with its validated
      // application-name suffix and does not report what it became.
      return context.success("window.title.set", request.requestId, { status: "applied" });

    case "window.state.set":
      if (request.protocolVersion.minor < 16) {
        return context.failure(request.requestId, "operation.unsupported", "window.state.set requires protocol 1.16 or later.");
      }
      if (!isWindowStateSetPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "window.state.set requires one closed state string.");
      }
      if (!context.hasCapability(sessionId, "window.state")) {
        return context.failure(request.requestId, "capability.denied", "window.state.set requires the window.state capability.", { capability: "window.state" });
      }
      // This is not native observation: the mock records only the closed state
      // it accepted so a separately granted Protocol 1.30 snapshot can model
      // the documented host boundary deterministically.
      if (sessionWindows.presentationState !== request.payload.state) {
        sessionWindows.presentationState = request.payload.state;
        sessionWindows.pendingPresentationState = request.payload.state;
      }
      return context.success("window.state.set", request.requestId, { status: "applied" });

    case "window.state.get":
      if (request.protocolVersion.minor < 30) {
        return context.failure(request.requestId, "operation.unsupported", "window.state.get requires protocol 1.30 or later.");
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "window.state.get accepts no payload fields.");
      }
      if (!context.hasCapability(sessionId, "window.state.read")) {
        return context.failure(request.requestId, "capability.denied", "window.state.get requires the window.state.read capability.", { capability: "window.state.read" });
      }
      return context.success("window.state.get", request.requestId, {
        state: sessionWindows.presentationState,
      });

    case "window.state.changes.read": {
      if (request.protocolVersion.minor < 31) {
        return context.failure(request.requestId, "operation.unsupported", "window.state.changes.read requires protocol 1.31 or later.");
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "window.state.changes.read accepts no payload fields.");
      }
      if (!context.hasCapability(sessionId, "window.state.observe")) {
        return context.failure(request.requestId, "capability.denied", "window.state.changes.read requires the window.state.observe capability.", { capability: "window.state.observe" });
      }
      const state = sessionWindows.pendingPresentationState;
      sessionWindows.pendingPresentationState = null;
      return context.success("window.state.changes.read", request.requestId, { state });
    }

    case "window.focus.request":
      if (request.protocolVersion.minor < 20) {
        return context.failure(request.requestId, "operation.unsupported", "window.focus.request requires protocol 1.20 or later.");
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "window.focus.request accepts no payload fields.");
      }
      if (!context.hasCapability(sessionId, "window.focus")) {
        return context.failure(request.requestId, "capability.denied", "window.focus.request requires the window.focus capability.", { capability: "window.focus" });
      }
      // The mock has no native focus state. It reports only that the host
      // accepted the request, matching the intentionally one-way contract.
      return context.success("window.focus.request", request.requestId, { status: "requested" });

    case "window.fullscreen.set":
      if (request.protocolVersion.minor < 21) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "window.fullscreen.set requires protocol 1.21 or later.",
        );
      }
      if (!isWindowFullscreenSetPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "window.fullscreen.set requires one closed mode string.",
        );
      }
      if (!context.hasCapability(sessionId, "window.fullscreen")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "window.fullscreen.set requires the window.fullscreen capability.",
          { capability: "window.fullscreen" },
        );
      }
      // The mock deliberately stores no fullscreen state. It reports only
      // that the host accepted one closed request, matching the one-way
      // contract rather than simulating monitor or geometry observation.
      return context.success("window.fullscreen.set", request.requestId, { status: "applied" });

    case "window.size.set":
      if (request.protocolVersion.minor < 23) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "window.size.set requires protocol 1.23 or later.",
        );
      }
      if (!isWindowSizeSetPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "window.size.set requires one bounded logical width and height.",
        );
      }
      if (!context.hasCapability(sessionId, "window.size")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "window.size.set requires the window.size capability.",
          { capability: "window.size" },
        );
      }
      // The mock has no native surface. It reports acceptance only, never a
      // current size, position, monitor, DPI, or resulting outer rectangle.
      return context.success("window.size.set", request.requestId, { status: "applied" });

    case "window.open":
    case "window.open.v2":
    case "window.open.v3":
      if (request.protocolVersion.minor < secondaryDocumentOperationMinor(request.operation)) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          `${request.operation} requires a supported protocol version.`,
        );
      }
      if (
        !isRecord(request.payload) ||
        Object.keys(request.payload).length !== 2 ||
        typeof request.payload.title !== "string" ||
        typeof request.payload.document !== "string" ||
        new TextEncoder().encode(request.payload.document).byteLength > MAX_UI_DOCUMENT_REQUEST_BYTES
      ) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          `${request.operation} requires one exact title and bounded document.`,
        );
      }
      if (!isWindowOpenPayload(request.payload) || !isWindowTitleProposal(request.payload.title)) {
        return context.failure(
          request.requestId,
          "window.title_invalid",
          `${request.operation} title is invalid.`,
        );
      }
      if (!context.hasCapability(sessionId, "window.open")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          `${request.operation} requires the window.open capability.`,
          { capability: "window.open" },
        );
      }
      if (!context.hasCapability(sessionId, "ui.document.write")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          `${request.operation} requires the ui.document.write capability.`,
          { capability: "ui.document.write" },
        );
      }
      if (sessionWindows.revisions.size >= 4 || sessionWindows.nextSecondaryId > 65_535) {
        return context.failure(
          request.requestId,
          "window.unavailable",
          "the session window group is unavailable.",
        );
      }
      {
        const windowId = `window-${sessionWindows.nextSecondaryId}` as SecondarySessionWindowId;
        sessionWindows.nextSecondaryId += 1;
        sessionWindows.revisions.set(windowId, 1);
        return context.success(request.operation, request.requestId, { windowId });
      }

    case "window.close":
      if (request.protocolVersion.minor < 25) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "window.close requires protocol 1.25 or later.",
        );
      }
      if (!isWindowClosePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "window.close requires one canonical secondary windowId.",
        );
      }
      if (!context.hasCapability(sessionId, "window.close")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "window.close requires the window.close capability.",
          { capability: "window.close" },
        );
      }
      if (!sessionWindows.revisions.delete(request.payload.windowId)) {
        return context.failure(
          request.requestId,
          "window.unavailable",
          "the requested session window is unavailable.",
        );
      }
      return context.success("window.close", request.requestId, { status: "requested" });

    case "menu.replace":
      if (request.protocolVersion.minor < 18) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "menu.replace requires protocol 1.18 or later.",
        );
      }
      if (!isMenuReplacePayload(request.payload, request.protocolVersion.minor >= 24)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "menu.replace requires one exact bounded complete menu model.",
        );
      }
      if (!context.hasCapability(sessionId, "menu.write")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "menu.replace requires the menu.write capability.",
          { capability: "menu.write" },
        );
      }
      menu.revision += 1;
      return context.success("menu.replace", request.requestId, {
        revision: menu.revision.toString(),
      });

    case "menu.context.replace":
      if (request.protocolVersion.minor < 32) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "menu.context.replace requires protocol 1.32 or later.",
        );
      }
      if (!isContextMenuReplacePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "menu.context.replace requires one exact bounded complete context-menu model.",
        );
      }
      if (!context.hasCapability(sessionId, "menu.context.write")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "menu.context.replace requires the menu.context.write capability.",
          { capability: "menu.context.write" },
        );
      }
      contextMenu.revision += 1;
      return context.success("menu.context.replace", request.requestId, {
        revision: contextMenu.revision.toString(),
      });

  }

  return undefined;
}
