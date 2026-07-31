import { MockHost } from "@anodrel/mock-host";
import { PlatformClient } from "@anodrel/sdk";

const host = new MockHost({
  applicationId: "anodrel.sample",
  grantedCapabilities: ["diagnostics.read"],
  now: () => new Date("2026-07-31T00:00:00.000Z"),
});
const client = new PlatformClient(host.createTransport());

const [ping, capabilities, health] = await Promise.all([
  client.ping("2026-07-31T00:00:00.000Z"),
  client.getCapabilities(),
  client.getHealth(),
]);

console.log(JSON.stringify({ ping, capabilities, health }, null, 2));
