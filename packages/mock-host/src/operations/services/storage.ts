import {
  isEmptyPayload,
  isStorageStateReplacePayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "../context.js";

/** Handles the mock host's one bounded atomic storage snapshot. */
export function dispatchStorageServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "storage.state.read":
      if (request.protocolVersion.minor < 10) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "storage.state.read requires protocol 1.10 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "storage.state.read requires an empty payload.",
        );
      }
      if (!context.hasCapability(sessionId, "storage.state.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "storage.state.read requires the storage.state.read capability.",
          { capability: "storage.state.read" },
        );
      }
      return context.success(
        "storage.state.read",
        request.requestId,
        context.storageSnapshot === undefined
          ? { status: "absent" }
          : { status: "snapshot", snapshot: context.storageSnapshot },
      );

    case "storage.state.replace":
      if (request.protocolVersion.minor < 10) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "storage.state.replace requires protocol 1.10 or later.",
        );
      }
      if (!isStorageStateReplacePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "storage.state.replace requires one bounded snapshot.",
        );
      }
      if (!context.hasCapability(sessionId, "storage.state.replace")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "storage.state.replace requires the storage.state.replace capability.",
          { capability: "storage.state.replace" },
        );
      }
      context.storageSnapshot = request.payload.snapshot;
      return context.success("storage.state.replace", request.requestId, { status: "replaced" });

    case "storage.state.clear":
      if (request.protocolVersion.minor < 10) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "storage.state.clear requires protocol 1.10 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "storage.state.clear requires an empty payload.",
        );
      }
      if (!context.hasCapability(sessionId, "storage.state.clear")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "storage.state.clear requires the storage.state.clear capability.",
          { capability: "storage.state.clear" },
        );
      }
      context.storageSnapshot = undefined;
      return context.success("storage.state.clear", request.requestId, { status: "cleared" });

    default:
      return undefined;
  }
}
