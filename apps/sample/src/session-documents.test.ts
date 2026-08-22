import assert from "node:assert/strict";
import test from "node:test";

import {
  LIVE_STATUS_ACTION,
  LIVE_STATUS_ASSERTIVE_DOCUMENT,
  LIVE_STATUS_INITIAL_DOCUMENT,
  LIVE_STATUS_POLITE_DOCUMENT,
} from "./session-documents.js";

type StatusNode = {
  readonly id: string;
  readonly kind: "status";
  readonly value: string;
  readonly politeness: "polite" | "assertive";
};

type LiveStatusDocument = {
  readonly format: string;
  readonly root: { readonly children: readonly (StatusNode | { readonly kind: string; readonly id: string })[] };
};

function statusIn(source: string): StatusNode {
  const document = JSON.parse(source) as LiveStatusDocument;
  assert.equal(document.format, "anodrel.ui.document.v3");
  const statuses = document.root.children.filter((node): node is StatusNode => node.kind === "status");
  assert.equal(statuses.length, 1, "a live-status document has exactly one status node");
  const [status] = statuses;
  assert.ok(status, "the one status node is available");
  return status;
}

test("live-status sample documents preserve one visible status across complete v3 replacements", () => {
  const initial = statusIn(LIVE_STATUS_INITIAL_DOCUMENT);
  const polite = statusIn(LIVE_STATUS_POLITE_DOCUMENT);
  const assertive = statusIn(LIVE_STATUS_ASSERTIVE_DOCUMENT);

  assert.equal(initial.politeness, "polite");
  assert.equal(polite.politeness, "polite");
  assert.equal(assertive.politeness, "assertive");
  assert.notEqual(initial.value, polite.value);
  assert.notEqual(polite.value, assertive.value);

  for (const source of [
    LIVE_STATUS_INITIAL_DOCUMENT,
    LIVE_STATUS_POLITE_DOCUMENT,
    LIVE_STATUS_ASSERTIVE_DOCUMENT,
  ]) {
    const document = JSON.parse(source) as LiveStatusDocument;
    assert.ok(
      document.root.children.some((node) => node.id === LIVE_STATUS_ACTION && node.kind === "action"),
      "every replacement retains the one semantic action",
    );
  }
});
