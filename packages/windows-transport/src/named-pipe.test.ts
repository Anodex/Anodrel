import assert from "node:assert/strict";
import { createServer, type Socket } from "node:net";
import test from "node:test";

import { PlatformClient } from "@anodrel/sdk";

import { BootstrapInvitation } from "./bootstrap.js";
import { WindowsNamedPipeTransport } from "./named-pipe.js";

const token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

test("uses an authenticated Windows pipe through the public SDK", { timeout: 5_000 }, async () => {
  const pipeName = `\\\\.\\pipe\\anodrel-transport-test-${process.pid}-${Date.now()}`;
  const server = createServer((socket) => serveSession(socket));
  server.listen(pipeName);
  await new Promise<void>((resolve, reject) => {
    server.once("listening", resolve);
    server.once("error", reject);
  });

  const invitation = BootstrapInvitation.fromValidated(pipeName, "transport-test", token);
  const transport = await WindowsNamedPipeTransport.connect(invitation);
  const client = new PlatformClient(transport, { next: () => "health-request" });
  const health = await client.getHealth();
  assert.equal(health.status, "ready");
  await transport.close();
  await new Promise<void>((resolve, reject) => {
    server.close((error) => (error === undefined ? resolve() : reject(error)));
  });
});

function serveSession(socket: Socket): void {
  let buffer = Buffer.alloc(0);
  let authenticated = false;
  socket.on("data", (chunk: Buffer) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (buffer.byteLength >= 12) {
      const length = buffer.readUInt32LE(8);
      if (buffer.byteLength < 12 + length) {
        return;
      }
      const payload = JSON.parse(buffer.subarray(12, 12 + length).toString("utf8")) as {
        readonly kind: string;
      };
      buffer = buffer.subarray(12 + length);
      if (!authenticated) {
        assert.equal(payload.kind, "session.authenticate");
        authenticated = true;
        socket.write(frame({ kind: "session.authenticated" }));
        continue;
      }
      assert.equal(payload.kind, "request");
      socket.write(
        frame({
          protocolVersion: { major: 1, minor: 0 },
          kind: "response",
          requestId: "health-request",
          status: "success",
          result: {
            status: "ready",
            hostName: "test-host",
            protocolVersion: { major: 1, minor: 0 },
          },
          diagnostics: { hostName: "test-host" },
        }),
      );
    }
  });
  socket.on("end", () => {
    socket.end();
  });
}

function frame(payload: unknown): Buffer {
  const body = Buffer.from(JSON.stringify(payload), "utf8");
  const output = Buffer.alloc(12 + body.byteLength);
  output.write("ANDR", 0, "ascii");
  output.writeUInt16LE(1, 4);
  output.writeUInt16LE(0, 6);
  output.writeUInt32LE(body.byteLength, 8);
  body.copy(output, 12);
  return output;
}
