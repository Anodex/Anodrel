import assert from "node:assert/strict";
import test from "node:test";
import { MockHost } from "@anodrel/mock-host";
import {
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  PROTOCOL_VERSION,
  createRequest,
  isWireRequestEnvelope,
} from "@anodrel/protocol";
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

test("SDK and host agree on a capability-checked file dialog cancellation", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.openFileDialog([{ label: "Text", extensions: ["txt"] }]), {
    status: "cancelled",
  });
});

test("file dialog validates filters and its independent host grant", async () => {
  const denied = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => denied.openFileDialog([{ label: "Text", extensions: ["txt"] }]),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "dialog.open_file",
  );

  const granted = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => granted.openFileDialog([{ label: "Raw", extensions: ["*.txt"] }]),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});

test("SDK and host agree on a separately authorized save dialog", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.save_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.saveFileDialog([{ label: "Text", extensions: ["txt"] }]), {
    status: "cancelled",
  });

  const openOnlyClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => openOnlyClient.saveFileDialog([{ label: "Text", extensions: ["txt"] }]),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "dialog.save_file",
  );

  await assert.rejects(
    () => client.saveFileDialog([{ label: "Raw", extensions: ["*.txt"] }]),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});

test("selection-scoped file text keeps selection and reading separately granted", async () => {
  const selectionClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  assert.deepEqual(
    await selectionClient.openFileDialogWithReference([{ label: "Text", extensions: ["txt"] }]),
    { status: "cancelled" },
  );

  await assert.rejects(
    () => selectionClient.readSelectedFileText("AbCdEfGhIjKlMnOpQrStUv"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "file.read_text",
  );

  const readClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["file.read_text"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => readClient.readSelectedFileText("AbCdEfGhIjKlMnOpQrStUv"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "file.unavailable",
  );
  await assert.rejects(
    () => readClient.readSelectedFileText("C:/private.txt"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
});

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
