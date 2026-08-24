/** Per-transport state that models a bounded group of mock UI windows. */
export interface UiWindowState {
  nextSecondaryId: number;
  readonly revisions: Map<string, number>;
}

export function createUiWindowState(): UiWindowState {
  return {
    nextSecondaryId: 1,
    revisions: new Map([["main", 0]]),
  };
}

/** Per-transport revision state for the mock's complete application menu. */
export interface MenuState {
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
