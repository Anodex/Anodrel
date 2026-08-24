//! Validation for bounded, capability-gated host service requests.

import {
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_CREDENTIAL_NAME_BYTES,
  MAX_CREDENTIAL_SECRET_HEX_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MAX_FILE_BINARY_WRITE_BYTES,
  MAX_FILE_DIALOG_FILTERS,
  MAX_FILE_DIALOG_REQUEST_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MAX_STORAGE_SNAPSHOT_REQUEST_BYTES,
  SAVE_REFERENCE_BYTES,
  SELECTION_REFERENCE_BYTES,
} from "../index.js";
import type { PayloadFor } from "../index.js";
import { canonicalBase64UrlDecodedLength } from "../base64url.js";
import { isRecord, isValidatedHttpsTextFetchUrl, isValidatedHttpsUrl } from "./primitives.js";

/** Validates the exact bounded text payload for a clipboard write. */
export function isClipboardWritePayload(
  value: unknown,
): value is PayloadFor<"clipboard.write"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.text === "string" &&
    new TextEncoder().encode(value.text).byteLength <= MAX_CLIPBOARD_TEXT_REQUEST_BYTES
  );
}

/** Validates the exact bounded URL payload for an external HTTPS handoff. */
export function isExternalOpenPayload(
  value: unknown,
): value is PayloadFor<"external.open"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.url === "string" &&
    new TextEncoder().encode(value.url).byteLength <= MAX_EXTERNAL_LINK_REQUEST_BYTES &&
    isValidatedHttpsUrl(value.url)
  );
}

/** Validates strict structured filters for one host-owned file picker. */
export function isFileDialogOpenPayload(
  value: unknown,
): value is PayloadFor<"dialog.open_file"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    Array.isArray(value.filters) &&
    value.filters.length > 0 &&
    value.filters.length <= MAX_FILE_DIALOG_FILTERS &&
    new TextEncoder().encode(JSON.stringify(value)).byteLength <= MAX_FILE_DIALOG_REQUEST_BYTES &&
    value.filters.every(isFileDialogFilter)
  );
}

/** Validates the exact empty payload for one host-owned folder picker. */
export function isFolderDialogOpenPayload(
  value: unknown,
): value is PayloadFor<"dialog.open_folder"> {
  return isRecord(value) && Object.keys(value).length === 0;
}

/** Validates one exact opaque selection reference for a bounded file text read. */
export function isFileTextReadPayload(
  value: unknown,
): value is PayloadFor<"file.read_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.selectionReference === "string" &&
    value.selectionReference.length === SELECTION_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.selectionReference)
  );
}

/** Validates an exact protocol-safe whole application-state replacement. */
export function isStorageStateReplacePayload(
  value: unknown,
): value is PayloadFor<"storage.state.replace"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.snapshot === "string" &&
    new TextEncoder().encode(value.snapshot).byteLength <= MAX_STORAGE_SNAPSHOT_REQUEST_BYTES
  );
}

/** Validates one exact, non-enumerable credential name. */
export function isCredentialReadPayload(
  value: unknown,
): value is PayloadFor<"credential.read"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    isCredentialName(value.name)
  );
}

/** Validates one credential name and its canonical bounded secret encoding. */
export function isCredentialWritePayload(
  value: unknown,
): value is PayloadFor<"credential.write"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isCredentialName(value.name) &&
    isCanonicalCredentialSecret(value.secret)
  );
}

/** Maximum UTF-16 code units in a notification title. */
export const MAX_NOTIFICATION_TITLE_UTF16_UNITS = 63;

/** Maximum UTF-16 code units in a notification body. */
export const MAX_NOTIFICATION_BODY_UTF16_UNITS = 255;

/**
 * Returns whether a value is exactly one notification title and body.
 *
 * An extra field is a mismatch rather than something to ignore, so a future
 * urgency, icon, or action field cannot be smuggled past protocol 1.13.
 */
export function isNotificationShowPayload(
  value: unknown,
): value is PayloadFor<"notification.show"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    isNotificationText(value.title, MAX_NOTIFICATION_TITLE_UTF16_UNITS, false) &&
    isNotificationText(value.body, MAX_NOTIFICATION_BODY_UTF16_UNITS, true)
  );
}

/**
 * Returns whether a value is bounded notification text.
 *
 * Length is measured in UTF-16 code units because that is what the host's
 * native buffers count. Control characters are rejected so text cannot forge a
 * second message or misrepresent its source; a body may carry line feeds
 * because the target surface renders them as breaks.
 */
function isNotificationText(value: unknown, maximumUnits: number, allowLineFeed: boolean): boolean {
  if (typeof value !== "string" || value.length === 0 || value.length > maximumUnits) {
    return false;
  }
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (allowLineFeed && code === 0x0a) {
      continue;
    }
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) {
      return false;
    }
  }
  return true;
}

export function isNetworkFetchTextPayload(
  value: unknown,
): value is PayloadFor<"network.fetch_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 1 &&
    typeof value.url === "string" &&
    new TextEncoder().encode(value.url).byteLength <= MAX_NETWORK_FETCH_REQUEST_BYTES &&
    isValidatedHttpsTextFetchUrl(value.url)
  );
}

export function isFileTextWritePayload(
  value: unknown,
): value is PayloadFor<"file.write_text"> {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.saveReference === "string" &&
    value.saveReference.length === SAVE_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.saveReference) &&
    typeof value.text === "string" &&
    new TextEncoder().encode(value.text).byteLength <= MAX_FILE_TEXT_WRITE_BYTES
  );
}

/** The bounded validation outcome for one binary-output protocol payload. */
export type FileBinaryWritePayloadStatus = "valid" | "invalid" | "too_large";

/** Validates the exact non-decoding shape of a binary-output request. */
export function isFileBinaryWritePayloadShape(
  value: unknown,
): value is { readonly saveReference: string; readonly bytesBase64Url: string } {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.saveReference === "string" &&
    value.saveReference.length === SAVE_REFERENCE_BYTES &&
    /^[A-Za-z0-9_-]+$/.test(value.saveReference) &&
    typeof value.bytesBase64Url === "string"
  );
}

/**
 * Classifies one exact binary-output payload without decoding it.
 *
 * This shares the Rust core's strict canonical base64url grammar so the mock
 * can preserve the same invalid-versus-too-large protocol outcome.
 */
export function classifyFileBinaryWritePayload(value: unknown): FileBinaryWritePayloadStatus {
  if (!isFileBinaryWritePayloadShape(value)) {
    return "invalid";
  }
  const decodedLength = canonicalBase64UrlDecodedLength(value.bytesBase64Url);
  if (decodedLength === undefined) {
    return "invalid";
  }
  return decodedLength > MAX_FILE_BINARY_WRITE_BYTES ? "too_large" : "valid";
}

/** Validates one exact canonical bounded binary-output payload. */
export function isFileBinaryWritePayload(
  value: unknown,
): value is PayloadFor<"file.write_binary"> {
  return classifyFileBinaryWritePayload(value) === "valid";
}

/** Returns whether a value is the stable protocol credential-name grammar. */
export function isCredentialName(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_CREDENTIAL_NAME_BYTES &&
    /^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?$/.test(value)
  );
}

/** Returns whether a value is a non-empty, lowercase, even-length secret hex string. */
export function isCanonicalCredentialSecret(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_CREDENTIAL_SECRET_HEX_BYTES &&
    value.length % 2 === 0 &&
    /^[0-9a-f]+$/.test(value)
  );
}

function isFileDialogFilter(value: unknown): boolean {
  return (
    isRecord(value) &&
    Object.keys(value).length === 2 &&
    typeof value.label === "string" &&
    value.label.length > 0 &&
    value.label.length <= 64 &&
    /^[\x20-\x7e]+$/.test(value.label) &&
    Array.isArray(value.extensions) &&
    value.extensions.length > 0 &&
    value.extensions.length <= 8 &&
    value.extensions.every(
      (extension) =>
        typeof extension === "string" && extension.length > 0 && extension.length <= 16 && /^[a-z0-9]+$/.test(extension),
    )
  );
}

