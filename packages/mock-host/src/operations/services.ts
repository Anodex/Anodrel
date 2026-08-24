import {
  classifyFileBinaryWritePayload,
  isClipboardWritePayload,
  isEmptyPayload,
  isExternalOpenPayload,
  isFileBinaryWritePayloadShape,
  isFileDialogOpenPayload,
  isFileTextReadPayload,
  isFileTextWritePayload,
  isNetworkFetchTextPayload,
  isStorageStateReplacePayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "./context.js";

export function dispatchServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
    case "session.close":
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

    case "dialog.open_file":
      if (request.protocolVersion.minor < 7) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.open_file requires protocol 1.7 or later.",
        );
      }
      if (!isFileDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.open_file requires strict bounded filters.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.open_file")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.open_file requires the dialog.open_file capability.",
          { capability: "dialog.open_file" },
        );
      }
      return context.success("dialog.open_file", request.requestId, { status: "cancelled" });

    case "dialog.save_file":
      if (request.protocolVersion.minor < 8) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.save_file requires protocol 1.8 or later.",
        );
      }
      if (!isFileDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.save_file requires strict bounded filters.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.save_file")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.save_file requires the dialog.save_file capability.",
          { capability: "dialog.save_file" },
        );
      }
      return context.success("dialog.save_file", request.requestId, { status: "cancelled" });

    case "dialog.open_file.v2":
      if (request.protocolVersion.minor < 9) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.open_file.v2 requires protocol 1.9 or later.",
        );
      }
      if (!isFileDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.open_file.v2 requires strict bounded filters.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.open_file")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.open_file.v2 requires the dialog.open_file capability.",
          { capability: "dialog.open_file" },
        );
      }
      return context.success("dialog.open_file.v2", request.requestId, { status: "cancelled" });

    case "file.read_text":
      if (request.protocolVersion.minor < 9) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "file.read_text requires protocol 1.9 or later.",
        );
      }
      if (!isFileTextReadPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "file.read_text requires one exact selection reference.",
        );
      }
      if (!context.hasCapability(sessionId, "file.read_text")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "file.read_text requires the file.read_text capability.",
          { capability: "file.read_text" },
        );
      }
      return context.failure(
        request.requestId,
        "file.unavailable",
        "selected file is unavailable.",
      );

    case "dialog.save_file.v2":
      if (request.protocolVersion.minor < 17) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.save_file.v2 requires protocol 1.17 or later.",
        );
      }
      if (!isFileDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.save_file.v2 requires strict bounded filters.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.save_file")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.save_file.v2 requires the dialog.save_file capability.",
          { capability: "dialog.save_file" },
        );
      }
      return context.success("dialog.save_file.v2", request.requestId, { status: "cancelled" });

    case "file.write_text":
      if (request.protocolVersion.minor < 17) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "file.write_text requires protocol 1.17 or later.",
        );
      }
      if (!isFileTextWritePayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "file.write_text requires one exact save reference and bounded text.",
        );
      }
      if (!context.hasCapability(sessionId, "file.write_text")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "file.write_text requires the file.write_text capability.",
          { capability: "file.write_text" },
        );
      }
      return context.failure(
        request.requestId,
        "file.unavailable",
        "selected output is unavailable.",
      );

    case "file.write_binary": {
      if (request.protocolVersion.minor < 22) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "file.write_binary requires protocol 1.22 or later.",
        );
      }
      if (!isFileBinaryWritePayloadShape(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "file.write_binary requires one exact save reference and base64url data.",
        );
      }
      if (!context.hasCapability(sessionId, "file.write_binary")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "file.write_binary requires the file.write_binary capability.",
          { capability: "file.write_binary" },
        );
      }
      const binaryStatus = classifyFileBinaryWritePayload(request.payload);
      if (binaryStatus === "invalid") {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "file.write_binary requires canonical base64url data.",
        );
      }
      if (binaryStatus === "too_large") {
        return context.failure(
          request.requestId,
          "file.binary_too_large",
          "selected output binary data is too large.",
        );
      }
      return context.failure(
        request.requestId,
        "file.unavailable",
        "selected output is unavailable.",
      );
    }

    case "storage.state.read":
      if (request.protocolVersion.minor < 10) {
        return context.failure(request.requestId, "operation.unsupported", "storage.state.read requires protocol 1.10 or later.");
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "storage.state.read requires an empty payload.");
      }
      if (!context.hasCapability(sessionId, "storage.state.read")) {
        return context.failure(request.requestId, "capability.denied", "storage.state.read requires the storage.state.read capability.", { capability: "storage.state.read" });
      }
      return context.success("storage.state.read", request.requestId, context.storageSnapshot === undefined ? { status: "absent" } : { status: "snapshot", snapshot: context.storageSnapshot });

    case "storage.state.replace":
      if (request.protocolVersion.minor < 10) {
        return context.failure(request.requestId, "operation.unsupported", "storage.state.replace requires protocol 1.10 or later.");
      }
      if (!isStorageStateReplacePayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "storage.state.replace requires one bounded snapshot.");
      }
      if (!context.hasCapability(sessionId, "storage.state.replace")) {
        return context.failure(request.requestId, "capability.denied", "storage.state.replace requires the storage.state.replace capability.", { capability: "storage.state.replace" });
      }
      context.storageSnapshot = request.payload.snapshot;
      return context.success("storage.state.replace", request.requestId, { status: "replaced" });

    case "storage.state.clear":
      if (request.protocolVersion.minor < 10) {
        return context.failure(request.requestId, "operation.unsupported", "storage.state.clear requires protocol 1.10 or later.");
      }
      if (!isEmptyPayload(request.payload)) {
        return context.failure(request.requestId, "request.payload_invalid", "storage.state.clear requires an empty payload.");
      }
      if (!context.hasCapability(sessionId, "storage.state.clear")) {
        return context.failure(request.requestId, "capability.denied", "storage.state.clear requires the storage.state.clear capability.", { capability: "storage.state.clear" });
      }
      context.storageSnapshot = undefined;
      return context.success("storage.state.clear", request.requestId, { status: "cleared" });

  }

  return undefined;
}

