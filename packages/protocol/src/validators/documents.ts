//! Validation of bounded semantic document replacement.

import { MAX_UI_DOCUMENT_REQUEST_BYTES } from "../index.js";
import type { PayloadFor } from "../index.js";
import { isRecord } from "./primitives.js";

/** Validates the bounded outer payload for an authenticated UI document update. */
export function isUiDocumentReplacePayload(
  value: unknown,
): value is PayloadFor<"ui.document.replace"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.document === "string" &&
    new TextEncoder().encode(value.document).byteLength <= MAX_UI_DOCUMENT_REQUEST_BYTES
  );
}
