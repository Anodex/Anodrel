//! Validation of the protocol envelopes that select a host operation.

import {
  MAX_CANCELLATION_ID_BYTES,
  MAX_OPERATION_BYTES,
  MAX_REQUEST_ID_BYTES,
} from "../index.js";
import type { CancellationEnvelope, EmptyPayload, PayloadFor, WireRequestEnvelope } from "../index.js";
import { isLimitedIdentifier, isProtocolVersion, isRecord } from "./primitives.js";

export function isWireRequestEnvelope(value: unknown): value is WireRequestEnvelope {
  if (!isRecord(value) || value.kind !== "request") {
    return false;
  }

  return (
    isProtocolVersion(value.protocolVersion) &&
    typeof value.requestId === "string" &&
    isLimitedIdentifier(value.requestId, MAX_REQUEST_ID_BYTES) &&
    typeof value.operation === "string" &&
    isLimitedIdentifier(value.operation, MAX_OPERATION_BYTES) &&
    (value.cancellationId === undefined ||
      (typeof value.cancellationId === "string" &&
        isLimitedIdentifier(value.cancellationId, MAX_CANCELLATION_ID_BYTES)))
  );
}

export function isCancellationEnvelope(value: unknown): value is CancellationEnvelope {
  return (
    isRecord(value) &&
    value.kind === "cancel" &&
    isProtocolVersion(value.protocolVersion) &&
    typeof value.cancellationId === "string" &&
    isLimitedIdentifier(value.cancellationId, MAX_CANCELLATION_ID_BYTES)
  );
}

export function isPingPayload(value: unknown): value is PayloadFor<"platform.ping"> {
  return isRecord(value) && typeof value.sentAt === "string";
}

export function isEmptyPayload(value: unknown): value is EmptyPayload {
  return isRecord(value) && Object.keys(value).length === 0;
}
