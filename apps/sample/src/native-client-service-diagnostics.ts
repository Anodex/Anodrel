import { PlatformClient } from "@anodrel/sdk";

/**
 * Runs the bounded non-window service probes requested by the native sample.
 *
 * Each switch is intentionally a development-host diagnostic rather than an
 * application preference. Results never print selected paths, opaque handles,
 * or host failure details.
 */
export async function runRequestedPlatformServiceDiagnostics(
  client: PlatformClient,
  arguments_: readonly string[],
): Promise<number> {
  if (arguments_.includes("--request-open-file")) {
    const dialog = await client.openFileDialog([
      { label: "Documents", extensions: ["txt", "json", "md"] },
    ]);
    if (dialog.status !== "selected" && dialog.status !== "cancelled") {
      return 18;
    }
  }

  if (arguments_.includes("--request-open-folder")) {
    const dialog = await client.openFolderDialog();
    if (dialog.status !== "selected" && dialog.status !== "cancelled") {
      return 35;
    }
    if (dialog.status === "selected" && dialog.path.length === 0) {
      return 35;
    }
  }

  if (arguments_.includes("--request-selected-folder-entries")) {
    const selection = await client.openFolderDialogWithReference();
    if (selection.status === "selected") {
      const entries = await client.readSelectedFolderEntries(selection.folderReference);
      if (entries.status !== "entries" || entries.entries.length > 32) {
        return 36;
      }
      // Deliberately prints only the bounded snapshot count. A diagnostic never
      // needs to echo a selected path, opaque reference, or child name.
      console.log(`Anodrel read ${entries.entries.length} direct folder entries.`);
    } else if (selection.status !== "cancelled") {
      return 36;
    }
  }

  if (arguments_.includes("--request-save-file")) {
    const dialog = await client.saveFileDialog([
      { label: "Documents", extensions: ["txt", "json", "md"] },
    ]);
    if (dialog.status !== "saved" && dialog.status !== "cancelled") {
      return 19;
    }
  }

  if (arguments_.includes("--request-save-file-text")) {
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

  if (arguments_.includes("--request-save-file-binary")) {
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

  if (arguments_.includes("--request-selected-file-text")) {
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

  if (arguments_.includes("--request-storage-state")) {
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

  if (arguments_.includes("--request-diagnostics")) {
    const diagnostics = await client.readDiagnosticEntries();
    if (
      diagnostics.entries.length !== 2 ||
      diagnostics.entries[0]?.component !== "core" ||
      diagnostics.entries[1]?.component !== "transport"
    ) {
      return 22;
    }
  }

  if (arguments_.includes("--request-notification")) {
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

  return 0;
}

/** Writes, reads, and removes one session-scoped credential diagnostic. */
export async function runCredentialDiagnostic(client: PlatformClient): Promise<number> {
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
