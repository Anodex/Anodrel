import { PlatformClient, PlatformRemoteError } from "@anodrel/sdk";

/** Runs native window diagnostics requested by the development sample. */
export async function runRequestedWindowDiagnostics(
  client: PlatformClient,
  arguments_: readonly string[],
): Promise<number> {
  if (arguments_.includes("--request-window-title")) {
    // The host composes the caption: it appends the application's validated
    // display name, so this proposal cannot claim to be another application.
    // Deliberately proposes a name that would be a lie on its own.
    const applied = await client.setWindowTitle("Windows Security");
    if (applied.status !== "applied") {
      return 25;
    }
  }

  if (arguments_.includes("--request-window-state")) {
    // A closed sequence exercises every public state request. The short pauses
    // make the native transitions visible to an operator; the client still
    // learns only that the UI thread accepted each command.
    for (const state of ["minimized", "maximized", "restored"] as const) {
      const applied = await client.setWindowState(state);
      if (applied.status !== "applied") {
        return 28;
      }
      await delay(650);
    }
  }

  if (arguments_.includes("--request-window-state-read")) {
    // Reading starts with the host's actual initial state, then confirms the
    // snapshots after this session's own requests. No native window detail or
    // promise of a durable state crosses the protocol boundary.
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

  if (arguments_.includes("--request-window-state-changes")) {
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

  if (arguments_.includes("--request-window-focus")) {
    // Give an operator a short window to bring another application forward.
    // The result still says only that Anodrel asked Windows; Windows may
    // foreground the session window or flash its taskbar under its own rules.
    await delay(1_500);
    const requested = await client.requestWindowFocus();
    if (requested.status !== "requested") {
      return 29;
    }
  }

  if (arguments_.includes("--request-window-fullscreen")) {
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

  if (arguments_.includes("--request-window-size")) {
    // The request names only a bounded logical client area. It cannot move the
    // window or choose a display; the host derives its own native frame.
    const applied = await client.setWindowSize(800, 520);
    if (applied.status !== "applied") {
      return 32;
    }
  }

  if (arguments_.includes("--request-window-size-while-fullscreen")) {
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

  return 0;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
