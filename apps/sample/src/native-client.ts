import { readFileSync } from "node:fs";

import { PlatformClient } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

import {
  SCROLL_SESSION_ACTION,
  SCROLL_SESSION_DOCUMENT,
  STANDARD_SESSION_ACTION,
  STANDARD_SESSION_DOCUMENT,
} from "./session-documents.js";

// The native diagnostic is intentionally interactive. Two minutes leaves room
// to inspect, scroll, or dismiss one host-owned dialog without creating an
// unbounded background event read.
const UI_EVENT_WAIT_ATTEMPTS = 1_200;
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
    const scrollDiagnostic = process.argv.includes("--request-scroll-document");
    const update = scrollDiagnostic
      ? await client.replaceUiDocumentV2(SCROLL_SESSION_DOCUMENT)
      : await client.replaceUiDocument(STANDARD_SESSION_DOCUMENT);
    if (update.revision !== "1") {
      return 16;
    }

    if (process.argv.includes("--request-open-file")) {
      const dialog = await client.openFileDialog([
        { label: "Documents", extensions: ["txt", "json", "md"] },
      ]);
      if (dialog.status !== "selected" && dialog.status !== "cancelled") {
        return 18;
      }
    }

    if (process.argv.includes("--request-save-file")) {
      const dialog = await client.saveFileDialog([
        { label: "Documents", extensions: ["txt", "json", "md"] },
      ]);
      if (dialog.status !== "saved" && dialog.status !== "cancelled") {
        return 19;
      }
    }

    if (process.argv.includes("--request-selected-file-text")) {
      const selection = await client.openFileDialogWithReference([
        { label: "Text", extensions: ["txt", "json", "md"] },
      ]);
      if (selection.status === "selected") {
        const text = await client.readSelectedFileText(selection.selectionReference);
        if (text.status !== "text") {
          return 20;
        }
      } else if (selection.status !== "cancelled") {
        return 20;
      }
    }

    if (process.argv.includes("--request-storage-state")) {
      const replaced = await client.replaceStorageState("Anodrel storage diagnostic");
      if (replaced.status !== "replaced") {
        return 21;
      }
      const state = await client.readStorageState();
      if (state.status !== "snapshot" || state.snapshot !== "Anodrel storage diagnostic") {
        return 21;
      }
      const cleared = await client.clearStorageState();
      if (cleared.status !== "cleared") {
        return 21;
      }
    }

    if (process.argv.includes("--wait-for-ui-event")) {
      const eventResult = await waitForSampleAction(
        client,
        update.revision,
        scrollDiagnostic ? SCROLL_SESSION_ACTION : STANDARD_SESSION_ACTION,
      );
      if (eventResult !== 0) {
        return eventResult;
      }
      const close = await client.closeSession();
      return close.status === "accepted" ? 0 : 17;
    }
    return 0;
  } catch {
    return 13;
  } finally {
    await transport.close();
  }
}

async function waitForSampleAction(
  client: PlatformClient,
  revision: string,
  expectedAction: string,
): Promise<number> {
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
        event.payload.action === expectedAction
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
