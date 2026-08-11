import { readFileSync } from "node:fs";

import { PlatformClient } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

import { pollSchedule } from "./poll-schedule.js";
import {
  SCROLL_SESSION_ACTION,
  SCROLL_SESSION_DOCUMENT,
  STANDARD_SESSION_ACTION,
  STANDARD_SESSION_DOCUMENT,
} from "./session-documents.js";

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

    if (process.argv.includes("--request-diagnostics")) {
      const diagnostics = await client.readDiagnosticEntries();
      if (
        diagnostics.entries.length !== 2 ||
        diagnostics.entries[0]?.component !== "core" ||
        diagnostics.entries[1]?.component !== "transport"
      ) {
        return 22;
      }
    }

    if (process.argv.includes("--request-notification")) {
      // Acceptance means the host handed the values to the operating system.
      // There is deliberately nothing here that could report whether the user
      // saw, silenced, or dismissed it.
      const shown = await client.showNotification(
        "Anodrel notification diagnostic",
        "This came through the private pipe.\nActivate the window action to finish.",
      );
      if (shown.status !== "shown") {
        return 24;
      }
    }

    if (process.argv.includes("--request-window-title")) {
      // The host composes the caption: it appends the application's validated
      // display name, so this proposal cannot claim to be another application.
      // Deliberately proposes a name that would be a lie on its own.
      const applied = await client.setWindowTitle("Windows Security");
      if (applied.status !== "applied") {
        return 25;
      }
    }

    if (process.argv.includes("--request-credentials")) {
      const result = await runCredentialDiagnostic(client);
      if (result !== 0) {
        return result;
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

/**
 * Waits for the one semantic action the host renders for this diagnostic.
 *
 * The wait is paced by the shared backoff schedule, so an immediate click is
 * answered within a few tens of milliseconds while an open window costs far
 * fewer idle round trips than a fixed interval. Running out of schedule is the
 * timeout.
 */
async function waitForSampleAction(
  client: PlatformClient,
  revision: string,
  expectedAction: string,
): Promise<number> {
  for (const interval of pollSchedule()) {
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
    await delay(interval);
  }

  return 17;
}

async function runCredentialDiagnostic(client: PlatformClient): Promise<number> {
  const name = `sample-session-${process.pid}`;
  const secret = "00aaff";
  const written = await client.writeCredential(name, secret);
  if (written.status !== "written") {
    return 23;
  }

  let read;
  let deleted;
  try {
    read = await client.readCredential(name);
  } finally {
    deleted = await client.deleteCredential(name);
  }
  return read.status === "found" && read.secret === secret && deleted.status === "deleted"
    ? 0
    : 23;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

// The bootstrap launcher directs child output to NUL. Never serialize an error
// here because it could contain sensitive host material; the numeric stage is
// sufficient for a developer to locate the failing boundary.
process.exitCode = await run();
