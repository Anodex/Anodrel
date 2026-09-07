import { readFileSync } from "node:fs";

import { PlatformClient } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

import {
  replaceRequestedMenu,
  runRequestedUiEventDiagnostic,
  verifyInitialFieldState,
} from "./native-client-events.js";
import {
  runCredentialDiagnostic,
  runRequestedPlatformServiceDiagnostics,
} from "./native-client-service-diagnostics.js";
import { runRequestedWindowDiagnostics } from "./native-client-window-diagnostics.js";
import {
  FIELD_SESSION_DOCUMENT,
  LIVE_STATUS_INITIAL_DOCUMENT,
  MENU_SESSION_DOCUMENT,
  SCROLL_SESSION_DOCUMENT,
  STANDARD_SESSION_DOCUMENT,
} from "./session-documents.js";

process.on("uncaughtException", () => {
  process.exitCode = 14;
});
process.on("unhandledRejection", () => {
  process.exitCode = 15;
});

/** Connects the fixed native diagnostic and coordinates its selected probes. */
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

    const arguments_ = process.argv;
    const scrollDiagnostic = arguments_.includes("--request-scroll-document");
    const fieldDiagnostic = arguments_.includes("--request-field-read");
    const menuDiagnostic = arguments_.includes("--request-native-menu");
    const liveStatusDiagnostic = arguments_.includes("--request-live-status");
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
      menuRevision = await replaceRequestedMenu(client);
      if (menuRevision === undefined) {
        return 30;
      }
    }

    if (fieldDiagnostic && !(await verifyInitialFieldState(client))) {
      return 26;
    }

    const serviceResult = await runRequestedPlatformServiceDiagnostics(client, arguments_);
    if (serviceResult !== 0) {
      return serviceResult;
    }

    const windowResult = await runRequestedWindowDiagnostics(client, arguments_);
    if (windowResult !== 0) {
      return windowResult;
    }

    if (arguments_.includes("--request-credentials")) {
      const credentialResult = await runCredentialDiagnostic(client);
      if (credentialResult !== 0) {
        return credentialResult;
      }
    }

    if (arguments_.includes("--wait-for-ui-event")) {
      return runRequestedUiEventDiagnostic(
        client,
        {
          fieldDiagnostic,
          liveStatusDiagnostic,
          menuDiagnostic,
          menuRevision,
          scrollDiagnostic,
        },
        update.revision,
      );
    }
    return 0;
  } catch {
    return 13;
  } finally {
    await transport.close();
  }
}

// The bootstrap launcher directs child output to NUL. Never serialize an error
// here because it could contain sensitive host material; the numeric stage is
// sufficient for a developer to locate the failing boundary.
process.exitCode = await run();
