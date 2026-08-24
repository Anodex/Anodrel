/** Fixed limits and protocol-version values shared by Anodrel boundaries. */

export const PROTOCOL_VERSION = { major: 1, minor: 28 } as const;
export const MAX_REQUEST_ID_BYTES = 256;
export const MAX_OPERATION_BYTES = 128;
export const MAX_CANCELLATION_ID_BYTES = 256;
export const MAX_UI_DOCUMENT_REQUEST_BYTES = 24 * 1024;
export const MAX_CLIPBOARD_TEXT_REQUEST_BYTES = 24 * 1024;
export const MAX_EXTERNAL_LINK_REQUEST_BYTES = 2 * 1024;
/** Maximum UTF-8 bytes in the exact HTTPS text-fetch URL payload. */
export const MAX_NETWORK_FETCH_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_REQUEST_BYTES = 2 * 1024;
export const MAX_FILE_DIALOG_FILTERS = 8;
export const MAX_FILE_TEXT_RESPONSE_BYTES = 8 * 1024;
export const MAX_FILE_TEXT_WRITE_BYTES = 8 * 1024;
/** Maximum decoded bytes in one exact binary-output replacement. */
export const MAX_FILE_BINARY_WRITE_BYTES = 32 * 1024;
/** Maximum encoded JSON bytes in one complete native-menu replacement payload. */
export const MAX_MENU_REPLACE_REQUEST_BYTES = 16 * 1024;
export const MAX_MENUS = 8;
export const MAX_MENU_ITEMS = 16;
export const MAX_MENU_LABEL_BYTES = 32;
export const MAX_MENU_ITEM_LABEL_BYTES = 96;
export const MAX_MENU_ACTION_ID_BYTES = 64;
export const MAX_STORAGE_SNAPSHOT_REQUEST_BYTES = 24 * 1024;
export const SELECTION_REFERENCE_BYTES = 22;
/** Exact characters in a host-created save reference. */
export const SAVE_REFERENCE_BYTES = 22;
/** Maximum UTF-8 bytes in an exact credential name (ASCII only). */
export const MAX_CREDENTIAL_NAME_BYTES = 64;
/** Maximum characters in the canonical hexadecimal representation of a secret. */
export const MAX_CREDENTIAL_SECRET_HEX_BYTES = 4_096;
/** Smallest logical client width accepted by `window.size.set`. */
export const MIN_WINDOW_CLIENT_WIDTH = 320;
/** Largest logical client width accepted by `window.size.set`. */
export const MAX_WINDOW_CLIENT_WIDTH = 3_840;
/** Smallest logical client height accepted by `window.size.set`. */
export const MIN_WINDOW_CLIENT_HEIGHT = 240;
/** Largest logical client height accepted by `window.size.set`. */
export const MAX_WINDOW_CLIENT_HEIGHT = 2_160;
