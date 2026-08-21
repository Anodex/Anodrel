import assert from "node:assert/strict";
import test from "node:test";

import { MockHost } from "@anodrel/mock-host";
import { PlatformClient } from "@anodrel/sdk";

import { collectCommandLineReport } from "./report.js";

test("command-line report uses only public host facts", async () => {
  const host = new MockHost({
    applicationId: "org.example.command-line",
    grantedCapabilities: ["diagnostics.read", "clipboard.read"],
  });

  const report = await collectCommandLineReport(new PlatformClient(host.createTransport()));

  assert.deepEqual(report, {
    applicationId: "org.example.command-line",
    grantedCapabilities: ["diagnostics.read", "clipboard.read"],
    hostName: "anodrel-mock-host",
    protocolVersion: "1.16",
  });
});
