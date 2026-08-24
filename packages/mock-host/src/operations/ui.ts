import {
  isEmptyPayload,
  isUiDocumentReplacePayload,
  isUiDocumentReplaceWindowPayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import { secondaryDocumentOperationMinor, type UiWindowState } from "../state.js";
import type { MockOperationContext } from "./context.js";

export function dispatchUiOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
  sessionWindows: UiWindowState,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "ui.fields.read":
      if (request.protocolVersion.minor < 15) {
        return context.failure(request.requestId, "operation.unsupported", "ui.fields.read requires protocol 1.15 or later.");
      }
      // No selector, deliberately: one would let a caller narrow a read and
      // repeat it until the typing had been reconstructed.
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "ui.fields.read accepts no payload fields.");
      }
      if (!context.hasCapability(sessionId, "ui.fields.read")) {
        return context.failure(request.requestId, "capability.denied", "ui.fields.read requires the ui.fields.read capability.", { capability: "ui.fields.read" });
      }
      // The mock has no surface, which is the same answer a real host gives
      // when it has none: one code, so an application cannot tell the cases
      // apart by asking.
      return context.failure(request.requestId, "ui.fields.unavailable", "no field values are available.");

    case "ui.document.replace":
    case "ui.document.replace.v2":
    case "ui.document.replace.v3":
      if (request.protocolVersion.minor < (request.operation === "ui.document.replace.v3" ? 26 : request.operation === "ui.document.replace.v2" ? 4 : 1)) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          `${request.operation} requires a supported protocol version.`,
        );
      }
      if (!isUiDocumentReplacePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          `${request.operation} requires one bounded document string.`,
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
      const revision = (sessionWindows.revisions.get("main") ?? 0) + 1;
      sessionWindows.revisions.set("main", revision);
      return context.success(request.operation, request.requestId, {
        revision: revision.toString(),
      });

    case "ui.document.replace.window":
    case "ui.document.replace.window.v2":
    case "ui.document.replace.window.v3":
      if (request.protocolVersion.minor < secondaryDocumentOperationMinor(request.operation)) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          `${request.operation} requires a supported protocol version.`,
        );
      }
      if (!isUiDocumentReplaceWindowPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          `${request.operation} requires one canonical windowId and bounded document.`,
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
      {
        const revision = sessionWindows.revisions.get(request.payload.windowId);
        if (revision === undefined) {
          return context.failure(
            request.requestId,
            "window.unavailable",
            "the requested session window is unavailable.",
          );
        }
        const next = revision + 1;
        sessionWindows.revisions.set(request.payload.windowId, next);
        return context.success(request.operation, request.requestId, {
          revision: next.toString(),
        });
      }

    case "ui.events.read":
      if (request.protocolVersion.minor < 2) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "ui.events.read requires protocol 1.2 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "ui.events.read does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "ui.events.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "ui.events.read requires the ui.events.read capability.",
          { capability: "ui.events.read" },
        );
      }
      return context.success("ui.events.read", request.requestId, {
        events: [],
        dropped: 0,
        discarded: 0,
      });

    case "ui.events.read.window":
      if (request.protocolVersion.minor < 25) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "ui.events.read.window requires protocol 1.25 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "ui.events.read.window does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "ui.events.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "ui.events.read.window requires the ui.events.read capability.",
          { capability: "ui.events.read" },
        );
      }
      return context.success("ui.events.read.window", request.requestId, {
        events: [],
        dropped: 0,
        discarded: 0,
      });

  }

  return undefined;
}
