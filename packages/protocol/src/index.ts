/**
 * Versioned, transport-neutral messages shared by Anodrel clients and hosts.
 * Values crossing the boundary must be JSON-compatible.
 */

import { PROTOCOL_VERSION } from "./constants.js";
import type { RequestEnvelope } from "./messages.js";
import type { UiInteractionEvent, WindowUiInteractionEvent } from "./validators.js";

export { encodeCanonicalBase64Url } from "./base64url.js";

export * from "./constants.js";
export * from "./messages.js";

export interface ProtocolVersion {
  readonly major: number;
  readonly minor: number;
}

/** Capabilities are granted by the host policy, never by rendered application content. */
export type Capability =
  | "diagnostics.read"
  | "ui.document.write"
  | "ui.events.read"
  | "session.close"
  | "clipboard.read"
  | "clipboard.write"
  | "external.open"
  | "network.fetch"
  | "dialog.open_file"
  | "dialog.open_folder"
  | "folder.read_entries"
  | "dialog.save_file"
  | "file.read_text"
  | "file.write_text"
  | "file.write_binary"
  | "storage.state.read"
  | "storage.state.replace"
  | "storage.state.clear"
  | "credential.read"
  | "credential.write"
  | "credential.delete"
  | "notification.show"
  | "window.title"
  | "ui.fields.read"
  | "window.state"
  | "window.state.read"
  | "window.state.observe"
  | "window.focus"
  | "window.fullscreen"
  | "window.size"
  | "window.open"
  | "window.close"
  | "menu.write";

export type EmptyPayload = Record<string, never>;

/** Conservative classification for one direct selected-folder entry. */
export type FolderEntryKind = "file" | "directory" | "other";

/** One direct child exposed by a consumed selected-folder reference. */
export interface FolderEntry {
  readonly name: string;
  readonly kind: FolderEntryKind;
}

/** The complete set of presentation states an application may request or observe. */
export type WindowState = "minimized" | "maximized" | "restored";

/** The only reversible fullscreen modes an application may request. */
export type WindowFullscreenMode = "fullscreen" | "windowed";

/**
 * An opaque identity for one view in the current authenticated UI session.
 *
 * `main` names the session's primary view. The host issues secondary values as
 * canonical `window-<n>` strings and never treats either form as a native
 * handle, a global name, or a cross-session lookup key.
 */
export type SessionWindowId = "main" | SecondarySessionWindowId;

/** An opaque secondary view identity returned only by `window.open`. */
export type SecondarySessionWindowId = `window-${number}`;

/** One ASCII key permitted in a canonical local native-menu shortcut. */
export type NativeMenuShortcutKey =
  | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L"
  | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X"
  | "Y" | "Z" | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";

/** A canonical local shortcut for one semantic native-menu command. */
export type NativeMenuShortcut =
  | `Ctrl+${NativeMenuShortcutKey}`
  | `Ctrl+Shift+${NativeMenuShortcutKey}`;

/** One enabled or disabled semantic command in a native session menu. */
export interface NativeMenuItem {
  readonly id: string;
  readonly label: string;
  readonly enabled: boolean;
  /** Optional Protocol 1.24 local semantic shortcut. */
  readonly shortcut?: NativeMenuShortcut;
}

/** One top-level native session menu with its complete ordered item set. */
export interface NativeSessionMenu {
  readonly label: string;
  readonly items: readonly NativeMenuItem[];
}

export interface PlatformOperationMap {
  "platform.ping": {
    readonly payload: { readonly sentAt: string };
    readonly result: { readonly receivedAt: string; readonly hostName: string };
  };
  "platform.capabilities": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly applicationId: string;
      readonly grantedCapabilities: readonly Capability[];
    };
  };
  "platform.health": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly status: "ready";
      readonly hostName: string;
      readonly protocolVersion: ProtocolVersion;
    };
  };
  "diagnostics.entries.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly entries: readonly {
        readonly sequence: string;
        readonly level: "info";
        readonly component: string;
        readonly event: string;
      }[];
    };
  };
  "credential.read": {
    readonly payload: { readonly name: string };
    readonly result:
      | { readonly status: "found"; readonly secret: string }
      | { readonly status: "not_found" };
  };
  "credential.write": {
    readonly payload: { readonly name: string; readonly secret: string };
    readonly result: { readonly status: "written" };
  };
  "credential.delete": {
    readonly payload: { readonly name: string };
    readonly result: { readonly status: "deleted" } | { readonly status: "not_found" };
  };
  /**
   * Shows one bounded notification.
   *
   * The result reports only that the host handed the values over. It never
   * describes what the user experienced: whether notifications are silenced,
   * a focus mode is active, or this application is muted is not observable.
   */
  "notification.show": {
    readonly payload: { readonly title: string; readonly body: string };
    readonly result: { readonly status: "shown" };
  };
  /**
   * Proposes the title of this session's own window.
   *
   * A proposal, not an assignment. The host validates it and composes the
   * displayed caption with an application-name suffix that the proposal cannot
   * suppress or forge, so a title can say what is being shown and never change
   * what the application is.
   *
   * There is no window target: the host resolves the window from the
   * authenticated session. The result reports acceptance only — the composed
   * caption is deliberately not returned.
   */
  "window.title.set": {
    readonly payload: { readonly title: string };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Requests one standard presentation state for this session's own window.
   *
   * The host resolves the window from the authenticated session. There is no
   * target, native handle, geometry, or change event; success means only that
   * the host UI thread accepted the closed action.
   */
  "window.state.set": {
    readonly payload: { readonly state: WindowState };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Takes one immediate snapshot of the requesting session's own standard
   * window state. The distinct `window.state.read` grant is required.
   *
   * There is no target, handle, geometry, monitor, focus, fullscreen state,
   * timestamp, subscription, or change event. The returned value can be stale
   * as soon as it is received; see `docs/WINDOW_STATE_OBSERVATION.md`.
   */
  "window.state.get": {
    readonly payload: EmptyPayload;
    readonly result: { readonly state: WindowState };
  };
  /**
   * Consumes the latest coalesced native presentation change for this
   * session's own window. The distinct `window.state.observe` grant is
   * required.
   *
   * There is no target, native handle, timestamp, sequence, history, wait,
   * callback, or subscription. `null` says only that no unread change is
   * retained; see `docs/WINDOW_STATE_CHANGES.md`.
   */
  "window.state.changes.read": {
    readonly payload: EmptyPayload;
    readonly result: { readonly state: WindowState | null };
  };
  /**
   * Asks Windows to foreground this session's one host-owned window.
   *
   * It has no target, native handle, retry policy, focus readback, or event.
   * Windows decides whether the request is accepted; success reports only that
   * Windows accepted it, not what a person saw or used afterward.
   */
  "window.focus.request": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "requested" };
  };
  /**
   * Chooses reversible borderless fullscreen for this session's own window.
   *
   * The host retains native style and placement facts privately. There is no
   * target, monitor, geometry, display-mode control, state readback, or event;
   * success means only that the host UI thread accepted the closed action.
   */
  "window.fullscreen.set": {
    readonly payload: { readonly mode: WindowFullscreenMode };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Requests a bounded logical client size for this session's own window.
   *
   * The host derives the native framed rectangle at its current DPI. There is
   * no target, position, monitor, DPI, bounds readback, or geometry event;
   * success means only that the host UI thread accepted the request.
   */
  "window.size.set": {
    readonly payload: { readonly width: number; readonly height: number };
    readonly result: { readonly status: "applied" };
  };
  /**
   * Opens one independently revised secondary view in this session.
   *
   * The host chooses all native presentation details. The returned ID is
   * opaque and session-scoped; it cannot be converted to a handle, a desktop
   * position, or a way to enumerate other views.
   */
  "window.open": {
    readonly payload: { readonly title: string; readonly document: string };
    readonly result: { readonly windowId: SecondarySessionWindowId };
  };
  /** Opens one exact v2 scroll document in a bounded secondary session view. */
  "window.open.v2": {
    readonly payload: { readonly title: string; readonly document: string };
    readonly result: { readonly windowId: SecondarySessionWindowId };
  };
  /** Opens one exact v3 document in a bounded secondary session view. */
  "window.open.v3": {
    readonly payload: { readonly title: string; readonly document: string };
    readonly result: { readonly windowId: SecondarySessionWindowId };
  };
  /** Requests a close for one previously issued secondary view. */
  "window.close": {
    readonly payload: { readonly windowId: SecondarySessionWindowId };
    readonly result: { readonly status: "requested" };
  };
  /**
   * Replaces this authenticated session's complete native menu model.
   *
   * There is no native command number, accelerator, target, callback, or
   * handle. A successful revision is host-owned and opaque to the SDK.
   */
  "menu.replace": {
    readonly payload: { readonly menus: readonly NativeSessionMenu[] };
    readonly result: { readonly revision: string };
  };
  /**
   * Reads every field value on this session's own current surface.
   *
   * A snapshot, not a stream. The payload is empty and there is no selector:
   * a caller able to narrow a read to one field could repeat it until what
   * someone was typing had been reconstructed. Returning the whole surface
   * makes every read cost the same, so reading often gains nothing.
   *
   * The result carries values only — no caret, selection, timestamp, or
   * edited flag, because those describe the typing rather than the value.
   */
  "ui.fields.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly fields: ReadonlyArray<{ readonly id: string; readonly value: string }>;
    };
  };
  "ui.document.replace": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.document.replace.v2": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.document.replace.v3": {
    readonly payload: { readonly document: string };
    readonly result: { readonly revision: string };
  };
  /** Replaces the strict v1 document of one known session view. */
  "ui.document.replace.window": {
    readonly payload: { readonly windowId: SessionWindowId; readonly document: string };
    readonly result: { readonly revision: string };
  };
  /** Replaces one known session view with an exact v2 scroll document. */
  "ui.document.replace.window.v2": {
    readonly payload: { readonly windowId: SessionWindowId; readonly document: string };
    readonly result: { readonly revision: string };
  };
  /** Replaces one known session view with an exact v3 document. */
  "ui.document.replace.window.v3": {
    readonly payload: { readonly windowId: SessionWindowId; readonly document: string };
    readonly result: { readonly revision: string };
  };
  "ui.events.read": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly events: readonly UiInteractionEvent[];
      readonly dropped: number;
      readonly discarded: number;
    };
  };
  /** Reads bounded semantic events from each current session view. */
  "ui.events.read.window": {
    readonly payload: EmptyPayload;
    readonly result: {
      readonly events: readonly WindowUiInteractionEvent[];
      readonly dropped: number;
      readonly discarded: number;
    };
  };
  "session.close": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "accepted" };
  };
  "clipboard.read": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "text"; readonly text: string }
      | { readonly status: "no_text" };
  };
  "clipboard.write": {
    readonly payload: { readonly text: string };
    readonly result: { readonly status: "written" };
  };
  "external.open": {
    readonly payload: { readonly url: string };
    readonly result: { readonly status: "opened" };
  };
  /**
   * Fetches one bounded UTF-8 response from a host-authorized HTTPS origin.
   *
   * There is deliberately no method, body, header, cookie, credential,
   * redirect, proxy, timeout, client-certificate, callback, or native-handle
   * field. A non-2xx status remains a successful protocol result when its
   * bounded text body is representable.
   */
  "network.fetch_text": {
    readonly payload: { readonly url: string };
    readonly result: { readonly statusCode: number; readonly text: string };
  };
  "dialog.open_file": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string }
      | { readonly status: "cancelled" };
  };
  /** Selects one user-chosen filesystem folder without granting later access. */
  "dialog.open_folder": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "selected"; readonly path: string }
      | { readonly status: "cancelled" };
  };
  /** Selects one folder while retaining a one-use folder-entry reference. */
  "dialog.open_folder.v2": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "selected"; readonly path: string; readonly folderReference: string }
      | { readonly status: "cancelled" };
  };
  /** Returns one bounded direct-entry snapshot through a one-use folder reference. */
  "folder.read_entries": {
    readonly payload: { readonly folderReference: string };
    readonly result: {
      readonly status: "entries";
      readonly entries: readonly FolderEntry[];
      readonly complete: boolean;
    };
  };
  "dialog.save_file": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "saved"; readonly path: string }
      | { readonly status: "cancelled" };
  };
  "dialog.open_file.v2": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string; readonly selectionReference: string }
      | { readonly status: "cancelled" };
  };
  "file.read_text": {
    readonly payload: { readonly selectionReference: string };
    readonly result: { readonly status: "text"; readonly text: string };
  };
  "dialog.save_file.v2": {
    readonly payload: {
      readonly filters: readonly { readonly label: string; readonly extensions: readonly string[] }[];
    };
    readonly result:
      | { readonly status: "selected"; readonly path: string; readonly saveReference: string }
      | { readonly status: "cancelled" };
  };
  "file.write_text": {
    readonly payload: { readonly saveReference: string; readonly text: string };
    readonly result: { readonly status: "written" };
  };
  "file.write_binary": {
    readonly payload: { readonly saveReference: string; readonly bytesBase64Url: string };
    readonly result: { readonly status: "written" };
  };
  "storage.state.read": {
    readonly payload: EmptyPayload;
    readonly result:
      | { readonly status: "snapshot"; readonly snapshot: string }
      | { readonly status: "absent" };
  };
  "storage.state.replace": {
    readonly payload: { readonly snapshot: string };
    readonly result: { readonly status: "replaced" };
  };
  "storage.state.clear": {
    readonly payload: EmptyPayload;
    readonly result: { readonly status: "cleared" };
  };
}

export type PlatformOperation = keyof PlatformOperationMap;
export type PayloadFor<TOperation extends PlatformOperation> =
  PlatformOperationMap[TOperation]["payload"];
export type ResultFor<TOperation extends PlatformOperation> =
  PlatformOperationMap[TOperation]["result"];

export function createRequest<TOperation extends PlatformOperation>(
  requestId: string,
  operation: TOperation,
  payload: PayloadFor<TOperation>,
  cancellationId?: string,
): RequestEnvelope<TOperation> {
  return {
    protocolVersion: PROTOCOL_VERSION,
    kind: "request",
    requestId,
    operation,
    payload,
    ...(cancellationId === undefined ? {} : { cancellationId }),
  };
}

export function isSupportedProtocolVersion(version: ProtocolVersion): boolean {
  return (
    version.major === PROTOCOL_VERSION.major &&
    version.minor <= PROTOCOL_VERSION.minor
  );
}

export function protocolVersionToString(version: ProtocolVersion): string {
  return `${version.major}.${version.minor}`;
}

export * from "./validators.js";
export { isRecord } from "./validators/primitives.js";
