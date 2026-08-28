import {
  classifyFileBinaryWritePayload,
  isFileBinaryWritePayloadShape,
  isFileDialogOpenPayload,
  isFileTextReadPayload,
  isFileTextWritePayload,
  isFolderDialogOpenPayload,
  isFolderEntriesReadPayload,
  type ResponseEnvelope,
  type WireRequestEnvelope,
} from "@anodrel/protocol";

import type { MockOperationContext } from "../context.js";

/** Handles file and folder operations against deliberately unavailable mock resources. */
export function dispatchFileServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  switch (request.operation) {
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

    case "dialog.open_folder":
      if (request.protocolVersion.minor < 28) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.open_folder requires protocol 1.28 or later.",
        );
      }
      if (!isFolderDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.open_folder does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.open_folder")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.open_folder requires the dialog.open_folder capability.",
          { capability: "dialog.open_folder" },
        );
      }
      return context.success("dialog.open_folder", request.requestId, { status: "cancelled" });

    case "dialog.open_folder.v2":
      if (request.protocolVersion.minor < 29) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "dialog.open_folder.v2 requires protocol 1.29 or later.",
        );
      }
      if (!isFolderDialogOpenPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "dialog.open_folder.v2 does not accept a payload.",
        );
      }
      if (!context.hasCapability(sessionId, "dialog.open_folder")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "dialog.open_folder.v2 requires the dialog.open_folder capability.",
          { capability: "dialog.open_folder" },
        );
      }
      return context.success("dialog.open_folder.v2", request.requestId, { status: "cancelled" });

    case "folder.read_entries":
      if (request.protocolVersion.minor < 29) {
        return context.failure(
          request.requestId,
          "operation.unsupported",
          "folder.read_entries requires protocol 1.29 or later.",
        );
      }
      if (!isFolderEntriesReadPayload(request.payload)) {
        return context.failure(
          request.requestId,
          "request.payload_invalid",
          "folder.read_entries requires one exact folder reference.",
        );
      }
      if (!context.hasCapability(sessionId, "folder.read_entries")) {
        return context.failure(
          request.requestId,
          "capability.denied",
          "folder.read_entries requires the folder.read_entries capability.",
          { capability: "folder.read_entries" },
        );
      }
      return context.failure(
        request.requestId,
        "folder.unavailable",
        "selected folder is unavailable.",
      );

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

    default:
      return undefined;
  }
}
