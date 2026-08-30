import test from "node:test";
import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, readdir, readFile, rm, utimes, writeFile } from "node:fs/promises";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { createLearnSleyServer, testInternals } from "../server.mjs";

const trialId = "sley.enter.return_text";
const goodCode = 'task main -> Text {\n  return "hello, Sley"\n}\n';

async function makeFakeCompiler(dir) {
  const compilerPath = path.join(dir, "fake-sley");
  await writeFile(compilerPath, `#!/usr/bin/env bash
set -euo pipefail
cmd="$1"
source="\${@: -1}"
if grep -q TIMEOUT_FORK "$source"; then
  sleep 30 &
  echo "$!" > "\${LEARNSLEY_CHILD_PID_FILE:-/dev/null}"
  wait
fi
if grep -q FLOOD "$source"; then
  python3 - <<'PY'
print("x" * 200000)
PY
  sleep 30
fi
if grep -q FAIL "$source"; then
  echo '{"error":"fail"}'
  exit 1
fi
case "$cmd" in
  check) echo '{"ok":true}' ;;
  run) echo '{"kind":"Text","value":"hello, Sley"}' ;;
  graph) echo '{"nodes":[]}' ;;
  seal) echo '{"seal":true}' ;;
  format) cat "$source" ;;
  *) echo "unknown command" >&2; exit 2 ;;
esac
`, "utf8");
  await chmod(compilerPath, 0o755);
  return compilerPath;
}

async function eventually(assertion, timeoutMs = 1000) {
  const startedAt = Date.now();
  let lastError;
  while (Date.now() - startedAt < timeoutMs) {
    try {
      return await assertion();
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
  }
  throw lastError;
}

async function startHarness(host = "127.0.0.1", options = {}) {
  const dir = await mkdtemp(path.join(os.tmpdir(), "learnsley-test-"));
  const sleyBin = await makeFakeCompiler(dir);
  const { server, appState } = createLearnSleyServer({
    host,
    port: 0,
    sleyBin,
    progressPath: path.join(dir, "progress.json"),
    tmpDir: path.join(dir, "tmp"),
    timeoutMs: 200,
    logger: { warn() {} },
    ...options
  });
  await mkdir(appState.tmpDir, { recursive: true });
  await new Promise((resolve) => server.listen(0, host, resolve));
  const address = server.address();
  const origin = `http://${host === "::1" ? "[::1]" : host}:${address.port}`;
  return {
    dir,
    server,
    appState,
    origin,
    close: async () => {
      await new Promise((resolve) => server.close(resolve));
      await rm(dir, { recursive: true, force: true });
    }
  };
}

async function getJson(url) {
  const response = await fetch(url);
  return { response, payload: await response.json() };
}

async function postJson(harness, body, headers = {}) {
  const response = await fetch(`${harness.origin}/api/run`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      origin: harness.origin,
      "x-learnsley-csrf": harness.appState.csrfToken,
      ...headers
    },
    body: JSON.stringify(body)
  });
  return { response, payload: await response.json() };
}

function rawRequest(port, { host = "127.0.0.1", hostHeader, origin, contentType = "application/json", csrf, body = {} }) {
  return new Promise((resolve, reject) => {
    const data = typeof body === "string" ? body : JSON.stringify(body);
    const req = http.request({
      host,
      port,
      path: "/api/run",
      method: "POST",
      headers: {
        host: hostHeader,
        origin,
        "content-type": contentType,
        "x-learnsley-csrf": csrf,
        "content-length": Buffer.byteLength(data)
      }
    }, (res) => {
      let responseBody = "";
      res.setEncoding("utf8");
      res.on("data", (chunk) => { responseBody += chunk; });
      res.on("end", () => resolve({ status: res.statusCode, body: JSON.parse(responseBody) }));
    });
    req.on("error", reject);
    req.end(data);
  });
}

test("mutations require loopback host, same-origin Origin, CSRF, and application/json", async () => {
  const harness = await startHarness();
  try {
    const port = harness.server.address().port;
    const validBody = { trialId, code: goodCode };

    assert.equal((await rawRequest(port, {
      hostHeader: "evil.example",
      origin: harness.origin,
      csrf: harness.appState.csrfToken,
      body: validBody
    })).status, 403);

    for (const malformedHost of ["[::1]evil", "localhost@evil.example", "localhost:99999"]) {
      assert.equal((await rawRequest(port, {
        hostHeader: malformedHost,
        origin: `http://${malformedHost}`,
        csrf: harness.appState.csrfToken,
        body: validBody
      })).status, 403);
    }

    assert.equal((await rawRequest(port, {
      hostHeader: `127.0.0.1:${port}`,
      origin: "null",
      csrf: harness.appState.csrfToken,
      body: validBody
    })).status, 403);

    assert.equal((await rawRequest(port, {
      hostHeader: `127.0.0.1:${port}`,
      origin: "http://evil.example",
      csrf: harness.appState.csrfToken,
      contentType: "text/plain",
      body: JSON.stringify(validBody)
    })).status, 415);

    assert.equal((await postJson(harness, validBody, { "x-learnsley-csrf": "" })).response.status, 403);
    assert.equal((await postJson(harness, validBody, { "x-learnsley-csrf": "wrong" })).response.status, 403);

    const valid = await postJson(harness, validBody);
    assert.equal(valid.response.status, 200);
    assert.equal(valid.payload.ok, true);
  } finally {
    await harness.close();
  }
});

test("GET routes remain read-only and health does not disclose compiler paths", async () => {
  const harness = await startHarness();
  try {
    const before = await getJson(`${harness.origin}/api/progress`);
    const health = await getJson(`${harness.origin}/api/health`);
    const trials = await getJson(`${harness.origin}/api/trials`);
    const after = await getJson(`${harness.origin}/api/progress`);

    assert.equal(health.response.status, 200);
    assert.equal(JSON.stringify(health.payload).includes(harness.appState.sleyBin), false);
    assert.equal(trials.response.status, 200);
    assert.deepEqual(after.payload.progress.attempts, before.payload.progress.attempts);
  } finally {
    await harness.close();
  }
});

test("IPv4 and IPv6 loopback origins can make valid bundled-UI style mutations", async (t) => {
  const ipv4 = await startHarness("127.0.0.1");
  try {
    assert.equal((await postJson(ipv4, { trialId, code: goodCode })).response.status, 200);
  } finally {
    await ipv4.close();
  }

  let ipv6;
  try {
    ipv6 = await startHarness("::1");
  } catch (error) {
    t.skip(`IPv6 loopback unavailable: ${error.message}`);
    return;
  }
  try {
    assert.equal((await postJson(ipv6, { trialId, code: goodCode })).response.status, 200);
  } finally {
    await ipv6.close();
  }
});

test("successful, failed, timed-out, and output-flooded runs clean temporary workspaces", async () => {
  const harness = await startHarness();
  try {
    assert.equal((await postJson(harness, { trialId, code: goodCode })).response.status, 200);
    assert.equal((await postJson(harness, { trialId, code: `${goodCode}\n// FAIL` })).payload.run.ok, false);
    const timedOut = await postJson(harness, { trialId, code: `${goodCode}\n// TIMEOUT_FORK` });
    assert.equal(timedOut.payload.run.check.timedOut, true);
    const flooded = await postJson(harness, { trialId, code: `${goodCode}\n// FLOOD` });
    assert.equal(flooded.payload.run.check.outputFlooded, true);
    assert.deepEqual(await readdir(harness.appState.tmpDir), []);
  } finally {
    await harness.close();
  }
});

test("cancelled compiler run terminates the process group", async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "learnsley-cancel-"));
  try {
    const compiler = await makeFakeCompiler(dir);
    const source = path.join(dir, "main.sley");
    const childPidFile = path.join(dir, "child.pid");
    await writeFile(source, `${goodCode}\n// TIMEOUT_FORK`, "utf8");
    const controller = new AbortController();
    const promise = testInternals.runSley(compiler, ["check", "--json", source], {
      timeoutMs: 5000,
      signal: controller.signal,
      env: { LEARNSLEY_CHILD_PID_FILE: childPidFile }
    });
    await eventually(async () => {
      const childPid = Number.parseInt(await readFile(childPidFile, "utf8"), 10);
      assert.ok(childPid > 0);
    });
    const childPid = Number.parseInt(await readFile(childPidFile, "utf8"), 10);
    controller.abort();
    const result = await promise;
    assert.equal(result.cancelled, true);
    await eventually(() => assert.throws(() => process.kill(childPid, 0), /ESRCH/));
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

test("stale crash workspaces are pruned within bounded retention", async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "learnsley-prune-"));
  try {
    const oldTime = new Date(Date.now() - 7 * 60 * 60 * 1000);
    for (let index = 0; index < testInternals.staleWorkspaceMaxCount + 3; index += 1) {
      const runDir = path.join(dir, `run-${index}`);
      await mkdir(runDir, { recursive: true });
      await writeFile(path.join(runDir, "main.sley"), "stale", "utf8");
      await utimes(runDir, oldTime, oldTime);
    }
    await testInternals.pruneStaleWorkspaces(dir, { warn() {} });
    assert.equal((await readdir(dir)).length, 0);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
