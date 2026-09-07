import { PlatformClient } from "@anodrel/sdk";

import { pollSchedule } from "./poll-schedule.js";
import {
  FIELD_SESSION_ACTION,
  FIELD_SESSION_IDS,
  fieldEchoDocument,
  LIVE_STATUS_ACTION,
  LIVE_STATUS_ASSERTIVE_DOCUMENT,
  LIVE_STATUS_POLITE_DOCUMENT,
  MENU_SESSION_ACTION,
  SCROLL_SESSION_ACTION,
  STANDARD_SESSION_ACTION,
} from "./session-documents.js";

/** How long the field diagnostic leaves its echoed values on screen. */
const ECHO_VISIBLE_MILLISECONDS = 8_000;
/** Time to hear each manual live-status update before the diagnostic closes. */
const LIVE_STATUS_VISIBLE_MILLISECONDS = 3_000;

/** Installs the one host-owned menu used by the menu diagnostic. */
export async function replaceRequestedMenu(client: PlatformClient): Promise<string | undefined> {
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
  return replaced.revision === "1" ? replaced.revision : undefined;
}

/** Confirms that field reads initially contain only document-supplied values. */
export async function verifyInitialFieldState(client: PlatformClient): Promise<boolean> {
  // Read before anyone has had a chance to type. Whatever comes back is what
  // the document set, which is the point: an application starts a field and
  // learns nothing more until it asks again.
  const before = await client.readUiFields();
  const initial = new Map(before.fields.map((entry) => [entry.id, entry.value]));
  if (initial.get(FIELD_SESSION_IDS[0]) !== "" || initial.get(FIELD_SESSION_IDS[1]) !== "edit me") {
    return false;
  }
  console.log("Before typing, the host reports what this application set.");
  return true;
}

/**
 * Waits for one requested semantic event, runs its follow-up diagnostic, and
 * closes the session. A caller invokes this only when the event switch exists.
 */
export async function runRequestedUiEventDiagnostic(
  client: PlatformClient,
  options: {
    readonly fieldDiagnostic: boolean;
    readonly liveStatusDiagnostic: boolean;
    readonly menuDiagnostic: boolean;
    readonly menuRevision: string | undefined;
    readonly scrollDiagnostic: boolean;
  },
  revision: string,
): Promise<number> {
  if (options.menuDiagnostic) {
    const eventResult = await waitForMenuAction(
      client,
      options.menuRevision ?? "",
      MENU_SESSION_ACTION,
    );
    if (eventResult !== 0) {
      return eventResult;
    }
    const close = await client.closeSession();
    return close.status === "accepted" ? 0 : 17;
  }

  let expectedAction = STANDARD_SESSION_ACTION;
  if (options.scrollDiagnostic) {
    expectedAction = SCROLL_SESSION_ACTION;
  } else if (options.fieldDiagnostic) {
    expectedAction = FIELD_SESSION_ACTION;
  } else if (options.liveStatusDiagnostic) {
    expectedAction = LIVE_STATUS_ACTION;
  }
  const eventResult = await waitForSampleAction(client, revision, expectedAction);
  if (eventResult !== 0) {
    return eventResult;
  }

  if (options.fieldDiagnostic) {
    // Only now, after a deliberate action, does this application learn anything
    // about what was typed. Everything between the two reads happened without
    // it.
    const after = await client.readUiFields();
    // Published back into the window rather than logged: the host suppresses
    // this child's console output on purpose, so a printed value would be
    // invisible. This also proves the text really reached the application, on
    // the surface the person is already looking at.
    await client.replaceUiDocument(fieldEchoDocument(after.fields));
    await delay(ECHO_VISIBLE_MILLISECONDS);
  }

  if (options.liveStatusDiagnostic) {
    // Each replacement carries ordinary visible text. The application gets only
    // the accepted revision; it cannot inspect a listener or learn whether
    // Windows announced either value.
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

/** Waits for the one semantic action the host renders for this diagnostic. */
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

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
