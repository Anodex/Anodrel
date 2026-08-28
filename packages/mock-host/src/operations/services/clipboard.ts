import {
  isClipboardWritePayload,
  isEmptyPayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "../context.js";

/** Handles clipboard reads and writes against host-owned mock state. */
export function dispatchClipboardServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "clipboard.read":
      if (request.protocolVersion.minor < 5) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "clipboard.read requires protocol 1.5 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "clipboard.read does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "clipboard.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "clipboard.read requires the clipboard.read capability.",
          { capability: "clipboard.read" },
        );
      }
      return context.success(
        "clipboard.read",
        request.requestId,
        context.clipboardText === undefined
          ? { status: "no_text" }
          : { status: "text", text: context.clipboardText },
      );

    case "clipboard.write":
      if (request.protocolVersion.minor < 5) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "clipboard.write requires protocol 1.5 or later.",
        );
      }
      if (!isClipboardWritePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "clipboard.write requires one bounded text string.",
        );
      }
      if (!context.hasCapability(sessionId, "clipboard.write")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "clipboard.write requires the clipboard.write capability.",
          { capability: "clipboard.write" },
        );
      }
      context.clipboardText = request.payload.text;
      return context.success("clipboard.write", request.requestId, { status: "written" });

    default:
      return undefined;
  }
}
