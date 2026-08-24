import type {
  Capability,
  PlatformOperation,
  ProtocolError,
  ProtocolErrorCode,
  ResponseEnvelope,
  ResultFor,
} from "@anodrel/protocol";

/** Host-owned data and response helpers made available to operation handlers. */
export interface MockOperationContext {
  readonly applicationId: string;
  readonly grantedCapabilities: readonly Capability[];
  readonly hostName: string;
  readonly now: () => Date;
  clipboardText: string | undefined;
  storageSnapshot: string | undefined;
  readonly credentials: Map<string, string>;
  readonly networkTextResponse: Readonly<{ statusCode: number; text: string }> | undefined;
  hasCapability(sessionId: string, capability: Capability): boolean;
  success<TOperation extends PlatformOperation>(
    operation: TOperation,
    requestId: string,
    result: ResultFor<TOperation>,
  ): ResponseEnvelope<TOperation>;
  failure(
    requestId: string,
    code: ProtocolErrorCode,
    message: string,
    details?: ProtocolError["details"],
  ): ResponseEnvelope;
}
