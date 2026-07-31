import assert from "node:assert/strict";
import test from "node:test";

import { BootstrapError, decodeBootstrapInvitation } from "./bootstrap.js";

const token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

function frame(payload: string): Buffer {
  const body = Buffer.from(payload, "utf8");
  const output = Buffer.alloc(12 + body.byteLength);
  output.write("ANBI", 0, "ascii");
  output.writeUInt16LE(1, 4);
  output.writeUInt16LE(0, 6);
  output.writeUInt32LE(body.byteLength, 8);
  body.copy(output, 12);
  return output;
}

test("reads and consumes a strict bootstrap invitation", () => {
  const invitation = decodeBootstrapInvitation(
    frame(`{"kind":"bootstrap.invitation","pipeName":"\\\\\\\\.\\\\pipe\\\\anodrel.v1.test","protocolVersion":{"major":1,"minor":0},"sessionId":"test","token":"${token}"}`),
  );
  assert.deepEqual(invitation.takeAuthentication(), {
    kind: "session.authenticate",
    sessionId: "test",
    token,
  });
  assert.throws(() => invitation.takeAuthentication(), BootstrapError);
});

test("rejects a duplicate bootstrap field", () => {
  const payload = `{"kind":"bootstrap.invitation","kind":"bootstrap.invitation","pipeName":"\\\\\\\\.\\\\pipe\\\\anodrel.v1.test","protocolVersion":{"major":1,"minor":0},"sessionId":"test","token":"${token}"}`;
  assert.throws(() => decodeBootstrapInvitation(frame(payload)), BootstrapError);
});
