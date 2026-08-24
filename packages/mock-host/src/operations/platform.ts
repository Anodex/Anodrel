import {
  PROTOCOL_VERSION,
  isCredentialReadPayload,
  isCredentialWritePayload,
  isEmptyPayload,
  isNotificationShowPayload,
  isPingPayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "./context.js";

export function dispatchPlatformOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "platform.ping":
      if (!isPingPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "platform.ping requires a sentAt string.",
        );
      }
      return context.success("platform.ping", request.requestId, {
        receivedAt: context.now().toISOString(),
        hostName: context.hostName,
      });

    case "platform.capabilities":
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "platform.capabilities does not accept a payload.",
        );
      }
      return context.success("platform.capabilities", request.requestId, {
        applicationId: context.applicationId,
        grantedCapabilities: [...context.grantedCapabilities],
      });

    case "platform.health":
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "platform.health does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "diagnostics.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "platform.health requires the diagnostics.read capability.",
          { capability: "diagnostics.read" },
        );
      }
      return context.success("platform.health", request.requestId, {
        status: "ready",
        hostName: context.hostName,
        protocolVersion: PROTOCOL_VERSION,
      });

    case "diagnostics.entries.read":
      if (request.protocolVersion.minor < 11) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "diagnostics.entries.read requires protocol 1.11 or later.",
        );
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "diagnostics.entries.read does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "diagnostics.read")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "diagnostics.entries.read requires the diagnostics.read capability.",
          { capability: "diagnostics.read" },
        );
      }
      return context.success("diagnostics.entries.read", request.requestId, {
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
        return context.failure(request.requestId, "operation.unsupported", "credential.read requires protocol 1.12 or later.");
      }
      if (!isCredentialReadPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "credential.read requires one exact credential name.");
      }
      if (!context.hasCapability(sessionId, "credential.read")) {
        return context.failure(request.requestId, "capability.denied", "credential.read requires the credential.read capability.", { capability: "credential.read" });
      }
      {
        const secret = context.credentials.get(request.payload.name);
        return context.success("credential.read", request.requestId, secret === undefined
          ? { status: "not_found" }
          : { status: "found", secret });
      }

    case "credential.write":
      if (request.protocolVersion.minor < 12) {
        return context.failure(request.requestId, "operation.unsupported", "credential.write requires protocol 1.12 or later.");
      }
      if (!isCredentialWritePayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "credential.write requires one exact credential name and canonical secret.");
      }
      if (!context.hasCapability(sessionId, "credential.write")) {
        return context.failure(request.requestId, "capability.denied", "credential.write requires the credential.write capability.", { capability: "credential.write" });
      }
      context.credentials.set(request.payload.name, request.payload.secret);
      return context.success("credential.write", request.requestId, { status: "written" });

    case "credential.delete":
      if (request.protocolVersion.minor < 12) {
        return context.failure(request.requestId, "operation.unsupported", "credential.delete requires protocol 1.12 or later.");
      }
      if (!isCredentialReadPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "credential.delete requires one exact credential name.");
      }
      if (!context.hasCapability(sessionId, "credential.delete")) {
        return context.failure(request.requestId, "capability.denied", "credential.delete requires the credential.delete capability.", { capability: "credential.delete" });
      }
      return context.success("credential.delete", request.requestId, {
        status: context.credentials.delete(request.payload.name) ? "deleted" : "not_found",
      });

    case "notification.show":
      if (request.protocolVersion.minor < 13) {
        return context.failure(request.requestId, "operation.unsupported", "notification.show requires protocol 1.13 or later.");
      }
      if (!isNotificationShowPayload(request.payload)) {
        // The failure never echoes the offending text back: a refusal must
        // not become a way to have the host repeat content.
        return context.failure(request.requestId, "request.payload_invalid", "notification.show requires one title and one body string.");
      }
      if (!context.hasCapability(sessionId, "notification.show")) {
        return context.failure(request.requestId, "capability.denied", "notification.show requires the notification.show capability.", { capability: "notification.show" });
      }
      // Handed over, never seen: the mock reports acceptance and nothing
      // about what a user would have experienced.
      return context.success("notification.show", request.requestId, { status: "shown" });

  }

  return undefined;
}

