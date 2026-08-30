import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

const root = mkdtempSync(path.join(os.tmpdir(), "sley-contract-kit-"));
const cli = path.resolve("src/cli.mjs");

function run(args) {
  return spawnSync(process.execPath, [cli, ...args], { encoding: "utf8" });
}

function writeJson(file, value) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, `${JSON.stringify(value)}\n`);
}

try {
  const report = path.join(root, "report.json");
  writeJson(report, { schema: "sley.verify.report.v0", status: "passed" });
  const valid = run(["validate", "--json", "--schema", "sley.verify.report.v0", report]);
  assert.equal(valid.status, 0, valid.stderr);
  assert.equal(JSON.parse(valid.stdout).ok, true);

  for (const args of [
    ["validate", "--wat", report],
    ["validate", "--schema"],
    ["compare", "one"],
    ["--json", "validate"],
  ]) {
    const result = run(args);
    assert.equal(result.status, 2, `${args.join(" ")}\n${result.stdout}\n${result.stderr}`);
  }

  const empty = path.join(root, "empty");
  mkdirSync(empty);
  assert.equal(run(["check-fixtures", empty]).status, 2);
  assert.equal(run(["check-fixtures", empty, "--allow-empty"]).status, 0);
  assert.equal(run(["compare", empty, empty]).status, 2);
  assert.equal(run(["compare", empty, empty, "--allow-empty"]).status, 0);

  const left = path.join(root, "left");
  const right = path.join(root, "right");
  writeJson(path.join(left, "one.json"), { id: "one", schema: "sley.one.v0", root: "alpha" });
  writeJson(path.join(right, "one.json"), { id: "one", schema: "sley.one.v0", root: "alpha" });
  assert.equal(run(["compare", left, right, "--json"]).status, 0);

  writeJson(path.join(right, "one.json"), { id: "one", schema: "sley.one.v0", root: "beta" });
  const drift = run(["compare", left, right, "--json"]);
  assert.equal(drift.status, 1);
  assert.equal(JSON.parse(drift.stdout).drift[0].field, "root");

  writeJson(path.join(left, "two.json"), { id: "one", schema: "sley.two.v0" });
  const duplicate = run(["check-fixtures", left, "--json"]);
  assert.equal(duplicate.status, 2);
  assert.equal(JSON.parse(duplicate.stdout).error.code, "DUPLICATE_ID");

  writeFileSync(path.join(root, "bad.json"), "{");
  const malformed = run(["validate", "--json", "--schema", "sley.bad.v0", path.join(root, "bad.json")]);
  assert.equal(malformed.status, 2);
  assert.equal(JSON.parse(malformed.stdout).error.code, "MALFORMED_JSON");

  const huge = path.join(root, "huge.json");
  writeJson(huge, { schema: "sley.huge.v0", payload: "x".repeat(512) });
  const hugeResult = run(["validate", "--json", "--max-bytes", "64", "--schema", "sley.huge.v0", huge]);
  assert.equal(JSON.parse(hugeResult.stdout).error.code, "FILE_TOO_LARGE");

  let nested = { schema: "sley.deep.v0" };
  for (let index = 0; index < 70; index += 1) nested = { child: nested };
  const deep = path.join(root, "deep.json");
  writeJson(deep, nested);
  assert.equal(JSON.parse(run(["validate", "--json", "--schema", "sley.deep.v0", deep]).stdout).error.code, "JSON_TOO_DEEP");

  const unreadable = path.join(root, "unreadable.json");
  writeJson(unreadable, { schema: "sley.private.v0" });
  chmodSync(unreadable, 0o000);
  const permission = run(["validate", "--json", "--schema", "sley.private.v0", unreadable]);
  chmodSync(unreadable, 0o600);
  if (process.getuid?.() !== 0) {
    assert.equal(permission.status, 3);
    assert.equal(JSON.parse(permission.stdout).error.code, "READ_ERROR");
  }

  console.log("contract-kit smoke ok");
} finally {
  rmSync(root, { recursive: true, force: true });
}
