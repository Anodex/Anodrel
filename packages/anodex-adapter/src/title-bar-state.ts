import type { WindowState } from "@anodrel/protocol";

/** The minimum public Anodrel boundary needed by an Anodex-style title bar. */
export interface AnodexWindowStateClient {
  getWindowState(): Promise<{ readonly state: WindowState }>;
  setWindowState(
    state: WindowState,
  ): Promise<{ readonly status: "applied" }>;
}

/** The complete presentation state one title bar needs to render its action. */
export interface AnodexTitleBarState {
  readonly isMaximized: boolean;
  readonly actionLabel: "Maximize" | "Restore";
}

/** Maps Anodrel's closed state to the Anodex title-bar control presentation. */
export function anodexTitleBarState(state: WindowState): AnodexTitleBarState {
  const isMaximized = state === "maximized";
  return {
    isMaximized,
    actionLabel: isMaximized ? "Restore" : "Maximize",
  };
}

/** Reads the one snapshot needed to render an Anodex-style title-bar action. */
export async function readAnodexTitleBarState(
  client: AnodexWindowStateClient,
): Promise<AnodexTitleBarState> {
  const { state } = await client.getWindowState();
  return anodexTitleBarState(state);
}

/**
 * Requests the opposite maximize/restore state, then takes a fresh snapshot.
 *
 * A state-set response proves acceptance only, so the refresh is required for
 * an honest glyph. The adapter never creates a listener or infers a later
 * native state from its request.
 */
export async function toggleAnodexTitleBarState(
  client: AnodexWindowStateClient,
): Promise<AnodexTitleBarState> {
  const { state } = await client.getWindowState();
  await client.setWindowState(state === "maximized" ? "restored" : "maximized");
  return readAnodexTitleBarState(client);
}
