import assert from "node:assert/strict";
import test from "node:test";
import { MockHost } from "@anodrel/mock-host";
import { PROTOCOL_VERSION, createRequest, isWireRequestEnvelope } from "@anodrel/protocol";
import { PlatformClient, PlatformRemoteError, type RequestIdFactory } from "@anodrel/sdk";

const fixedTime = () => new Date("2026-07-31T12:00:00.000Z");

test("SDK and host agree on successful core operations", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["diagnostics.read"],
    hostName: "test-host",
    now: fixedTime,
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.ping("2026-07-31T11:59:00.000Z"), {
    receivedAt: "2026-07-31T12:00:00.000Z",
    hostName: "test-host",
  });
  assert.deepEqual(await client.getCapabilities(), {
    applicationId: "test.application",
    grantedCapabilities: ["diagnostics.read"],
  });
  assert.deepEqual(await client.getHealth(), {
    status: "ready",
    hostName: "test-host",
    protocolVersion: PROTOCOL_VERSION,
  });
});

test("SDK and host bind UI document revisions to one granted transport session", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["ui.document.write"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());
  const document =
    '{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Hello","fontSize":16,"tone":"primary"}}';

  assert.deepEqual(await client.replaceUiDocument(document), { revision: "1" });
  assert.deepEqual(await client.replaceUiDocument(document), { revision: "2" });
});

test("UI document replacement requires a host-issued grant", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.replaceUiDocument('{"format":"anodrel.ui.document.v1","root":{}}'),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "ui.document.write",
  );
});

test("SDK and host agree on a bounded granted UI event read", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["ui.events.read"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.readUiEvents(), {
    events: [],
    dropped: 0,
    discarded: 0,
  });
});

test("UI event reads require a host-issued grant", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.readUiEvents(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "ui.events.read",
  );
});

test("SDK and host agree on a granted session close request", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["session.close"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.closeSession(), { status: "accepted" });
});

test("session close requires a host-issued grant", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.closeSession(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "session.close",
  );
});

test("host derives grants from policy and ignores a forged request context", async () => {
  const host = new MockHost({ applicationId: "test.application", now: fixedTime });
  const request = {
    ...createRequest("request-1", "platform.health", {}),
    capabilityContext: {
      applicationId: "forged.application",
      sessionId: "forged-session",
      grantedCapabilities: ["diagnostics.read"],
    },
  };

  const response = await host.handle(request);
  assert.equal(response.status, "failure");
  if (response.status === "failure") {
    assert.equal(response.error.code, "capability.denied");
  }
});

test("host rejects unsupported versions and malformed payloads with typed errors", async () => {
  const host = new MockHost({ applicationId: "test.application", now: fixedTime });
  const unsupported = await host.handle({
    ...createRequest("request-1", "platform.ping", { sentAt: "2026-07-31T11:59:00.000Z" }),
    protocolVersion: { major: 2, minor: 0 },
  });
  assert.equal(unsupported.status, "failure");
  if (unsupported.status === "failure") {
    assert.equal(unsupported.error.code, "protocol.version_unsupported");
  }

  const malformed = await host.handle({
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId: "request-2",
    operation: "platform.ping",
    payload: {},
  });
  assert.equal(malformed.status, "failure");
  if (malformed.status === "failure") {
    assert.equal(malformed.error.code, "request.payload_invalid");
  }
});

test("protocol enforces UTF-8 identifier limits before a host can echo them", async () => {
  const host = new MockHost({ applicationId: "test.application", now: fixedTime });
  const oversizedRequestId = "🙂".repeat(65);
  const request = {
    ...createRequest("request-1", "platform.health", {}),
    requestId: oversizedRequestId,
  };

  assert.equal(isWireRequestEnvelope(request), false);
  const response = await host.handle(request);
  assert.equal(response.status, "failure");
  if (response.status === "failure") {
    assert.equal(response.error.code, "request.invalid");
  }
});

test("a cancellation identity prevents work that has not started", async () => {
  const host = new MockHost({ applicationId: "test.application", now: fixedTime });
  const transport = host.createTransport();
  await transport.cancel({
    protocolVersion: PROTOCOL_VERSION,
    kind: "cancel",
    cancellationId: "cancel-1",
  });

  const response = await transport.send(
    createRequest(
      "request-1",
      "platform.ping",
      { sentAt: "2026-07-31T11:59:00.000Z" },
      "cancel-1",
    ),
  );
  assert.equal(response.status, "failure");
  if (response.status === "failure") {
    assert.equal(response.error.code, "request.cancelled");
  }
});

test("SDK turns a structured host failure into a typed error", async () => {
  const host = new MockHost({ applicationId: "test.application", now: fixedTime });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(client.getHealth(), (error: unknown) => {
    assert.ok(error instanceof PlatformRemoteError);
    assert.equal(error.code, "capability.denied");
    return true;
  });
});

class SequenceRequestIds implements RequestIdFactory {
  private current = 0;

  next(): string {
    this.current += 1;
    return `request-${this.current}`;
  }
}
