import { PlatformProtocolError } from "./errors.js";
import type { RequestIdFactory } from "./transport.js";

/** Creates cryptographically strong request IDs for production clients. */
export class UuidRequestIdFactory implements RequestIdFactory {
  next(): string {
    const generator = globalThis.crypto?.randomUUID;
    if (generator === undefined) {
      throw new PlatformProtocolError(
        "This runtime cannot generate a cryptographically strong request ID.",
      );
    }

    return generator.call(globalThis.crypto);
  }
}
