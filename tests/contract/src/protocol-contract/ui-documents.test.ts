import {
  assert,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  SequenceRequestIds,
  test,
} from "./support.js";

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

test("SDK and host expose version two UI document replacement separately", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["ui.document.write"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());
  const document =
    '{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"text","value":"Hello","fontSize":16,"tone":"primary"}}}';

  assert.deepEqual(await client.replaceUiDocumentV2(document), { revision: "1" });
});

test("SDK and host keep secondary scroll documents on their explicit operations", async () => {
  const document =
    '{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"text","value":"Hello","fontSize":16,"tone":"primary"}}}';
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.open", "ui.document.write"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  const opened = await client.openWindowV2("Scrollable notes", document);
  assert.deepEqual(opened, { windowId: "window-1" });
  assert.deepEqual(await client.replaceUiDocumentV2InWindow(opened.windowId, document), {
    revision: "2",
  });

  const older = await host.createTransport().send({
    protocolVersion: { major: 1, minor: 26 },
    kind: "request",
    requestId: "scroll-before-protocol-1.27",
    operation: "window.open.v2",
    payload: { title: "Scrollable notes", document },
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});

test("SDK and host keep version three status documents on their explicit operations", async () => {
  const document =
    '{"format":"anodrel.ui.document.v3","root":{"id":"result","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}';
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.open", "ui.document.write"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.replaceUiDocumentV3(document), { revision: "1" });
  const opened = await client.openWindowV3("Result", document);
  assert.deepEqual(opened, { windowId: "window-1" });
  assert.deepEqual(await client.replaceUiDocumentV3InWindow(opened.windowId, document), {
    revision: "2",
  });

  const older = await host.createTransport().send({
    protocolVersion: { major: 1, minor: 25 },
    kind: "request",
    requestId: "status-before-protocol-1.26",
    operation: "ui.document.replace.v3",
    payload: { document },
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
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
