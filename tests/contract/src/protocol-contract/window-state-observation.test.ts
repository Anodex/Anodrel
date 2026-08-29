import {
  assert,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

test("a session state snapshot is pull-only and separately granted", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.state", "window.state.read"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.getWindowState(), { state: "restored" });
  assert.deepEqual(await client.setWindowState("maximized"), { status: "applied" });
  assert.deepEqual(await client.getWindowState(), { state: "maximized" });

  const no_read_grant = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["window.state"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => no_read_grant.getWindowState(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.state.read",
  );
});

test("a state snapshot accepts no query, target, or subscription fields", async () => {
  const transport = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.state.read"],
  }).createTransport();

  for (const payload of [
    { target: "another-window" },
    { includeBounds: true },
    { subscribe: true },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 30 },
      kind: "request",
      requestId: `window-state-get-${JSON.stringify(payload)}`,
      operation: "window.state.get",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 29 },
    kind: "request",
    requestId: "window-state-get-before-protocol-1.30",
    operation: "window.state.get",
    payload: {},
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});
