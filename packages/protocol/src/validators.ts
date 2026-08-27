//! Runtime validation exports, grouped by the protocol boundary they protect.

export {
  isCancellationEnvelope,
  isEmptyPayload,
  isPingPayload,
  isWireRequestEnvelope,
} from "./validators/envelopes.js";
export { isUiDocumentReplacePayload } from "./validators/documents.js";
export {
  MAX_NOTIFICATION_BODY_UTF16_UNITS,
  MAX_NOTIFICATION_TITLE_UTF16_UNITS,
  classifyFileBinaryWritePayload,
  isCanonicalCredentialSecret,
  isClipboardWritePayload,
  isCredentialName,
  isCredentialReadPayload,
  isCredentialWritePayload,
  isExternalOpenPayload,
  isFileBinaryWritePayload,
  isFileBinaryWritePayloadShape,
  isFileDialogOpenPayload,
  isFolderEntriesReadPayload,
  isFolderDialogOpenPayload,
  isFileTextReadPayload,
  isFileTextWritePayload,
  isNetworkFetchTextPayload,
  isNotificationShowPayload,
  isStorageStateReplacePayload,
} from "./validators/services.js";
export type { FileBinaryWritePayloadStatus } from "./validators/services.js";
export { isMenuReplacePayload } from "./validators/menus.js";
export {
  MAX_WINDOW_TITLE_UTF16_UNITS,
  isCanonicalSecondaryWindowId,
  isCanonicalSessionWindowId,
  isUiDocumentReplaceWindowPayload,
  isWindowClosePayload,
  isWindowFullscreenSetPayload,
  isWindowOpenPayload,
  isWindowSizeSetPayload,
  isWindowStateSetPayload,
  isWindowTitleProposal,
  isWindowTitleSetPayload,
} from "./validators/windows.js";
export type {
  MenuActionInvokedEvent,
  UiInteractionEvent,
  WindowUiInteractionEvent,
} from "./validators/windows.js";
