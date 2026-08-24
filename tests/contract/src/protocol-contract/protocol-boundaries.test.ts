import {
  assert,
  createRequest,
  fixedTime,
  isWireRequestEnvelope,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  PROTOCOL_VERSION,
  SequenceRequestIds,
  test,
} from "./support.js";

test("a field read is a whole-surface snapshot behind its own grant", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["ui.fields.read"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  // The mock has no surface, and that is the same single code a real host
  // returns when it has none — an application cannot tell the cases apart.
  await assert.rejects(
    () => client.readUiFields(),
    (error: unknown) =>
      error instanceof PlatformRemoteError && error.code === "ui.fields.unavailable",
  );

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.readUiFields(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "ui.fields.read",
  );

  // No selector may ride along. Its absence is what stops a caller narrowing a
  // read to one field and repeating it until the typing is reconstructed.
  const transport = host.createTransport();
  for (const payload of [
    { id: "password" },
    { fields: ["password"] },
    { since: 1 },
    { includeCaret: true },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 15 },
      kind: "request",
      requestId: `selector-${JSON.stringify(payload)}`,
      operation: "ui.fields.read",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }
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
