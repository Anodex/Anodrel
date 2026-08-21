import {
  PROTOCOL_VERSION,
  createRequest,
  type CancellationEnvelope,
  type PayloadFor,
  type PlatformOperation,
  type RequestEnvelope,
  type ResultFor,
  type NativeSessionMenu,
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
   * native handle, current-state readback, or state-change event. A successful
   * result means the host UI thread accepted the closed request.
   */
  setWindowState(state: WindowState): Promise<ResultFor<"window.state.set">> {
    return this.request("window.state.set", { state });
  }

  /**
   * Replaces this session's complete native menu bar.
   *
   * Items are semantic display commands only. The host owns every native menu
   * identifier and accepts no accelerator, callback, target, handle, or
   * command payload through this method.
   */
  replaceMenu(
    menus: readonly NativeSessionMenu[],
  ): Promise<ResultFor<"menu.replace">> {
    return this.request("menu.replace", { menus });
  }

  replaceUiDocument(document: string): Promise<ResultFor<"ui.document.replace">> {
    return this.request("ui.document.replace", { document });
  }

  replaceUiDocumentV2(document: string): Promise<ResultFor<"ui.document.replace.v2">> {
    return this.request("ui.document.replace.v2", { document });
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

  openFileDialog(
    filters: readonly { readonly label: string; readonly extensions: readonly string[] }[],
  ): Promise<ResultFor<"dialog.open_file">> {
    return this.request("dialog.open_file", { filters });
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
