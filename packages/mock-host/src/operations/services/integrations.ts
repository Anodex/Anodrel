import {
  isExternalOpenPayload,
  isNetworkFetchTextPayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "../context.js";

/** Handles externally scoped operations without granting an OS integration. */
export function dispatchIntegrationServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "external.open":
      if (request.protocolVersion.minor < 6) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "external.open requires protocol 1.6 or later.",
        );
      }
      if (!isExternalOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "external.open requires one bounded URL string.",
        );
      }
      if (!context.hasCapability(sessionId, "external.open")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "external.open requires the external.open capability.",
          { capability: "external.open" },
        );
      }
      return context.success("external.open", request.requestId, { status: "opened" });

    case "network.fetch_text":
      if (request.protocolVersion.minor < 19) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "network.fetch_text requires protocol 1.19 or later.",
        );
      }
      if (!isNetworkFetchTextPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "network.fetch_text requires one exact bounded HTTPS URL.",
        );
      }
      if (!context.hasCapability(sessionId, "network.fetch")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "network.fetch_text requires the network.fetch capability.",
          { capability: "network.fetch" },
        );
      }
      if (context.networkTextResponse === undefined) {
        return context.failure(
          request.requestId,
          "network.unavailable",
          "network text fetch is unavailable.",
        );
      }
      return context.success("network.fetch_text", request.requestId, context.networkTextResponse);

    default:
      return undefined;
  }
}
