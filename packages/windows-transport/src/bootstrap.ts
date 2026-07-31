import { TextDecoder } from "node:util";

import { isStrictObject, parseStrictJson, type StrictJsonObject } from "./strict-json.js";

const MAGIC = "ANBI";
const HEADER_BYTES = 12;
const MAX_PAYLOAD_BYTES = 2_048;
const MAX_FRAME_BYTES = HEADER_BYTES + MAX_PAYLOAD_BYTES;
const PIPE_PREFIX = "\\\\.\\pipe\\anodrel.v1.";

export class BootstrapError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BootstrapError";
  }
}

export class BootstrapInvitation {
  #token: string;

  private constructor(
    readonly pipeName: string,
    readonly sessionId: string,
    token: string,
  ) {
    this.#token = token;
  }

  static fromValidated(
    pipeName: string,
    sessionId: string,
    token: string,
  ): BootstrapInvitation {
    return new BootstrapInvitation(pipeName, sessionId, token);
  }

  /** Returns the only valid authentication object and clears this object's token. */
  takeAuthentication(): { readonly kind: "session.authenticate"; readonly sessionId: string; readonly token: string } {
    if (this.#token.length === 0) {
      throw new BootstrapError("Bootstrap invitation was already consumed.");
    }
    const token = this.#token;
    this.#token = "";
    return { kind: "session.authenticate", sessionId: this.sessionId, token };
  }
}

export async function readBootstrapInvitation(
  source: AsyncIterable<Uint8Array>,
): Promise<BootstrapInvitation> {
  const chunks: Buffer[] = [];
  let length = 0;
  for await (const chunk of source) {
    const bytes = Buffer.from(chunk);
    length += bytes.byteLength;
    if (length > MAX_FRAME_BYTES) {
      throw new BootstrapError("Bootstrap input exceeds the frame limit.");
    }
    chunks.push(bytes);
  }
  const frame = Buffer.concat(chunks, length);
  try {
    return decodeBootstrapInvitation(frame);
  } finally {
    frame.fill(0);
    for (const chunk of chunks) {
      chunk.fill(0);
    }
  }
}

export function decodeBootstrapInvitation(frame: Uint8Array): BootstrapInvitation {
  const bytes = Buffer.from(frame);
  if (bytes.byteLength < HEADER_BYTES) {
    throw new BootstrapError("Bootstrap frame is truncated.");
  }
  if (bytes.subarray(0, 4).toString("ascii") !== MAGIC) {
    throw new BootstrapError("Bootstrap frame magic is invalid.");
  }
  if (bytes.readUInt16LE(4) !== 1 || bytes.readUInt16LE(6) !== 0) {
    throw new BootstrapError("Bootstrap frame version is unsupported.");
  }
  const payloadLength = bytes.readUInt32LE(8);
  if (payloadLength > MAX_PAYLOAD_BYTES || bytes.byteLength !== HEADER_BYTES + payloadLength) {
    throw new BootstrapError("Bootstrap frame length is invalid.");
  }
  let payload: StrictJsonObject;
  try {
    const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(HEADER_BYTES));
    const parsed = parseStrictJson(text);
    if (!isStrictObject(parsed)) {
      throw new BootstrapError("Bootstrap payload is not an object.");
    }
    payload = parsed;
  } catch (error) {
    if (error instanceof BootstrapError) {
      throw error;
    }
    throw new BootstrapError("Bootstrap payload is not strict UTF-8 JSON.");
  }
  return invitationFromPayload(payload);
}

function invitationFromPayload(payload: StrictJsonObject): BootstrapInvitation {
  const kind = payload.kind;
  const version = payload.protocolVersion;
  const pipeName = payload.pipeName;
  const sessionId = payload.sessionId;
  const token = payload.token;
  if (
    Object.keys(payload).length !== 5 ||
    kind !== "bootstrap.invitation" ||
    !isStrictObject(version) ||
    Object.keys(version).length !== 2 ||
    version.major !== 1 ||
    version.minor !== 0 ||
    typeof pipeName !== "string" ||
    typeof sessionId !== "string" ||
    typeof token !== "string" ||
    !pipeName.startsWith(PIPE_PREFIX) ||
    Buffer.byteLength(pipeName, "utf8") > 512 ||
    Buffer.byteLength(sessionId, "utf8") === 0 ||
    Buffer.byteLength(sessionId, "utf8") > 128 ||
    !/^[0-9a-f]{64}$/u.test(token)
  ) {
    throw new BootstrapError("Bootstrap payload fields are invalid.");
  }
  return BootstrapInvitation.fromValidated(pipeName, sessionId, token);
}
