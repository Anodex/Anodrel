//! Validation of a bounded native session-menu snapshot.

import {
  MAX_MENU_ACTION_ID_BYTES,
  MAX_MENU_ITEM_LABEL_BYTES,
  MAX_MENU_ITEMS,
  MAX_MENU_LABEL_BYTES,
  MAX_MENU_REPLACE_REQUEST_BYTES,
  MAX_MENUS,
} from "../index.js";
import type { NativeMenuShortcut, PayloadFor } from "../index.js";
import { isRecord } from "./primitives.js";

/** Validates one exact bounded native-session menu replacement. */
export function isMenuReplacePayload(
  value: unknown,
  shortcutsAllowed = true,
): value is PayloadFor<"menu.replace"> {
  if (
    !isRecord(value) ||
    Object.keys(value).length !== 1 ||
    !Array.isArray(value.menus) ||
    value.menus.length === 0 ||
    value.menus.length > MAX_MENUS ||
    !hasAtMostEncodedJsonBytes(value, MAX_MENU_REPLACE_REQUEST_BYTES)
  ) {
    return false;
  }

  const actionIds = new Set<string>();
  const shortcuts = new Set<string>();
  return value.menus.every((menu) => isNativeSessionMenu(menu, actionIds, shortcuts, shortcutsAllowed));
}

function isNativeSessionMenu(
  value: unknown,
  actionIds: Set<string>,
  shortcuts: Set<string>,
  shortcutsAllowed: boolean,
): boolean {
  if (
    !isRecord(value) ||
    Object.keys(value).length !== 2 ||
    !isMenuText(value.label, MAX_MENU_LABEL_BYTES) ||
    !Array.isArray(value.items) ||
    value.items.length === 0 ||
    value.items.length > MAX_MENU_ITEMS
  ) {
    return false;
  }

  for (const item of value.items) {
    if (!isRecord(item)) {
      return false;
    }
    const hasShortcut = Object.prototype.hasOwnProperty.call(item, "shortcut");
    if (
      Object.keys(item).length !== 3 + Number(hasShortcut) ||
      !isMenuActionId(item.id) ||
      !isMenuText(item.label, MAX_MENU_ITEM_LABEL_BYTES) ||
      typeof item.enabled !== "boolean" ||
      actionIds.has(item.id)
    ) {
      return false;
    }
    if (hasShortcut) {
      const shortcut = item.shortcut;
      if (!shortcutsAllowed || !isMenuShortcut(shortcut) || shortcuts.has(shortcut)) {
        return false;
      }
      shortcuts.add(shortcut);
    }
    actionIds.add(item.id);
  }
  return true;
}

function isMenuShortcut(value: unknown): value is NativeMenuShortcut {
  return typeof value === "string" && /^Ctrl\+(?:Shift\+)?[A-Z0-9]$/.test(value);
}

function isMenuActionId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_MENU_ACTION_ID_BYTES &&
    /^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$/.test(value)
  );
}

function isMenuText(value: unknown, maximumBytes: number): value is string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    !isWellFormedUnicode(value) ||
    new TextEncoder().encode(value).byteLength > maximumBytes
  ) {
    return false;
  }
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) {
      return false;
    }
  }
  return true;
}

function isWellFormedUnicode(value: string): boolean {
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

function hasAtMostEncodedJsonBytes(value: unknown, maximumBytes: number): boolean {
  try {
    return new TextEncoder().encode(JSON.stringify(value)).byteLength <= maximumBytes;
  } catch {
    return false;
  }
}
