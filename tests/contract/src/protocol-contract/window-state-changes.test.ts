import {
  assert,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

test("session state changes are coalesced, pull-only, and separately granted", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["window.state", "window.state.observe"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.readWindowStateChanges(), { state: null });
  await client.setWindowState("maximized");
  await client.setWindowState("minimized");
  assert.deepEqual(await client.readWindowStateChanges(), { state: "minimized" });
  assert.deepEqual(await client.readWindowStateChanges(), { state: null });

  const no_observe_grant = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["window.state"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => no_observe_grant.readWindowStateChanges(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.state.observe",
  );
});

test("state changes accepts no target, query, or subscription fields", async () => {
  const transport = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.state.observe"],
  }).createTransport();

  for (const payload of [
    { target: "another-window" },
    { wait: true },
    { subscribe: true },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 31 },
      kind: "request",
      requestId: `window-state-changes-${JSON.stringify(payload)}`,
      operation: "window.state.changes.read",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 30 },
    kind: "request",
    requestId: "window-state-changes-before-protocol-1.31",
    operation: "window.state.changes.read",
    payload: {},
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});
