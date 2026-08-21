import type { PlatformClient } from "@anodrel/sdk";

/** The safe, public facts printed by the command-line example. */
export interface CommandLineReport {
  readonly applicationId: string;
  readonly grantedCapabilities: readonly string[];
  readonly hostName: string;
  readonly protocolVersion: string;
}

/** Reads only the session facts that the public protocol exposes. */
export async function collectCommandLineReport(client: PlatformClient): Promise<CommandLineReport> {
  const [capabilities, health] = await Promise.all([
    client.getCapabilities(),
    client.getHealth(),
  ]);

  return {
    applicationId: capabilities.applicationId,
    grantedCapabilities: capabilities.grantedCapabilities,
    hostName: health.hostName,
    protocolVersion: `${health.protocolVersion.major}.${health.protocolVersion.minor}`,
  };
}
