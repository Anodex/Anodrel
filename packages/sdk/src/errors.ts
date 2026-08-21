import type { ProtocolError } from "@anodrel/protocol";

/** A stable, safe failure response returned by the host. */
export class PlatformRemoteError extends Error {
  readonly code: ProtocolError["code"];
  readonly retryable: boolean;
  readonly details: ProtocolError["details"];

  constructor(error: ProtocolError) {
    super(error.message);
    this.name = "PlatformRemoteError";
    this.code = error.code;
    this.retryable = error.retryable;
    this.details = error.details;
  }
}

/** A malformed transport response or missing local security prerequisite. */
export class PlatformProtocolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PlatformProtocolError";
  }
}
