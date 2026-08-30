import {
  assert,
  MAX_CONTEXT_MENU_REPLACE_REQUEST_BYTES,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  PROTOCOL_VERSION,
  SequenceRequestIds,
  test,
} from "./support.js";

test("a native context menu is complete, separately granted, and placement-free", async () => {
  const items = [
    { id: "document.rename", label: "Rename", enabled: true },
    { id: "document.archive", label: "Archive", enabled: false },
  ] as const;
  const client = new PlatformClient(
    new MockHost({
      applicationId: "test.application",
      grantedCapabilities: ["menu.context.write"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await client.replaceContextMenu(items), { revision: "1" });
  assert.deepEqual(await client.replaceContextMenu(items), { revision: "2" });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.replaceContextMenu(items),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "menu.context.write",
  );

  const transport = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["menu.context.write"],
  }).createTransport();
  for (const [index, payload] of [
    { items: [] },
    { items: [{ id: "document.rename", label: "Rename", enabled: true, shortcut: "Ctrl+R" }] },
    { items: [{ id: "document.rename", label: "Rename", enabled: true, x: 12 }] },
    { items: [{ id: "document.rename", label: "Rename", enabled: true, selection: "text" }] },
    {
      items: [
        { id: "document.rename", label: "Rename", enabled: true },
        { id: "document.rename", label: "Rename again", enabled: true },
      ],
    },
    { items: [{ id: "document rename", label: "Rename", enabled: true }] },
    { items: [{ id: "document.rename", label: "Rename\nnow", enabled: true }] },
    {
      items: [{
        id: "document.rename",
        label: "x".repeat(MAX_CONTEXT_MENU_REPLACE_REQUEST_BYTES),
        enabled: true,
      }],
    },
  ].entries()) {
    const response = await transport.send({
      protocolVersion: PROTOCOL_VERSION,
      kind: "request",
      requestId: `context-menu-invalid-${index}`,
      operation: "menu.context.replace",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 31 },
    kind: "request",
    requestId: "context-menu-before-protocol-1.32",
    operation: "menu.context.replace",
    payload: { items } as never,
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});
