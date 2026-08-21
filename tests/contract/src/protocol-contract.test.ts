import assert from "node:assert/strict";
import test from "node:test";
import { MockHost } from "@anodrel/mock-host";
import {
  MAX_CLIPBOARD_TEXT_REQUEST_BYTES,
  MAX_EXTERNAL_LINK_REQUEST_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_MENU_REPLACE_REQUEST_BYTES,
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

test("SDK and host agree on one separately granted bounded HTTPS text fetch", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["network.fetch"],
      networkTextResponse: { statusCode: 200, text: "healthy" },
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.fetchHttpsText("https://api.example.test/status?format=text"), {
    statusCode: 200,
    text: "healthy",
  });
});

test("HTTPS text fetch checks its protocol version, exact payload, and host grant", async () => {
  const denied = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      networkTextResponse: { statusCode: 200, text: "healthy" },
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => denied.fetchHttpsText("https://api.example.test/status"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "network.fetch",
  );

  const granted = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["network.fetch"],
      networkTextResponse: { statusCode: 200, text: "healthy" },
    }).createTransport(),
    new SequenceRequestIds(),
  );
  for (const url of [
    "https://127.0.0.1/status",
    "https://api.example.test/status#fragment",
    "https://api.example.test/status%1",
    "x".repeat(MAX_NETWORK_FETCH_REQUEST_BYTES + 1),
  ]) {
    await assert.rejects(
      () => granted.fetchHttpsText(url),
      (error: unknown) =>
        error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
      `${url} was accepted`,
    );
  }

  const older = await new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["network.fetch"],
    networkTextResponse: { statusCode: 200, text: "healthy" },
  })
    .createTransport()
    .send({
      protocolVersion: { major: 1, minor: 18 },
      kind: "request",
      requestId: "network-before-protocol-1.19",
      operation: "network.fetch_text",
      payload: { url: "https://api.example.test/status" },
    });
  assert.equal(older.status, "failure");
  if (older.status === "failure") {
    assert.equal(older.error.code, "operation.unsupported");
  }
});

test("selection-scoped file writing keeps save selection and writing separately granted", async () => {
  const selectionClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.save_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  assert.deepEqual(
    await selectionClient.saveFileDialogWithReference([{ label: "Text", extensions: ["txt"] }]),
    { status: "cancelled" },
  );

  await assert.rejects(
    () => selectionClient.writeSelectedFileText("AbCdEfGhIjKlMnOpQrStUv", "selected text"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "file.write_text",
  );

  const writeClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["file.write_text"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => writeClient.writeSelectedFileText("AbCdEfGhIjKlMnOpQrStUv", "selected text"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "file.unavailable",
  );
  await assert.rejects(
    () => writeClient.writeSelectedFileText("C:/private.txt", "selected text"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
  await assert.rejects(
    () => writeClient.writeSelectedFileText("AbCdEfGhIjKlMnOpQrStUv", "x".repeat(MAX_FILE_TEXT_WRITE_BYTES + 1)),
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

test("a window title is a bounded proposal behind its own grant", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["window.title"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  // Acceptance only. The composed caption is deliberately not returned: it
  // would hand the application the host's framing format to probe, and it
  // already knows both halves of what it would be told.
  assert.deepEqual(await client.setWindowTitle("Quarterly Report.pdf"), {
    status: "applied",
  });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.setWindowTitle("Quarterly Report.pdf"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.title",
  );
});

test("a window title cannot be aimed at a window or split across lines", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.title"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  const rejected: readonly string[] = [
    "",
    // A title is a label rendered on one line. A newline or a carriage return
    // could split one window's title into what reads as two, or push the
    // visible text away from the host's application-name suffix.
    "Report\nWindows Security",
    "Report\rWindows Security",
    // Written as an escape rather than a literal control byte, so the intent
    // survives a copy and is visible to a reader.
    "Report\u001B[2K",
    "Report\u0000",
    "t".repeat(97),
  ];

  for (const title of rejected) {
    await assert.rejects(
      () => client.setWindowTitle(title),
      (error: unknown) =>
        error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
      `${JSON.stringify(title)} was accepted`,
    );
  }

  // A refusal must not repeat the text it rejected.
  await assert.rejects(
    () => client.setWindowTitle("Sensitive\rMarkerZQX"),
    (error: unknown) =>
      error instanceof PlatformRemoteError && !error.message.includes("MarkerZQX"),
  );

  // No target may ride along. The absence of a way to name a window is what
  // makes this capability impossible to aim at somebody else's.
  const transport = host.createTransport();
  for (const payload of [
    { title: "Report", target: "other-window" },
    { title: "Report", windowId: 2 },
    { caption: "Report" },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 14 },
      kind: "request",
      requestId: `aimed-${JSON.stringify(payload)}`,
      operation: "window.title.set",
      // Deliberately off-contract: this is the shape a client would send if it
      // believed it could name a window, and the host must refuse it.
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  // Accepted exactly at the bound.
  assert.deepEqual(await client.setWindowTitle("t".repeat(96)), { status: "applied" });
});

test("a session window state is closed, separately granted, and untargetable", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.state"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  for (const state of ["minimized", "maximized", "restored"] as const) {
    assert.deepEqual(await client.setWindowState(state), { status: "applied" });
  }

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.setWindowState("minimized"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.state",
  );

  // No target, geometry, focus request, or native state can ride along. The
  // mock shares the exact public contract, so this catches a drift between
  // SDK-facing validation and the host's protocol expectation.
  const transport = host.createTransport();
  for (const payload of [
    { state: "fullscreen" },
    { state: "minimized", target: "other-window" },
    { state: "restored", bounds: { width: 1 } },
    { state: "maximized", focus: true },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 16 },
      kind: "request",
      requestId: `window-state-${JSON.stringify(payload)}`,
      operation: "window.state.set",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 15 },
    kind: "request",
    requestId: "window-state-before-protocol-1.16",
    operation: "window.state.set",
    payload: { state: "restored" },
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});

test("a session window focus request is empty, separately granted, and untargetable", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.focus"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.requestWindowFocus(), { status: "requested" });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.requestWindowFocus(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.focus",
  );

  // A request that could name a target, change retry policy, or carry input
  // would be desktop-control authority. The only valid payload is exactly {}.
  const transport = host.createTransport();
  for (const payload of [
    null,
    { target: "other-window" },
    { handle: 7 },
    { retry: true },
    { input: "click" },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 20 },
      kind: "request",
      requestId: `window-focus-${JSON.stringify(payload)}`,
      operation: "window.focus.request",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 19 },
    kind: "request",
    requestId: "window-focus-before-protocol-1.20",
    operation: "window.focus.request",
    payload: {},
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});

test("a native session menu is complete, separately granted, and untargetable", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["menu.write"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());
  const menu = [
    {
      label: "File",
      items: [{ id: "document.new", label: "New document", enabled: true }],
    },
  ] as const;

  assert.deepEqual(await client.replaceMenu(menu), { revision: "1" });
  assert.deepEqual(await client.replaceMenu(menu), { revision: "2" });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.replaceMenu(menu),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "menu.write",
  );

  const transport = host.createTransport();
  for (const payload of [
    { menus: [] },
    {
      menus: [
        {
          label: "File",
          items: [{ id: "document.new", label: "New document", enabled: true, nativeId: 1 }],
        },
      ],
    },
    {
      menus: [
        {
          label: "File",
          items: [
            { id: "document.new", label: "New document", enabled: true },
            { id: "document.new", label: "Duplicate", enabled: true },
          ],
        },
      ],
    },
    {
      menus: [
        {
          label: "File",
          items: [{ id: "system command", label: "New document", enabled: true }],
        },
      ],
    },
    {
      menus: [
        {
          label: "\uD800",
          items: [{ id: "document.new", label: "New document", enabled: true }],
        },
      ],
    },
  ]) {
    const response = await transport.send({
      protocolVersion: PROTOCOL_VERSION,
      kind: "request",
      requestId: `menu-${JSON.stringify(payload)}`,
      operation: "menu.replace",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const items = Array.from({ length: 16 }, (_, item) => ({
    id: `command${item}`,
    label: "x".repeat(96),
    enabled: true,
  }));
  const oversizedPayload = {
    menus: Array.from({ length: 8 }, (_, menuIndex) => ({
      label: `Menu${menuIndex}`,
      items: items.map((item) => ({ ...item, id: `${item.id}-${menuIndex}` })),
    })),
  };
  assert.ok(JSON.stringify(oversizedPayload).length > MAX_MENU_REPLACE_REQUEST_BYTES);
  const oversized = await transport.send({
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId: "menu-oversized",
    operation: "menu.replace",
    payload: oversizedPayload as never,
  });
  assert.equal(
    oversized.status === "failure" ? oversized.error.code : undefined,
    "request.payload_invalid",
  );

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 17 },
    kind: "request",
    requestId: "menu-before-protocol-1.18",
    operation: "menu.replace",
    payload: { menus: menu } as never,
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});

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

class SequenceRequestIds implements RequestIdFactory {
  private current = 0;

  next(): string {
    this.current += 1;
    return `request-${this.current}`;
  }
}
