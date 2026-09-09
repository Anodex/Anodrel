/** Stable shared protocol data types without operation-specific behaviour. */

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
  | "menu.write"
  | "menu.context.write";

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

/** One enabled or disabled semantic command in a native context menu. */
export interface NativeContextMenuItem {
  readonly id: string;
  readonly label: string;
  readonly enabled: boolean;
}
