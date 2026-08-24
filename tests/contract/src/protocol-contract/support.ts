import assert from "node:assert/strict";
import test from "node:test";
import { MockHost } from "@anodrel/mock-host";
import {
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MAX_FILE_BINARY_WRITE_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_MENU_REPLACE_REQUEST_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MAX_WINDOW_CLIENT_HEIGHT,
  MAX_WINDOW_CLIENT_WIDTH,
  MIN_WINDOW_CLIENT_HEIGHT,
  MIN_WINDOW_CLIENT_WIDTH,
  PROTOCOL_VERSION,
  createRequest,
  isWireRequestEnvelope,
} from "@anodrel/protocol";
import { PlatformClient, PlatformRemoteError, type RequestIdFactory } from "@anodrel/sdk";

export {
  assert,
  test,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MAX_FILE_BINARY_WRITE_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_MENU_REPLACE_REQUEST_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MAX_WINDOW_CLIENT_HEIGHT,
  MAX_WINDOW_CLIENT_WIDTH,
  MIN_WINDOW_CLIENT_HEIGHT,
  MIN_WINDOW_CLIENT_WIDTH,
  PROTOCOL_VERSION,
  createRequest,
  isWireRequestEnvelope,
};

export const fixedTime = () => new Date("2026-07-31T12:00:00.000Z");

export class SequenceRequestIds implements RequestIdFactory {
  private current = 0;

  next(): string {
    this.current += 1;
    return `request-${this.current}`;
  }
}
