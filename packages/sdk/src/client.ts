import {
  MAX_FILE_BINARY_WRITE_BYTES,
  PROTOCOL_VERSION,
  createRequest,
  encodeCanonicalBase64Url,
  type CancellationEnvelope,
  type PayloadFor,
  type PlatformOperation,
  type RequestEnvelope,
  type ResultFor,
  type NativeContextMenuItem,
  type NativeSessionMenu,
  type SessionWindowId,
  type SecondarySessionWindowId,
  type WindowFullscreenMode,
  type WindowState,
} from "@anodrel/protocol";

import { PlatformProtocolError, PlatformRemoteError } from "./errors.js";
import { UuidRequestIdFactory } from "./request-id.js";
import type { PlatformTransport, RequestIdFactory } from "./transport.js";

/** A typed application client over one authenticated platform transport. */
export class PlatformClient {
  constructor(
    private readonly transport: PlatformTransport,
    private readonly requestIds: RequestIdFactory = new UuidRequestIdFactory(),
  ) {}

  ping(sentAt: string): Promise<ResultFor<"platform.ping">> {
    return this.request("platform.ping", { sentAt });
  }

  getCapabilities(): Promise<ResultFor<"platform.capabilities">> {
    return this.request("platform.capabilities", {});
  }

  getHealth(): Promise<ResultFor<"platform.health">> {
    return this.request("platform.health", {});
  }

  readDiagnosticEntries(): Promise<ResultFor<"diagnostics.entries.read">> {
    return this.request("diagnostics.entries.read", {});
  }

  readCredential(name: string): Promise<ResultFor<"credential.read">> {
    return this.request("credential.read", { name });
  }

  writeCredential(name: string, secret: string): Promise<ResultFor<"credential.write">> {
    return this.request("credential.write", { name, secret });
  }

  deleteCredential(name: string): Promise<ResultFor<"credential.delete">> {
    return this.request("credential.delete", { name });
  }

  /**
   * Shows one bounded notification.
   *
   * A successful result means the host handed the values to the operating
   * system, not that anyone saw them. There is deliberately no way to learn
   * whether the user has notifications silenced or this application muted.
   */
  showNotification(title: string, body: string): Promise<ResultFor<"notification.show">> {
    return this.request("notification.show", { title, body });
  }

  /**
   * Proposes the title of this session's own window.
   *
   * A proposal, not an assignment. The host composes the displayed caption by
   * appending the application's validated display name, so `Report.pdf` becomes
   * `Report.pdf — Your Application`. That suffix cannot be suppressed or forged,
   * which is what stops a title claiming to be another application.
   *
   * There is no window argument, and none is coming: the host resolves the
   * window from the authenticated session.
   */
  setWindowTitle(title: string): Promise<ResultFor<"window.title.set">> {
    return this.request("window.title.set", { title });
  }

  /**
   * Requests a standard presentation state for this session's own window.
   *
   * This is deliberately a command, not a window object: there is no target,
   * native handle, or state-change event. A successful result means the host
   * UI thread accepted the closed request.
   */
  setWindowState(state: WindowState): Promise<ResultFor<"window.state.set">> {
    return this.request("window.state.set", { state });
  }

  /**
   * Reads one immediate standard state of this session's own window.
   *
   * This requires the separate `window.state.read` capability. The result is
   * only a minimized/maximized/restored snapshot; it has no target, native
   * handle, geometry, focus, timestamp, subscription, or state-change event.
   * It may be stale immediately, so refresh after requesting a state change.
   */
  getWindowState(): Promise<ResultFor<"window.state.get">> {
    return this.request("window.state.get", {});
  }

  /**
   * Consumes the latest coalesced presentation change for this session's own
   * window.
   *
   * `null` means that no unread change is retained. This is an immediate pull,
   * not a listener, wait, callback, or subscription; callers choose their own
   * refresh cadence. See `docs/WINDOW_STATE_CHANGES.md`.
   */
  readWindowStateChanges(): Promise<ResultFor<"window.state.changes.read">> {
    return this.request("window.state.changes.read", {});
  }

  /**
   * Asks Windows to foreground this session's own host window.
   *
   * There is no window argument, focus result, prior-foreground observation,
   * retry, or callback. A successful result means only that the host submitted
   * this narrow request to Windows; system foreground policy remains in charge.
   */
  requestWindowFocus(): Promise<ResultFor<"window.focus.request">> {
    return this.request("window.focus.request", {});
  }

  /**
   * Chooses reversible borderless fullscreen for this session's own window.
   *
   * This is not display control: there is no window target, monitor,
   * coordinate, display mode, geometry, fullscreen-state readback, or event.
   * The host retains the restoration details and success means only that its
   * UI thread accepted the closed requested mode.
   */
  setWindowFullscreen(
    mode: WindowFullscreenMode,
  ): Promise<ResultFor<"window.fullscreen.set">> {
    return this.request("window.fullscreen.set", { mode });
  }

  /**
   * Requests one bounded logical client size for this session's own window.
   *
   * Width and height are whole 96-DPI client-area pixels. The host alone maps
   * them to its framed window at the current DPI; there is no target, position,
   * monitor, geometry readback, resize event, or native handle.
   */
  setWindowSize(
    width: number,
    height: number,
  ): Promise<ResultFor<"window.size.set">> {
    return this.request("window.size.set", { width, height });
  }

  /**
   * Opens one independently revised secondary view in this authenticated
   * session. The returned identity is opaque and session-local: retain it only
   * to update or close this view through the matching SDK methods.
   *
   * Native size, position, monitor, parentage, and handles remain host-owned.
   */
  openWindow(
    title: string,
    document: string,
  ): Promise<ResultFor<"window.open">> {
    return this.request("window.open", { title, document });
  }

  /** Opens a secondary view from one exact version-2 scroll document. */
  openWindowV2(
    title: string,
    document: string,
  ): Promise<ResultFor<"window.open.v2">> {
    return this.request("window.open.v2", { title, document });
  }

  /** Opens a secondary view from one exact version-3 UI document. */
  openWindowV3(
    title: string,
    document: string,
  ): Promise<ResultFor<"window.open.v3">> {
    return this.request("window.open.v3", { title, document });
  }

  /**
   * Asks the host to close one secondary view issued to this session.
   *
   * Acceptance means the host queued the request; it does not report whether
   * Windows has destroyed a native surface or why a view later became absent.
   * The primary `main` view cannot be passed here—use `closeSession()` to end
   * the whole authenticated session.
   */
  closeWindow(windowId: SecondarySessionWindowId): Promise<ResultFor<"window.close">> {
    return this.request("window.close", { windowId });
  }

  /**
   * Replaces this session's complete native menu bar.
   *
   * Items are semantic display commands only. The host owns every native menu
   * identifier. Protocol 1.24 items can name only a canonical local semantic
   * shortcut; this method accepts no native accelerator, callback, target,
   * handle, or command payload.
   */
  replaceMenu(
    menus: readonly NativeSessionMenu[],
  ): Promise<ResultFor<"menu.replace">> {
    return this.request("menu.replace", { menus });
  }

  /**
   * Replaces this session's complete host-owned native context menu.
   *
   * A context menu appears only through the host's documented local input
   * route. This call supplies semantic display items only: it has no point,
   * selection, link, target, native handle, callback, shortcut, or command
   * number. The returned revision is host-owned and opaque.
   */
  replaceContextMenu(
    items: readonly NativeContextMenuItem[],
  ): Promise<ResultFor<"menu.context.replace">> {
    return this.request("menu.context.replace", { items });
  }

  replaceUiDocument(document: string): Promise<ResultFor<"ui.document.replace">> {
    return this.request("ui.document.replace", { document });
  }

  replaceUiDocumentV2(document: string): Promise<ResultFor<"ui.document.replace.v2">> {
    return this.request("ui.document.replace.v2", { document });
  }

  /** Replaces the primary surface with one exact version-3 UI document. */
  replaceUiDocumentV3(document: string): Promise<ResultFor<"ui.document.replace.v3">> {
    return this.request("ui.document.replace.v3", { document });
  }

  /**
   * Replaces the strict v1 document of `main` or one secondary view this
   * session received from `openWindow`. Every view keeps its own revision.
   */
  replaceUiDocumentInWindow(
    windowId: SessionWindowId,
    document: string,
  ): Promise<ResultFor<"ui.document.replace.window">> {
    return this.request("ui.document.replace.window", { windowId, document });
  }

  /** Replaces `main` or one issued secondary view with an exact v2 scroll document. */
  replaceUiDocumentV2InWindow(
    windowId: SessionWindowId,
    document: string,
  ): Promise<ResultFor<"ui.document.replace.window.v2">> {
    return this.request("ui.document.replace.window.v2", { windowId, document });
  }

  /** Replaces `main` or one issued secondary view with an exact v3 document. */
  replaceUiDocumentV3InWindow(
    windowId: SessionWindowId,
    document: string,
  ): Promise<ResultFor<"ui.document.replace.window.v3">> {
    return this.request("ui.document.replace.window.v3", { windowId, document });
  }

  /**
   * Reads every field value on this session's own current surface.
   *
   * A snapshot taken when you ask, not a stream. There is no way to name a
   * field and no change event, so this cannot be used to follow what someone
   * is typing — call it when a person has finished, such as on a submit
   * action. See `docs/UI_FIELDS.md`.
   */
  readUiFields(): Promise<ResultFor<"ui.fields.read">> {
    return this.request("ui.fields.read", {});
  }

  readUiEvents(): Promise<ResultFor<"ui.events.read">> {
    return this.request("ui.events.read", {});
  }

  /**
   * Drains bounded semantic events from every current session view.
   *
   * Each event carries its opaque `windowId`. Event order is meaningful only
   * inside that one view; do not infer cross-window desktop timing from this
   * batch.
   */
  readWindowUiEvents(): Promise<ResultFor<"ui.events.read.window">> {
    return this.request("ui.events.read.window", {});
  }

  closeSession(): Promise<ResultFor<"session.close">> {
    return this.request("session.close", {});
  }

  readClipboardText(): Promise<ResultFor<"clipboard.read">> {
    return this.request("clipboard.read", {});
  }

  writeClipboardText(text: string): Promise<ResultFor<"clipboard.write">> {
    return this.request("clipboard.write", { text });
  }

  openExternalLink(url: string): Promise<ResultFor<"external.open">> {
    return this.request("external.open", { url });
  }

  /**
   * Fetches bounded UTF-8 text from one host-authorized HTTPS origin.
   *
   * This accepts only the URL. There are no options for methods, headers,
   * request bodies, cookies, credentials, redirects, proxies, timeouts, or
   * TLS. A non-2xx HTTP status is still a successful protocol result when the
   * host can represent its bounded text response.
   */
  fetchHttpsText(url: string): Promise<ResultFor<"network.fetch_text">> {
    return this.request("network.fetch_text", { url });
  }

  openFileDialog(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.open_file">> {
    return this.request("dialog.open_file", { filters });
  }

  /**
   * Opens one host-owned folder picker.
   *
   * A returned path is display data only: it is not a retained folder
   * permission, handle, enumeration route, or later filesystem authority.
   */
  openFolderDialog(): Promise<ResultFor<"dialog.open_folder">> {
    return this.request("dialog.open_folder", {});
  }

  /**
   * Opens one host-owned folder picker and returns a one-use entry reference.
   *
   * The returned path remains display data. Present the opaque reference only
   * to {@link readSelectedFolderEntries}; it is not a path, handle, permission,
   * or reusable grant.
   */
  openFolderDialogWithReference(): Promise<ResultFor<"dialog.open_folder.v2">> {
    return this.request("dialog.open_folder.v2", {});
  }

  /**
   * Returns one bounded direct-entry snapshot from a selected folder.
   *
   * This consumes the reference once. It has no recursive, pagination, child
   * path, metadata, content-read, mutation, or watch option.
   */
  readSelectedFolderEntries(folderReference: string): Promise<ResultFor<"folder.read_entries">> {
    return this.request("folder.read_entries", { folderReference });
  }

  saveFileDialog(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.save_file">> {
    return this.request("dialog.save_file", { filters });
  }

  openFileDialogWithReference(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.open_file.v2">> {
    return this.request("dialog.open_file.v2", { filters });
  }

  readSelectedFileText(selectionReference: string): Promise<ResultFor<"file.read_text">> {
    return this.request("file.read_text", { selectionReference });
  }

  /**
   * Opens a host-owned save picker and returns a one-use output reference.
   *
   * The returned path is display data only. Present its `saveReference` to
   * {@link writeSelectedFileText}; do not persist, parse, or transform it.
   */
  saveFileDialogWithReference(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.save_file.v2">> {
    return this.request("dialog.save_file.v2", { filters });
  }

  /**
   * Replaces bounded text through one previously captured output reference.
   *
   * The host consumes the reference once. This is not a path-based write API,
   * and success is not an atomic-replacement or durability guarantee.
   */
  writeSelectedFileText(
    saveReference: string,
    text: string,
  ): Promise<ResultFor<"file.write_text">> {
    return this.request("file.write_text", { saveReference, text });
  }

  /**
   * Replaces one selected destination with bounded binary data.
   *
   * The SDK uses Anodrel's first-party canonical base64url encoder and rejects
   * data above the Protocol 1.22 limit before constructing a transport frame.
   * The host still consumes the one-use save reference and provides the final
   * filesystem boundary; no path, offset, stream, or MIME value is accepted.
   */
  writeSelectedFileBinary(
    saveReference: string,
    bytes: Uint8Array,
  ): Promise<ResultFor<"file.write_binary">> {
    if (bytes.byteLength > MAX_FILE_BINARY_WRITE_BYTES) {
      return Promise.reject(
        new RangeError(`Binary output exceeds the ${MAX_FILE_BINARY_WRITE_BYTES}-byte protocol limit.`),
      );
    }
    return this.request("file.write_binary", {
      saveReference,
      bytesBase64Url: encodeCanonicalBase64Url(bytes),
    });
  }

  readStorageState(): Promise<ResultFor<"storage.state.read">> {
    return this.request("storage.state.read", {});
  }

  replaceStorageState(snapshot: string): Promise<ResultFor<"storage.state.replace">> {
    return this.request("storage.state.replace", { snapshot });
  }

  clearStorageState(): Promise<ResultFor<"storage.state.clear">> {
    return this.request("storage.state.clear", {});
  }

  async cancel(cancellationId: string): Promise<void> {
    await this.transport.cancel({
      protocolVersion: PROTOCOL_VERSION,
      kind: "cancel",
      cancellationId,
    });
  }

  private async request<TOperation extends PlatformOperation>(
    operation: TOperation,
    payload: PayloadFor<TOperation>,
    cancellationId?: string,
  ): Promise<ResultFor<TOperation>> {
    const request: RequestEnvelope<TOperation> = createRequest(
      this.requestIds.next(),
      operation,
      payload,
      cancellationId,
    );
    const response = await this.transport.send(request);

    if (response.requestId !== request.requestId) {
      throw new PlatformProtocolError(
        `Host response ID ${response.requestId} did not match request ID ${request.requestId}.`,
      );
    }

    if (response.status === "failure") {
      throw new PlatformRemoteError(response.error);
    }

    return response.result;
  }
}
