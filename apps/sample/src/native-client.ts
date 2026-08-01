import { readFileSync } from "node:fs";

import { PlatformClient } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

const UI_EVENT_WAIT_ATTEMPTS = 100;
const UI_EVENT_WAIT_MILLISECONDS = 100;

process.on("uncaughtException", () => {
  process.exitCode = 14;
});
process.on("unhandledRejection", () => {
  process.exitCode = 15;
});

async function run(): Promise<number> {
  let invitation;
  try {
    invitation = decodeBootstrapInvitation(readFileSync(0));
  } catch {
    return 11;
  }

  let transport: WindowsNamedPipeTransport;
  try {
    transport = await WindowsNamedPipeTransport.connect(invitation);
  } catch {
    return 12;
  }
  try {
    const client = new PlatformClient(transport);
    const health = await client.getHealth();
    if (health.status !== "ready") {
      return 13;
    }
    const update = await client.replaceUiDocument(
      '{"format":"anodrel.ui.document.v1","root":{"id":"sample.session.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"sample.session.eyebrow","kind":"text","value":"AUTHENTICATED ANODREL SESSION","fontSize":14,"tone":"accent"},{"id":"sample.session.title","kind":"text","value":"Native document delivered","fontSize":28,"tone":"primary"},{"id":"sample.session.detail","kind":"text","value":"This view came through the private pipe and remains free of native action authority.","fontSize":16,"tone":"secondary"},{"id":"sample.session.action","kind":"action","label":"Visual-only semantic action","fontSize":16,"enabled":true,"tone":"accent"}]}}',
    );
    if (update.revision !== "1") {
      return 16;
    }

    if (process.argv.includes("--wait-for-ui-event")) {
      return waitForSampleAction(client, update.revision);
    }
    return 0;
  } catch {
    return 13;
  } finally {
    await transport.close();
  }
}

async function waitForSampleAction(client: PlatformClient, revision: string): Promise<number> {
  for (let attempt = 0; attempt < UI_EVENT_WAIT_ATTEMPTS; attempt += 1) {
    const result = await client.readUiEvents();
    if (result.dropped !== 0 || result.discarded !== 0) {
      return 17;
    }
    if (result.events.length > 0) {
      const event = result.events[0];
      if (event === undefined) {
        return 17;
      }
      return event.eventName === "ui.action.invoked" &&
        event.payload.revision === revision &&
        event.payload.action === "sample.session.action"
        ? 0
        : 17;
    }
    await delay(UI_EVENT_WAIT_MILLISECONDS);
  }

  return 17;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

// The bootstrap launcher directs child output to NUL. Never serialize an error
// here because it could contain sensitive host material; the numeric stage is
// sufficient for a developer to locate the failing boundary.
process.exitCode = await run();
