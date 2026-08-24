import {
  assert,
  MAX_MENU_REPLACE_REQUEST_BYTES,
  MAX_WINDOW_CLIENT_HEIGHT,
  MAX_WINDOW_CLIENT_WIDTH,
  MIN_WINDOW_CLIENT_HEIGHT,
  MIN_WINDOW_CLIENT_WIDTH,
  MockHost,
  PlatformClient,
  PlatformRemoteError,
  PROTOCOL_VERSION,
  SequenceRequestIds,
  test,
} from "./support.js";

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

test("a session window fullscreen mode is closed, separately granted, and untargetable", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.fullscreen"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.setWindowFullscreen("fullscreen"), { status: "applied" });
  assert.deepEqual(await client.setWindowFullscreen("windowed"), { status: "applied" });

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.setWindowFullscreen("fullscreen"),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.fullscreen",
  );

  // A monitor, display mode, geometry, native style, or target would turn
  // reversible presentation into desktop-control authority.
  const transport = host.createTransport();
  for (const payload of [
    {},
    { mode: "exclusive" },
    { mode: "fullscreen", monitor: "other" },
    { mode: "windowed", bounds: { width: 1 } },
    { mode: true },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 21 },
      kind: "request",
      requestId: `window-fullscreen-${JSON.stringify(payload)}`,
      operation: "window.fullscreen.set",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 20 },
    kind: "request",
    requestId: "window-fullscreen-before-protocol-1.21",
    operation: "window.fullscreen.set",
    payload: { mode: "fullscreen" },
  });
  assert.equal(
    older.status === "failure" ? older.error.code : undefined,
    "operation.unsupported",
  );
});

test("a session window client size is bounded, separately granted, and unpositionable", async () => {
  const host = new MockHost({
    applicationId: "test.application",
    grantedCapabilities: ["window.size"],
  });
  const client = new PlatformClient(host.createTransport(), new SequenceRequestIds());

  assert.deepEqual(await client.setWindowSize(800, 600), { status: "applied" });
  assert.deepEqual(
    await client.setWindowSize(MAX_WINDOW_CLIENT_WIDTH, MAX_WINDOW_CLIENT_HEIGHT),
    { status: "applied" },
  );

  const ungranted = new PlatformClient(
    new MockHost({ applicationId: "test.application" }).createTransport(),
    new SequenceRequestIds(),
  );
  await assert.rejects(
    () => ungranted.setWindowSize(800, 600),
    (error: unknown) =>
      error instanceof PlatformRemoteError &&
      error.code === "capability.denied" &&
      error.details?.capability === "window.size",
  );

  // A position, target, display, native rectangle, fraction, or out-of-range
  // value would turn bounded client sizing into broader desktop authority.
  const transport = host.createTransport();
  for (const payload of [
    {},
    { width: MIN_WINDOW_CLIENT_WIDTH - 1, height: 600 },
    { width: 800, height: MIN_WINDOW_CLIENT_HEIGHT - 1 },
    { width: MAX_WINDOW_CLIENT_WIDTH + 1, height: 600 },
    { width: 800, height: MAX_WINDOW_CLIENT_HEIGHT + 1 },
    { width: 800.5, height: 600 },
    { width: 800, height: 600, x: 0 },
    { width: 800, height: 600, monitor: "other" },
  ]) {
    const response = await transport.send({
      protocolVersion: { major: 1, minor: 23 },
      kind: "request",
      requestId: `window-size-${JSON.stringify(payload)}`,
      operation: "window.size.set",
      payload: payload as never,
    });
    assert.equal(
      response.status === "failure" ? response.error.code : undefined,
      "request.payload_invalid",
      `${JSON.stringify(payload)} was accepted`,
    );
  }

  const older = await transport.send({
    protocolVersion: { major: 1, minor: 22 },
    kind: "request",
    requestId: "window-size-before-protocol-1.23",
    operation: "window.size.set",
    payload: { width: 800, height: 600 },
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
      items: [
        {
          id: "document.new",
          label: "New document",
          enabled: true,
          shortcut: "Ctrl+Shift+N",
        },
      ],
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
          items: [{ id: "document.new", label: "New document", enabled: true, shortcut: "Ctrl+n" }],
        },
      ],
    },
    {
      menus: [
        {
          label: "File",
          items: [
            { id: "document.primary", label: "Primary", enabled: true, shortcut: "Ctrl+N" },
            { id: "document.secondary", label: "Secondary", enabled: false, shortcut: "Ctrl+N" },
          ],
        },
      ],
    },
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

  const olderShortcut = await transport.send({
    protocolVersion: { major: 1, minor: 23 },
    kind: "request",
    requestId: "menu-shortcut-before-protocol-1.24",
    operation: "menu.replace",
    payload: {
      menus: [
        {
          label: "File",
          items: [{ id: "document.new", label: "New document", enabled: true, shortcut: "Ctrl+N" }],
        },
      ],
    } as never,
  });
  assert.equal(
    olderShortcut.status === "failure" ? olderShortcut.error.code : undefined,
    "request.payload_invalid",
  );

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
