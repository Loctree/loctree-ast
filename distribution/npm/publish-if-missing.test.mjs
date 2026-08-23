import assert from "node:assert/strict";
import test from "node:test";

import { classifyNpmView } from "./publish-if-missing.mjs";

test("exact immutable version is skipped", () => {
  assert.equal(
    classifyNpmView({ status: 0, stdout: '"0.14.3"\n', stderr: "" }, "0.14.3"),
    "already-published",
  );
});

test("registry 404 is the only publishable missing state", () => {
  assert.equal(
    classifyNpmView(
      { status: 1, stdout: "", stderr: "npm error code E404\n404 Not Found" },
      "0.14.3",
    ),
    "missing",
  );
});

test("authentication and transport errors fail closed", () => {
  assert.throws(
    () =>
      classifyNpmView(
        { status: 1, stdout: "", stderr: "npm error code E401" },
        "0.14.3",
      ),
    /failed without a registry 404/,
  );
});

test("unexpected registry version fails closed", () => {
  assert.throws(
    () =>
      classifyNpmView({ status: 0, stdout: '"0.14.2"\n', stderr: "" }, "0.14.3"),
    /unexpected version/,
  );
});
