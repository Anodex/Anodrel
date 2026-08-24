import type { ResponseEnvelope, WireRequestEnvelope } from "@anodrel/protocol";

import type { MenuState, UiWindowState } from "../state.js";
import type { MockOperationContext } from "./context.js";
import { dispatchPlatformOperation } from "./platform.js";
import { dispatchServiceOperation } from "./services.js";
import { dispatchUiOperation } from "./ui.js";
import { dispatchWindowOperation } from "./windows.js";

/** Routes an authenticated request to the focused mock operation family. */
export function dispatchMockOperation(
  context: MockOperationContext,
  request: WireRequestEnvelope,
  sessionId: string,
  sessionWindows: UiWindowState,
  menu: MenuState,
): ResponseEnvelope {
  const platformResponse = dispatchPlatformOperation(context, request, sessionId);
  if (platformResponse !== undefined) {
    return platformResponse;
  }

  const windowResponse = dispatchWindowOperation(context, request, sessionId, sessionWindows, menu);
  if (windowResponse !== undefined) {
    return windowResponse;
  }

  const uiResponse = dispatchUiOperation(context, request, sessionId, sessionWindows);
  if (uiResponse !== undefined) {
    return uiResponse;
  }

  const serviceResponse = dispatchServiceOperation(context, request, sessionId);
  if (serviceResponse !== undefined) {
    return serviceResponse;
  }

  return context.failure(
    request.requestId,
    "operation.unsupported",
    `Operation ${request.operation} is not supported by this host.`,
  );
}
