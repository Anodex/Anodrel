import {
  assert,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

test("notifications are one bounded announce behind their own grant", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["notification.show"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  // Accepted means handed over, never seen. There is no field here that could
  // tell an application whether the user has it silenced or muted.
  assert.deepEqual(await client.showNotification("Build finished", "Two targets\nzero warnings"), {
    status: "shown",
  });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.showNotification("Build finished", "Two targets"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "notification.show",
  );
});

test("notification text is bounded and cannot forge a second message", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["notification.show"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  const rejected: ReadonlyArray<readonly [string, string]> = [
    ["", "body"],
    ["title", ""],
    // A carriage return or an escape could present one notification as two
    // messages, or misrepresent where it came from.
    ["Build\rFAILED", "body"],
    ["title", "Build[31m"],
    // A title is a single line; only a body may carry breaks.
    ["first\nsecond", "body"],
    ["t".repeat(64), "body"],
    ["title", "b".repeat(256)],
  ];

  for (const [title, body] of rejected) {
    await assert.rejects(
      () => client.showNotification(title, body),
      (error: unknown) =>
        error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
      `${JSON.stringify(title)} / ${JSON.stringify(body)} was accepted`,
    );
  }

  // A refusal must not become a way to have the host repeat content, so the
  // failure carries none of the text it rejected. The marker is distinctive
  // enough that a generic message cannot contain it by coincidence.
  await assert.rejects(
    () => client.showNotification("Sensitive\rMarkerZQX", "body"),
    (error: unknown) =>
      error instanceof PlatformRemoteError && !error.message.includes("MarkerZQX"),
  );

  // Each value is accepted exactly at its bound.
  assert.deepEqual(await client.showNotification("t".repeat(63), "b".repeat(255)), {
    status: "shown",
  });
});
