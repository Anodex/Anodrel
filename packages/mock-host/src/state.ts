/** Per-transport state that models a bounded group of mock UI windows. */
export interface UiWindowState {
  nextSecondaryId: number;
  /** A deterministic stand-in for the host's session-owned state snapshot. */
  presentationState: "minimized" | "maximized" | "restored";
  /** The latest unread deterministic stand-in for a native state transition. */
  pendingPresentationState: "minimized" | "maximized" | "restored" | null;
  readonly revisions: Map<string, number>;
}

export function createUiWindowState(): UiWindowState {
  return {
    nextSecondaryId: 1,
    presentationState: "restored",
    pendingPresentationState: null,
    revisions: new Map([["main", 0]]),
  };
}

/** Per-transport revision state for the mock's complete application menu. */
export interface MenuState {
  revision: number;
}

/** Per-transport revision state for the mock's complete context menu. */
export interface ContextMenuState {
  revision: number;
}

export function secondaryDocumentOperationMinor(
  operation:
    | "window.open"
    | "window.open.v2"
    | "window.open.v3"
    | "ui.document.replace.window"
    | "ui.document.replace.window.v2"
    | "ui.document.replace.window.v3",
): number {
  switch (operation) {
    case "window.open":
    case "ui.document.replace.window":
      return 25;
    case "window.open.v3":
    case "ui.document.replace.window.v3":
      return 26;
    case "window.open.v2":
    case "ui.document.replace.window.v2":
      return 27;
  }
}

export function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (index + 1 >= value.length) {
        return false;
      }
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        return false;
      }
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return false;
    }
  }
  return true;
}

export function extractRequestId(value: unknown): string {
  if (
    typeof value === "object" &&
    value !== null &&
    "requestId" in value &&
    typeof value.requestId === "string" &&
    value.requestId.length > 0
  ) {
    return value.requestId;
  }

  return "invalid-request";
}
