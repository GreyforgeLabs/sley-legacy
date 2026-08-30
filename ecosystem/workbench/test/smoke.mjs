import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { PassThrough } from "node:stream";
import {
  VERSION,
  parseCliArgs,
  runBoundedCommand,
  startWorkbench,
} from "../src/server.mjs";

function makeCompiler(directory) {
  const executable = path.join(directory, "fake-sley");
  writeFileSync(executable, `#!/usr/bin/env bash
set -euo pipefail
command_name="$1"
target="\${@: -1}"
if grep -q SLOW "$target" 2>/dev/null; then
  sleep 30 &
  echo "$!" > "$target.child"
  wait
fi
if grep -q FLOOD "$target" 2>/dev/null; then
  i=0
  while [ "$i" -lt 10000 ]; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; i=$((i + 1)); done
  sleep 30
fi
printf '{"command":"%s","ok":true}\n' "$command_name"
`, "utf8");
  chmodSync(executable, 0o755);
  return executable;
}

async function eventually(assertion, timeoutMs = 1500) {
  const start = Date.now();
  let lastError;
  while (Date.now() - start < timeoutMs) {
    try {
      return assertion();
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
  }
  throw lastError;
}

async function harness(options = {}) {
  const directory = mkdtempSync(path.join(os.tmpdir(), "sley-workbench-"));
  const workspace = path.join(directory, "workspace");
  const sibling = path.join(directory, "workspace-sibling");
  const fs = await import("node:fs");
  fs.mkdirSync(workspace);
  fs.mkdirSync(sibling);
  writeFileSync(path.join(workspace, "main.sley"), "task main -> Text { return \"ok\" }\n");
  writeFileSync(path.join(workspace, "slow.sley"), "SLOW\n");
  writeFileSync(path.join(workspace, "flood.sley"), "FLOOD\n");
  writeFileSync(path.join(sibling, "outside.sley"), "outside\n");
  symlinkSync(path.join(sibling, "outside.sley"), path.join(workspace, "escape.sley"));
  const instance = await startWorkbench({
    host: "127.0.0.1",
    port: 0,
    sleyBin: makeCompiler(directory),
    target: workspace,
    timeoutMs: 200,
    maxOutputBytes: 4096,
    ...options,
  });
  return {
    ...instance,
    directory,
    workspace,
    sibling,
    close: async () => {
      await new Promise((resolve) => instance.server.close(resolve));
      rmSync(directory, { recursive: true, force: true });
    },
  };
}

async function postRun(instance, body, headers = {}) {
  return fetch(`${instance.state.origin}/api/run`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      origin: instance.state.origin,
      "x-sley-workbench-csrf": instance.state.csrfToken,
      ...headers,
    },
    body: JSON.stringify(body),
  });
}

function rawRequest(instance, requestOptions = {}) {
  return new Promise((resolve, reject) => {
    const address = instance.server.address();
    const request = http.request({
      host: "127.0.0.1",
      port: address.port,
      path: requestOptions.path || "/api/health",
      method: requestOptions.method || "GET",
      headers: requestOptions.headers || {},
    }, (response) => {
      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => { body += chunk; });
      response.on("end", () => resolve({ status: response.statusCode, body }));
    });
    request.on("error", reject);
    request.end(requestOptions.body || "");
  });
}

const defaults = parseCliArgs([], {});
assert.equal(defaults.host, "127.0.0.1");
assert.equal(defaults.port, 4173);
assert.equal(defaults.target, ".");
assert.equal(defaults.allowMutation, false);
assert.equal(defaults.origin, "http://127.0.0.1:4173");

const parsed = parseCliArgs([
  "--target", "project", "--port", "4999", "--host", "localhost",
  "--timeout-ms", "500", "--max-output-bytes", "2048", "--sley-bin", "/bin/false",
  "--allow-mutation",
], {});
assert.equal(parsed.target, "project");
assert.equal(parsed.port, 4999);
assert.equal(parsed.timeoutMs, 500);
assert.equal(parsed.maxOutputBytes, 2048);
assert.equal(parsed.allowMutation, true);

for (const args of [
  ["--target"], ["--unknown"], ["--port", "0"], ["--port", "nope"],
  ["--timeout-ms", "99"], ["--host", "0.0.0.0"],
  ["--host", "0.0.0.0", "--allow-remote"],
]) {
  assert.throws(() => parseCliArgs(args, {}), undefined, args.join(" "));
}
assert.equal(
  parseCliArgs([
    "--host", "0.0.0.0", "--allow-remote", "--origin", "https://workbench.example",
  ], {}).origin,
  "https://workbench.example",
);

const instance = await harness();
try {
  for (const route of ["/", "/app.js", "/styles.css"]) {
    const response = await fetch(`${instance.state.origin}${route}`);
    assert.equal(response.status, 200, route);
    assert.ok((await response.text()).length > 20);
  }
  assert.equal((await fetch(`${instance.state.origin}/missing.js`)).status, 404);
  assert.equal((await fetch(`${instance.state.origin}/api/health`)).status, 200);
  const health = await (await fetch(`${instance.state.origin}/api/health`)).json();
  assert.deepEqual(health, { ok: true, version: VERSION, mode: "read-only" });
  assert.equal(VERSION, JSON.parse(readFileSync("package.json", "utf8")).version);

  const hostileHost = await rawRequest(instance, { headers: { host: "evil.example" } });
  assert.equal(hostileHost.status, 403);

  const getRun = await fetch(`${instance.state.origin}/api/run?command=check`);
  assert.equal(getRun.status, 405);
  assert.equal((await postRun(instance, { command: "check", target: "main.sley" }, { origin: "http://evil.example" })).status, 403);
  assert.equal((await postRun(instance, { command: "check", target: "main.sley" }, { "x-sley-workbench-csrf": "wrong" })).status, 403);

  for (const target of [
    path.join(instance.sibling, "outside.sley"),
    "../workspace-sibling/outside.sley",
    "escape.sley",
  ]) {
    assert.equal((await postRun(instance, { command: "check", target })).status, 403, target);
  }

  const valid = await postRun(instance, { command: "check", target: "main.sley" });
  assert.equal(valid.status, 200);
  assert.equal((await valid.json()).ok, true);

  assert.equal((await postRun(instance, { command: "format", target: "main.sley" })).status, 403);

  const timedOut = await (await postRun(instance, { command: "check", target: "slow.sley" })).json();
  assert.equal(timedOut.ok, false);
  assert.equal(timedOut.timedOut, true);
  const childPid = Number.parseInt(readFileSync(path.join(instance.workspace, "slow.sley.child"), "utf8"), 10);
  await eventually(() => assert.throws(() => process.kill(childPid, 0), /ESRCH/));

  const flooded = await (await postRun(instance, { command: "check", target: "flood.sley" })).json();
  assert.equal(flooded.ok, false);
  assert.equal(flooded.outputFlooded, true);
  assert.ok(Buffer.byteLength(flooded.stdout) <= 4096);
} finally {
  await instance.close();
}

const mutationInstance = await harness({ allowMutation: true });
try {
  const result = await postRun(mutationInstance, { command: "format", target: "main.sley" });
  assert.equal(result.status, 200);
  assert.equal((await result.json()).ok, true);
} finally {
  await mutationInstance.close();
}

const cancellationDirectory = mkdtempSync(path.join(os.tmpdir(), "sley-workbench-cancel-"));
try {
  const compiler = makeCompiler(cancellationDirectory);
  const target = path.join(cancellationDirectory, "slow.sley");
  writeFileSync(target, "SLOW\n");
  const controller = new AbortController();
  const promise = runBoundedCommand(compiler, ["check", target], {
    timeoutMs: 5000,
    maxOutputBytes: 4096,
    signal: controller.signal,
  });
  await eventually(() => assert.ok(readFileSync(`${target}.child`, "utf8")));
  controller.abort();
  const result = await promise;
  assert.equal(result.cancelled, true);
} finally {
  rmSync(cancellationDirectory, { recursive: true, force: true });
}

const streamFailure = await harness({
  createReadStream() {
    const stream = new PassThrough();
    queueMicrotask(() => stream.emit("error", new Error("injected")));
    return stream;
  },
});
try {
  assert.equal((await fetch(`${streamFailure.state.origin}/app.js`)).status, 500);
} finally {
  await streamFailure.close();
}

console.log("sley-workbench smoke ok");
