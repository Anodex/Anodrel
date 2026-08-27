import {
  assert,
  MAX_FILE_BINARY_WRITE_BYTES,
  MAX_FILE_TEXT_WRITE_BYTES,
  MAX_NETWORK_FETCH_REQUEST_BYTES,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  PROTOCOL_VERSION,
  SequenceRequestIds,
  test,
} from "./support.js";

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

test("SDK and host keep folder selection separately granted and payload-free", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_folder"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.openFolderDialog(), { status: "cancelled" });

  const denied = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => denied.openFolderDialog(),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "dialog.open_folder",
  );
});

test("selected-folder entries need a separate grant and one exact reference", async () => {
  const selectionClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.open_folder"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  assert.deepEqual(await selectionClient.openFolderDialogWithReference(), { status: "cancelled" });

  await assert.rejects(
    () => selectionClient.readSelectedFolderEntries("AbCdEfGhIjKlMnOpQrStUv"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "folder.read_entries",
  );

  const entriesClient = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["folder.read_entries"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => entriesClient.readSelectedFolderEntries("AbCdEfGhIjKlMnOpQrStUv"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "folder.unavailable",
  );
  await assert.rejects(
    () => entriesClient.readSelectedFolderEntries("C:/private"),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
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

test("session-owned secondary windows stay bounded, opaque, and independently revised", async () => {
  const document =
    '{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Hello","fontSize":16,"tone":"primary"}}';
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.open", "window.close", "ui.document.write", "ui.events.read"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  const opened = await client.openWindow("Notes", document);
  assert.deepEqual(opened, { windowId: "window-1" });
  assert.deepEqual(await client.replaceUiDocumentInWindow(opened.windowId, document), {
    revision: "2",
  });
  assert.deepEqual(await client.replaceUiDocumentInWindow("main", document), { revision: "1" });
  assert.deepEqual(await client.readWindowUiEvents(), {
    events: [],
    dropped: 0,
    discarded: 0,
  });
  assert.deepEqual(await client.closeWindow(opened.windowId), { status: "requested" });

  await assert.rejects(
    () => client.replaceUiDocumentInWindow(opened.windowId, document),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "window.unavailable",
  );

  const ungrantedOpen = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["ui.document.write"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungrantedOpen.openWindow("Notes", document),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.open",
  );

  const noDocumentGrant = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["window.open"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => noDocumentGrant.openWindow("Notes", document),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "ui.document.write",
  );

  const transport = host.createTransport();
  for (const [operation, payload] of [
    ["window.open", { title: "Notes", document, width: 900 }],
    ["window.close", { windowId: "main" }],
    ["window.close", { windowId: "window-01" }],
    ["ui.document.replace.window", { windowId: "window-0", document }],
    ["ui.events.read.window", { windowId: "window-1" }],
  ] as const) {
    const response = await transport.send({
      protocolVersion: PROTOCOL_VERSION,
      kind: "request",
      requestId: `window-payload-${operation}-${JSON.stringify(payload)}`,
      operation,
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${operation} accepted ${JSON.stringify(payload)}`,
    );
  }

  const invalidTitle = await transport.send({
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId: "window-open-invalid-title",
    operation: "window.open",
    payload: { title: "Notes\nMore", document },
  });
  assert.equal(
    invalidTitle.status === "failure" ? invalidTitle.error.code : undefined,
    "window.title_invalid",
  );

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 24 },
    kind: "request",
    requestId: "window-open-before-protocol-1.25",
    operation: "window.open",
    payload: { title: "Notes", document },
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
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

test("binary output is canonical, bounded, and separately granted", async () => {
  const denied = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["dialog.save_file"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => denied.writeSelectedFileBinary("AbCdEfGhIjKlMnOpQrStUv", new Uint8Array([0, 1, 2, 255])),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "file.write_binary",
  );

  const granted = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["file.write_binary"],
    }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => granted.writeSelectedFileBinary("AbCdEfGhIjKlMnOpQrStUv", new Uint8Array([0, 1, 2, 255])),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "file.unavailable",
  );
  await assert.rejects(
    () => granted.writeSelectedFileBinary("C:/private.bin", new Uint8Array([0])),
    (error: unknown) => error instanceof PlatformRemoteError && error.code === "request.payload_invalid",
  );
  await assert.rejects(
    () => granted.writeSelectedFileBinary("AbCdEfGhIjKlMnOpQrStUv", new Uint8Array(MAX_FILE_BINARY_WRITE_BYTES + 1)),
    (error: unknown) => error instanceof RangeError,
  );

  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["file.write_binary"],
  });
  const malformed = await host.createTransport().send({
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId: "binary-noncanonical",
    operation: "file.write_binary",
    payload: { saveReference: "AbCdEfGhIjKlMnOpQrStUv", bytesBase64Url: "AB" },
  });
  assert.equal(malformed.status, "failure");
  if (malformed.status === "failure") {
    assert.equal(malformed.error.code, "request.payload_invalid");
  }

  const tooLarge = await host.createTransport().send({
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId: "binary-too-large",
    operation: "file.write_binary",
    payload: {
      saveReference: "AbCdEfGhIjKlMnOpQrStUv",
      bytesBase64Url: "AAAA".repeat(Math.floor(MAX_FILE_BINARY_WRITE_BYTES / 3) + 1),
    },
  });
  assert.equal(tooLarge.status, "failure");
  if (tooLarge.status === "failure") {
    assert.equal(tooLarge.error.code, "file.binary_too_large");
  }

  const older = await host.createTransport().send({
    protocolVersion: { major: 1, minor: 21 },
    kind: "request",
    requestId: "binary-before-protocol-1.22",
    operation: "file.write_binary",
    payload: { saveReference: "AbCdEfGhIjKlMnOpQrStUv", bytesBase64Url: "AA" },
  });
  assert.equal(older.status, "failure");
  if (older.status === "failure") {
    assert.equal(older.error.code, "operation.unsupported");
  }
});
