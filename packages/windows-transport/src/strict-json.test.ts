import assert from "node:assert/strict";
import test from "node:test";

import { StrictJsonError, isStrictObject, parseStrictJson } from "./strict-json.js";

test("strict JSON preserves valid surrogate pairs", () => {
  const parsed = parseStrictJson('{"name":"\\uD83D\\uDE80"}');
  assert.equal(isStrictObject(parsed) ? parsed.name : undefined, "🚀");
});

test("strict JSON rejects duplicate fields and malformed surrogates", () => {
  assert.throws(() => parseStrictJson('{"one":1,"one":2}'), StrictJsonError);
  assert.throws(() => parseStrictJson('"\\uD800"'), StrictJsonError);
});
