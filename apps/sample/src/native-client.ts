import { readFileSync } from "node:fs";

import { PlatformClient } from "@anodrel/sdk";
import { WindowsNamedPipeTransport, decodeBootstrapInvitation } from "@anodrel/windows-transport";

process.on("uncaughtException", () => {
  process.exitCode = 14;
});
process.on("unhandledRejection", () => {
  process.exitCode = 15;
});

async function run(): Promise<number> {
  let invitation;
  try {
    invitation = decodeBootstrapInvitation(readFileSync(0));
  } catch {
    return 11;
  }

  let transport: WindowsNamedPipeTransport;
  try {
    transport = await WindowsNamedPipeTransport.connect(invitation);
  } catch {
    return 12;
  }
  try {
    const health = await new PlatformClient(transport).getHealth();
    if (health.status !== "ready") {
      return 13;
    }
    return 0;
  } catch {
    return 13;
  } finally {
    await transport.close();
  }
}

// The bootstrap launcher directs child output to NUL. Never serialize an error
// here because it could contain sensitive host material; the numeric stage is
// sufficient for a developer to locate the failing boundary.
process.exitCode = await run();
