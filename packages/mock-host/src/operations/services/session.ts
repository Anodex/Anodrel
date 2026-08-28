import { isEmptyPayload, type ResponseEnvelope, type WireRequestEnvelope } from "@anodrel/protocol";

import type { MockOperationContext } from "../context.js";

/** Handles the closed session-lifecycle operation family. */
export function dispatchSessionServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  if (request.operation !== "session.close") {
    return undefined;
  }
  if (request.protocolVersion.minor < 3) {
    return context.failure(
      request.requestId,
      "operation.unsupported",
      "session.close requires protocol 1.3 or later.",
    );
  }
  if (!isEmptyPayload(request.payload)) {
    return context.failure(
      request.requestId,
      "request.payload_invalid",
      "session.close does not accept a payload.",
    );
  }
  if (!context.hasCapability(sessionId, "session.close")) {
    return context.failure(
      request.requestId,
      "capability.denied",
      "session.close requires the session.close capability.",
      { capability: "session.close" },
    );
  }
  return context.success("session.close", request.requestId, { status: "accepted" });
}
