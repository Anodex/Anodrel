import type { ResultFor } from "@anodrel/protocol";

/** The only Anodrel lifecycle boundary needed by an Anodex title-bar close control. */
export interface AnodexWindowCloseClient {
  closeSession(): Promise<ResultFor<"session.close">>;
}

/**
 * Requests the end of the authenticated session that owns this title bar.
 *
 * `accepted` proves only that the host accepted the request. It does not prove
 * that a native window has closed, every session view has ended, or a product
 * process has exited. The request carries no window target or native handle.
 */
export function requestAnodexTitleBarClose(
  client: AnodexWindowCloseClient,
): Promise<ResultFor<"session.close">> {
  return client.closeSession();
}
