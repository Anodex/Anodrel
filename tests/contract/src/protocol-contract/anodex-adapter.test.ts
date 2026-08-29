import {
  readAnodexTitleBarChange,
  readAnodexTitleBarState,
  requestAnodexTitleBarClose,
  toggleAnodexTitleBarState,
} from "@anodrel/anodex-adapter";

import {
  assert,
  MockHost,
  PlatformClient,
  SequenceRequestIds,
  test,
} from "./support.js";

test("the Anodex adapter renders and refreshes only the closed title-bar state", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "org.anodex.title-bar-test",
      grantedCapabilities: ["window.state", "window.state.read", "window.state.observe"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await readAnodexTitleBarState(client), {
    isMaximized: false,
    actionLabel: "Maximize",
  });
  assert.deepEqual(await toggleAnodexTitleBarState(client), {
    isMaximized: true,
    actionLabel: "Restore",
  });
  assert.deepEqual(await toggleAnodexTitleBarState(client), {
    isMaximized: false,
    actionLabel: "Maximize",
  });
  assert.deepEqual(await readAnodexTitleBarChange(client), {
    isMaximized: false,
    actionLabel: "Maximize",
  });
  assert.equal(await readAnodexTitleBarChange(client), null);
});

test("the Anodex title-bar close requests only its accepted session end", async () => {
  const client = new PlatformClient(
    new MockHost({
      applicationId: "org.anodex.title-bar-close-test",
      grantedCapabilities: ["session.close"],
    }).createTransport(),
    new SequenceRequestIds(),
  );

  assert.deepEqual(await requestAnodexTitleBarClose(client), {
    status: "accepted",
  });
});
