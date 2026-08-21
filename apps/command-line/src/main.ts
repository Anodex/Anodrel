import { MockHost } from "@anodrel/mock-host";
import { PlatformClient } from "@anodrel/sdk";

import { collectCommandLineReport } from "./report.js";

const host = new MockHost({
  applicationId: "org.anodrel.command-line-example",
  grantedCapabilities: ["diagnostics.read"],
  now: () => new Date("2026-08-21T00:00:00.000Z"),
});
const report = await collectCommandLineReport(new PlatformClient(host.createTransport()));

console.log(JSON.stringify(report, null, 2));
