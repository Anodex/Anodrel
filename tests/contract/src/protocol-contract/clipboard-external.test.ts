import {
  assert,
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

test("SDK and host keep clipboard read and write grants separate", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["clipboard.read", "clipboard.write"],
    clipboardText: "before",
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.readClipboardText(), { status: "text", text: "before" });
  assert.deepEqual(await client.writeClipboardText("after"), { status: "written" });
  assert.deepEqual(await client.readClipboardText(), { status: "text", text: "after" });
});

test("clipboard operations require their exact host-issued grants", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.readClipboardText(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "clipboard.read",
  );
  await assert.rejects(
    () => client.writeClipboardText("not written"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "clipboard.write",
  );
});

test("mock clipboard refuses text outside the protocol envelope limit", () => {
  assert.throws(
    () =>
      new MockHost({
        applicationId: "test.application",
        clipboardText: "x".repeat(MAX_CLIPBOARD_TEXT_REQUEST_BYTES + 1),
      }),
    /clipboard text exceeds/,
  );
});

test("SDK and host agree on a separately granted external HTTPS handoff", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["external.open"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.openExternalLink("https://docs.anodrel.dev/guide"), {
    status: "opened",
  });
});

test("external link operation checks its grant and bounded payload", async () => {
  const host = new MockHost({ applicationId: "test.application" });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  await assert.rejects(
    () => client.openExternalLink("https://docs.anodrel.dev/guide"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "external.open",
  );

  const granted = new PlatformClient(
    new MockHost({ applicationId: "test.application", grantedCapabilities: ["external.open"] })
      .createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => granted.openExternalLink("x".repeat(MAX_EXTERNAL_LINK_REQUEST_BYTES + 1)),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
  await assert.rejects(
    () => granted.openExternalLink("file:///C:/private.txt"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});
