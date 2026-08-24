import {
  assert,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

test("storage state keeps read, replace, and clear grants separate", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: [
        "storage.state.read",
        "storage.state.replace",
        "storage.state.clear",
      ],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.readStorageState(), { status: "absent" });
  assert.deepEqual(await client.replaceStorageState("saved state"), { status: "replaced" });
  assert.deepEqual(await client.readStorageState(), { status: "snapshot", snapshot: "saved state" });
  assert.deepEqual(await client.clearStorageState(), { status: "cleared" });
  assert.deepEqual(await client.readStorageState(), { status: "absent" });

  const readOnly = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["storage.state.read"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => readOnly.replaceStorageState("cannot save"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "storage.state.replace",
  );

  await assert.rejects(
    () => client.replaceStorageState("x".repeat(24 * 1024 + 1)),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});

test("credentials keep exact read, write, and delete grants separate", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["credential.read", "credential.write", "credential.delete"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.readCredential("refresh-token"), { status: "not_found" });
  assert.deepEqual(await client.writeCredential("refresh-token", "00aaff"), { status: "written" });
  assert.deepEqual(await client.readCredential("refresh-token"), { status: "found", secret: "00aaff" });
  assert.deepEqual(await client.deleteCredential("refresh-token"), { status: "deleted" });
  assert.deepEqual(await client.deleteCredential("refresh-token"), { status: "not_found" });

  const readOnly = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["credential.read"],
      credentials: { "refresh-token": "00aaff" },
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => readOnly.writeCredential("refresh-token", "00aaff"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "credential.write",
  );
  await assert.rejects(
    () => client.writeCredential("refresh-token", "not-hex"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});
