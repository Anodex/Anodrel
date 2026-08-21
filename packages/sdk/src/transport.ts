import type {
  CancellationEnvelope,
  PlatformOperation,
  RequestEnvelope,
  ResponseEnvelope,
} from "@anodrel/protocol";

/** Supplies request IDs for one application client. */
export interface RequestIdFactory {
  next(): string;
}

/**
 * The narrow, host-owned application transport boundary.
 *
 * A transport authenticates and binds requests to its session before a host
 * handles them. It must never trust a caller-supplied identity or capability
 * context.
 */
export interface PlatformTransport {
  send<TOperation extends PlatformOperation>(
    request: RequestEnvelope<TOperation>,
  ): Promise<ResponseEnvelope<TOperation>>;
  cancel(cancellation: CancellationEnvelope): Promise<void>;
}
