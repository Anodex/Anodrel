import {
  assert,
  fixedTime,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  PROTOCOL_VERSION,
  SequenceRequestIds,
  test,
} from "./support.js";

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

test("diagnostic reads expose only the granted closed record shape", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["diagnostics.read"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.readDiagnosticEntries(), {
    entries: [
      {
        sequence: "1",
        level: "info",
        component: "core",
        event: "Internal platform.health check completed.",
      },
    ],
  });
});

test("diagnostic reads require the existing host-issued diagnostics grant", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.readDiagnosticEntries(),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "capability.denied",
  );
});
