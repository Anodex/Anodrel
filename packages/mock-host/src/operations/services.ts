import type { ResponseEnvelope, WireRequestEnvelope } from "@anodrel/protocol";

import type { MockOperationContext } from "./context.js";
import { dispatchClipboardServiceOperation } from "./services/clipboard.js";
import { dispatchFileServiceOperation } from "./services/files.js";
import { dispatchIntegrationServiceOperation } from "./services/integrations.js";
import { dispatchSessionServiceOperation } from "./services/session.js";
import { dispatchStorageServiceOperation } from "./services/storage.js";

/** Routes an authenticated request to the matching mock service family. */
export function dispatchServiceOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
): ResponseEnvelope | undefined {
  const sessionResponse = dispatchSessionServiceOperation(context, request, sessionId);
  if (sessionResponse !== undefined) {
    return sessionResponse;
  }

  const clipboardResponse = dispatchClipboardServiceOperation(context, request, sessionId);
  if (clipboardResponse !== undefined) {
    return clipboardResponse;
  }

  const integrationResponse = dispatchIntegrationServiceOperation(context, request, sessionId);
  if (integrationResponse !== undefined) {
    return integrationResponse;
  }

  const fileResponse = dispatchFileServiceOperation(context, request, sessionId);
  if (fileResponse !== undefined) {
    return fileResponse;
  }

  return dispatchStorageServiceOperation(context, request, sessionId);
}
