import {
  readAnodexTitleBarState,
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
      grantedCapabilities: ["window.state", "window.state.read"],
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
});
