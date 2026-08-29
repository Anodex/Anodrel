import { readFileSync } from "node:fs";

import { PlatformClient, PlatformRemoteError } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

import { pollSchedule } from "./poll-schedule.js";
import {
  FIELD_SESSION_ACTION,
  FIELD_SESSION_DOCUMENT,
  FIELD_SESSION_IDS,
  LIVE_STATUS_ACTION,
  LIVE_STATUS_ASSERTIVE_DOCUMENT,
  LIVE_STATUS_INITIAL_DOCUMENT,
  LIVE_STATUS_POLITE_DOCUMENT,
  MENU_SESSION_ACTION,
  MENU_SESSION_DOCUMENT,
  fieldEchoDocument,
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

/**
 * How long the field diagnostic leaves its echoed values on screen.
 *
 * Long enough to read, short enough that an unattended run still finishes on
 * its own.
 */
const ECHO_VISIBLE_MILLISECONDS = 8_000;
/** Time to hear each manual live-status update before the diagnostic closes. */
const LIVE_STATUS_VISIBLE_MILLISECONDS = 3_000;

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
    const fieldDiagnostic = process.argv.includes("--request-field-read");
    const menuDiagnostic = process.argv.includes("--request-native-menu");
    const liveStatusDiagnostic = process.argv.includes("--request-live-status");
    let update;
    if (scrollDiagnostic) {
      update = await client.replaceUiDocumentV2(SCROLL_SESSION_DOCUMENT);
    } else if (liveStatusDiagnostic) {
      update = await client.replaceUiDocumentV3(LIVE_STATUS_INITIAL_DOCUMENT);
    } else if (fieldDiagnostic) {
      update = await client.replaceUiDocument(FIELD_SESSION_DOCUMENT);
    } else if (menuDiagnostic) {
      update = await client.replaceUiDocument(MENU_SESSION_DOCUMENT);
    } else {
      update = await client.replaceUiDocument(STANDARD_SESSION_DOCUMENT);
    }
    if (update.revision !== "1") {
      return 16;
    }

    let menuRevision: string | undefined;
    if (menuDiagnostic) {
      const replaced = await client.replaceMenu([
        {
          // Literal ampersands verify that the Windows adapter escapes User32
          // mnemonic markers rather than letting application text claim Alt
          // shortcuts.
          label: "File & actions",
          items: [
            {
              id: MENU_SESSION_ACTION,
              label: "Complete & close",
              enabled: true,
              shortcut: "Ctrl+Shift+M",
            },
          ],
        },
      ]);
      if (replaced.revision !== "1") {
        return 30;
      }
      menuRevision = replaced.revision;
    }

    if (fieldDiagnostic) {
      // Read before anyone has had a chance to type. Whatever comes back is
      // what the document set, which is the point: an application starts a
      // field and learns nothing more until it asks again.
      const before = await client.readUiFields();
      const initial = new Map(before.fields.map((entry) => [entry.id, entry.value]));
      if (initial.get(FIELD_SESSION_IDS[0]) !== "" || initial.get(FIELD_SESSION_IDS[1]) !== "edit me") {
        return 26;
      }
      console.log("Before typing, the host reports what this application set.");
    }

    if (process.argv.includes("--request-open-file")) {
      const dialog = await client.openFileDialog([
        { label: "Documents", extensions: ["txt", "json", "md"] },
      ]);
      if (dialog.status !== "selected" && dialog.status !== "cancelled") {
        return 18;
      }
    }

    if (process.argv.includes("--request-open-folder")) {
      const dialog = await client.openFolderDialog();
      if (dialog.status !== "selected" && dialog.status !== "cancelled") {
        return 35;
      }
      if (dialog.status === "selected" && dialog.path.length === 0) {
        return 35;
      }
    }

    if (process.argv.includes("--request-selected-folder-entries")) {
      const selection = await client.openFolderDialogWithReference();
      if (selection.status === "selected") {
        const entries = await client.readSelectedFolderEntries(selection.folderReference);
        if (entries.status !== "entries" || entries.entries.length > 32) {
          return 36;
        }
        // Deliberately prints only the bounded snapshot count. A diagnostic
        // never needs to echo a selected path, opaque reference, or child name.
        console.log(`Anodrel read ${entries.entries.length} direct folder entries.`);
      } else if (selection.status !== "cancelled") {
        return 36;
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

    if (process.argv.includes("--request-save-file-text")) {
      const selection = await client.saveFileDialogWithReference([
        { label: "Text", extensions: ["txt", "json", "md"] },
      ]);
      if (selection.status === "selected") {
        const written = await client.writeSelectedFileText(
          selection.saveReference,
          "Written by the Anodrel native file-write diagnostic.\n",
        );
        if (written.status !== "written") {
          return 29;
        }
      } else if (selection.status !== "cancelled") {
        return 29;
      }
    }

    if (process.argv.includes("--request-save-file-binary")) {
      const selection = await client.saveFileDialogWithReference([
        { label: "Binary export", extensions: ["bin"] },
      ]);
      if (selection.status === "selected") {
        const written = await client.writeSelectedFileBinary(
          selection.saveReference,
          new Uint8Array([0x41, 0x6e, 0x6f, 0x64, 0x72, 0x65, 0x6c, 0x00, 0xff]),
        );
        if (written.status !== "written") {
          return 31;
        }
      } else if (selection.status !== "cancelled") {
        return 31;
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

    if (process.argv.includes("--request-window-state")) {
      // A closed sequence exercises every public state request. The short
      // pauses make the native transitions visible to an operator; the client
      // still learns only that the UI thread accepted each command.
      for (const state of ["minimized", "maximized", "restored"] as const) {
        const applied = await client.setWindowState(state);
        if (applied.status !== "applied") {
          return 28;
        }
        await delay(650);
      }
    }

    if (process.argv.includes("--request-window-state-read")) {
      // Reading starts with the host's actual initial state, then confirms the
      // snapshots after this session's own requests. No native window detail
      // or promise of a durable state crosses the protocol boundary.
      if ((await client.getWindowState()).state !== "restored") {
        return 37;
      }
      for (const state of ["maximized", "restored"] as const) {
        if ((await client.setWindowState(state)).status !== "applied") {
          return 37;
        }
        if ((await client.getWindowState()).state !== state) {
          return 37;
        }
        await delay(650);
      }
    }

    if (process.argv.includes("--request-window-state-changes")) {
      // The initial visible size only establishes the host baseline. Later
      // requests cause ordinary native size messages; each pull consumes the
      // one coalesced current change without receiving timing or a listener.
      if ((await client.readWindowStateChanges()).state !== null) {
        return 38;
      }
      for (const state of ["maximized", "restored"] as const) {
        if ((await client.setWindowState(state)).status !== "applied") {
          return 38;
        }
        await delay(650);
        if ((await client.readWindowStateChanges()).state !== state) {
          return 38;
        }
      }
    }

    if (process.argv.includes("--request-window-focus")) {
      // Give an operator a short window to bring another application forward.
      // The result still says only that Anodrel asked Windows; Windows may
      // foreground the session window or flash its taskbar under its own rules.
      await delay(1_500);
      const requested = await client.requestWindowFocus();
      if (requested.status !== "requested") {
        return 29;
      }
    }

    if (process.argv.includes("--request-window-fullscreen")) {
      // The host picks the monitor for its own known window and keeps all
      // placement facts private. Pauses make both transitions observable, but
      // this client learns only that each closed mode was accepted.
      const entered = await client.setWindowFullscreen("fullscreen");
      if (entered.status !== "applied") {
        return 30;
      }
      await delay(900);
      const restored = await client.setWindowFullscreen("windowed");
      if (restored.status !== "applied") {
        return 30;
      }
    }

    if (process.argv.includes("--request-window-size")) {
      // The request names only a bounded logical client area. It cannot move
      // the window or choose a display; the host derives its own native frame.
      const applied = await client.setWindowSize(800, 520);
      if (applied.status !== "applied") {
        return 32;
      }
    }

    if (process.argv.includes("--request-window-size-while-fullscreen")) {
      const entered = await client.setWindowFullscreen("fullscreen");
      if (entered.status !== "applied") {
        return 33;
      }
      await delay(650);
      try {
        await client.setWindowSize(800, 520);
        return 33;
      } catch (error) {
        if (!(error instanceof PlatformRemoteError) || error.code !== "window.unavailable") {
          return 33;
        }
      }
      const restored = await client.setWindowFullscreen("windowed");
      if (restored.status !== "applied") {
        return 33;
      }
    }

    if (process.argv.includes("--request-credentials")) {
      const result = await runCredentialDiagnostic(client);
      if (result !== 0) {
        return result;
      }
    }

    if (process.argv.includes("--wait-for-ui-event")) {
      if (menuDiagnostic) {
        const eventResult = await waitForMenuAction(
          client,
          menuRevision ?? "",
          MENU_SESSION_ACTION,
        );
        if (eventResult !== 0) {
          return eventResult;
        }
        const close = await client.closeSession();
        return close.status === "accepted" ? 0 : 17;
      }
      let expectedAction = STANDARD_SESSION_ACTION;
      if (scrollDiagnostic) {
        expectedAction = SCROLL_SESSION_ACTION;
      } else if (fieldDiagnostic) {
        expectedAction = FIELD_SESSION_ACTION;
      } else if (liveStatusDiagnostic) {
        expectedAction = LIVE_STATUS_ACTION;
      }
      const eventResult = await waitForSampleAction(client, update.revision, expectedAction);
      if (eventResult !== 0) {
        return eventResult;
      }

      if (fieldDiagnostic) {
        // Only now, after a deliberate action, does this application learn
        // anything about what was typed. Everything between the two reads
        // happened without it.
        const after = await client.readUiFields();
        // Published back into the window rather than logged: the host
        // suppresses this child's console output on purpose, so a printed
        // value would be invisible. This also proves the text really reached
        // the application, on the surface the person is already looking at.
        await client.replaceUiDocument(fieldEchoDocument(after.fields));
        await new Promise((resolve) => setTimeout(resolve, ECHO_VISIBLE_MILLISECONDS));
      }

      if (liveStatusDiagnostic) {
        // Each replacement carries ordinary visible text. The application gets
        // only the accepted revision; it cannot inspect a listener or learn
        // whether Windows announced either value.
        const polite = await client.replaceUiDocumentV3(LIVE_STATUS_POLITE_DOCUMENT);
        if (polite.revision !== "2") {
          return 34;
        }
        await delay(LIVE_STATUS_VISIBLE_MILLISECONDS);
        const assertive = await client.replaceUiDocumentV3(LIVE_STATUS_ASSERTIVE_DOCUMENT);
        if (assertive.revision !== "3") {
          return 34;
        }
        await delay(LIVE_STATUS_VISIBLE_MILLISECONDS);
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

/** Waits specifically for the direct host-owned menu event shape. */
async function waitForMenuAction(
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
      return event.eventName === "menu.action.invoked" &&
        event.source === "native.menu" &&
        event.schemaVersion.major === 1 &&
        event.schemaVersion.minor === 18 &&
        event.payload.menuRevision === revision &&
        event.payload.action === expectedAction
        ? 0
        : 17;
    }
    await delay(interval);
  }

  return 17;
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
