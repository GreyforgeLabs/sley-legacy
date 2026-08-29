/** Clean-environment lifecycle proof for the packed Node worker client. */

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import {
  CONTRACT_VERSIONS,
  OPERATIONS,
  PROTOCOL,
  WorkerClient,
  WorkerClientCancelledError,
  WorkerClientClosedError,
  WorkerClientProtocolError,
  WorkerClientTimeoutError,
  ZERO_DIGEST,
} from "@sley/worker-client";

const root = path.resolve(process.argv[2]);
const fixture = JSON.parse(fs.readFileSync(path.join(root, "fixtures/contracts/worker_request_invoke.json"), "utf8"));
const budgets = fixture.budgets;
const adapter = {
  budgets,
  manifest: fixture.payload.manifest,
  record: fixture.payload.record,
  packageDigest: fixture.bindings.package_digest,
  sourceDigest: fixture.bindings.source_digest,
};

const client = new WorkerClient({
  command: [path.join(root, "bin/sley")],
  cwd: root,
  env: {
    SLEY_ALLOW_TEST_HOOKS: "1",
    SLEY_WORKER_TEST_SNAPSHOT_NONCE: "nonce:node-cancel",
    SLEY_WORKER_TEST_CRASH_NONCE: "nonce:node-crash",
  },
  maxRequests: 8,
  idleTimeoutMs: 120_000,
});

try {
  const ready = await client.start();
  assert.equal(ready.protocol, PROTOCOL);
  assert.equal(ready.code, "WORKER_READY");

  const handshake = await client.handshake(budgets);
  assert.equal(handshake.status, "passed");
  assert.equal(client.workerDigest, handshake.worker.worker_digest);

  const loaded = await client.load(adapter);
  assert.equal(loaded.status, "passed");
  assert.equal(loaded.result.stored_request_state, false);

  const capabilities = await client.capabilities(adapter);
  assert.equal(capabilities.status, "passed");
  assert.deepEqual(capabilities.result.authority, { mode: "local_replay_only", external_provider_calls: false });

  const passed = await client.invoke({ ...adapter, idempotencyKey: "idempotency:node-happy" });
  assert.equal(passed.status, "passed");
  assert.equal(passed.result.request_local_state_retained, false);

  const mismatchRequest = client.buildAdapterRequest("invoke", {
    ...adapter,
    sourceDigest: ZERO_DIGEST,
    idempotencyKey: "idempotency:node-mismatch",
  });
  const mismatch = await client.send(mismatchRequest);
  assert.equal(mismatch.failure_class, "source_mismatch");
  assert.equal(mismatch.retryable, false);

  const cancelRequest = client.buildAdapterRequest("invoke", {
    ...adapter,
    requestId: "request:node-cancel",
    nonce: "nonce:node-cancel",
    idempotencyKey: "idempotency:node-cancel",
  });
  const pendingCancel = client.send(cancelRequest);
  await client.waitForEvent({ code: "WORKER_PROCESS_STARTED", requestId: cancelRequest.request_id, timeoutMs: 15_000 });
  const cancelResponse = await client.cancel(cancelRequest.request_id, budgets);
  const cancelled = await pendingCancel;
  assert.equal(cancelResponse.status, "passed");
  assert.equal(cancelled.failure_class, "cancelled");
  assert.equal(cancelled.retryable, false);

  const crashRequest = client.buildAdapterRequest("invoke", {
    ...adapter,
    requestId: "request:node-crash",
    nonce: "nonce:node-crash",
    idempotencyKey: "idempotency:node-crash",
  });
  const crashed = await client.send(crashRequest);
  assert.equal(crashed.failure_class, "worker_crashed");
  assert.equal(crashed.retryable, true);

  const health = await client.health(budgets);
  assert.equal(health.status, "passed");
  assert.equal(health.worker.request_count, 3);
  assert.deepEqual(health.result, { healthy: true, state: "ready" });

  const shutdown = await client.shutdown(budgets);
  assert.equal(shutdown.status, "passed");
} finally {
  await client.close();
}

assert(client.messages.some((message) => message.failure_class === "worker_crashed"));
assert(client.messages.some((message) => message.failure_class === "cancelled"));
assert(!JSON.stringify(client.messages).includes("plan approved"));
console.log("node worker client clean-environment lifecycle passed");

const fakeScript = String.raw`
const ready = {
  schema: "sley.worker.event.v1",
  protocol: "sley.worker.v1",
  sequence: 1,
  request_id: null,
  operation: "session",
  phase: "ready",
  worker_state: "ready",
  code: "WORKER_READY",
  details: {},
};
const unknown = {
  schema: "sley.worker.response.v1",
  protocol: "sley.worker.v1",
  request_id: "request:unsolicited",
};
console.log(JSON.stringify(ready));
console.log(JSON.stringify(unknown));
setTimeout(() => {}, 30_000);
`;
const failedClient = new WorkerClient({ command: [process.execPath, "-e", fakeScript] });
try {
  await failedClient.start();
  await new Promise((resolve) => setTimeout(resolve, 100));
  const request = failedClient.buildControlRequest("handshake", budgets);
  assert.throws(() => failedClient.send(request), /worker response does not match one pending request/);
} finally {
  await failedClient.close();
}
console.log("node worker client post-reader-error rejection passed");

const fakeCapabilities = {
  protocol: PROTOCOL,
  operations: [...OPERATIONS],
  contract_versions: [...CONTRACT_VERSIONS],
  failure_classes: [
    "adapter_failed", "authority_denied", "budget_exhausted", "cancelled",
    "internal_error", "invalid_result", "protocol_mismatch", "source_mismatch",
    "timeout", "worker_crashed", "worker_unhealthy",
  ],
  request_states: ["accepted", "running", "completed", "failed", "cancelled", "timed_out"],
};
const fakeWorkerScript = String.raw`
const readline = require("node:readline");
const mode = process.env.SLEY_FAKE_MODE;
const protocol = ${JSON.stringify(PROTOCOL)};
const capabilities = ${JSON.stringify(fakeCapabilities)};
const isolation = {
  class: "fresh_subprocess_per_invoke",
  fresh_process_per_invoke: true,
  request_local_tempdir: true,
  clean_environment: true,
  closed_inherited_fds: true,
  resource_limits: true,
  process_group_cancellation: true,
  namespace_isolation_enforced: false,
};
const session = {
  schema: "sley.worker.session.v1",
  protocol,
  worker_id: "worker:fake",
  worker_digest: "sha256:" + "1".repeat(64),
  runtime_digest: "sha256:" + "2".repeat(64),
  state: "ready",
  generation: 1,
  request_count: 0,
  active_request_id: null,
  max_requests: 128,
  cache: { policy: "disabled", shared_mutable_state: false, hit_count: 0 },
  isolation,
};
const ready = {
  schema: "sley.worker.event.v1", protocol, sequence: 1, request_id: null,
  operation: "session", phase: "ready", worker_state: "ready", code: "WORKER_READY", details: {},
};
function response(request) {
  const value = {
    schema: "sley.worker.response.v1", protocol, request_id: request.request_id,
    operation: request.operation, status: "passed", failure_class: null,
    retryable: false, worker: session, result: request.operation === "handshake" ? capabilities : {},
    isolation, issues: [],
  };
  if (mode === "bad_handshake") value.isolation = { ...isolation, process_group_cancellation: false };
  return JSON.stringify(value);
}
console.log(JSON.stringify(ready));
if (mode === "malformed") setTimeout(() => process.stdout.write("{not-json\\n"), 10);
if (mode === "oversized") setTimeout(() => process.stdout.write("x".repeat(4096) + "\\n"), 10);
if (mode === "exit") setTimeout(() => process.exit(17), 20);
if (mode === "exit_pending") setTimeout(() => process.exit(18), 60);
const input = readline.createInterface({ input: process.stdin });
input.on("line", (line) => {
  const request = JSON.parse(line);
  if (mode === "never" || mode === "exit_pending") return;
  if (mode === "late") return setTimeout(() => console.log(response(request)), 80);
  console.log(response(request));
  if (mode === "duplicate") console.log(response(request));
});
`;

function fakeClient(mode, options = {}) {
  return new WorkerClient({
    command: [process.execPath, "-e", fakeWorkerScript],
    env: { SLEY_FAKE_MODE: mode },
    requestTimeoutMs: 1_000,
    ...options,
  });
}

{
  const timeoutClient = fakeClient("late", { requestTimeoutMs: 20 });
  try {
    await timeoutClient.start();
    const first = timeoutClient.buildControlRequest("handshake", budgets, { requestId: "request:late", nonce: "nonce:late" });
    await assert.rejects(timeoutClient.send(first), WorkerClientTimeoutError);
    assert.equal(timeoutClient.pendingRequestCount, 0);
    await new Promise((resolve) => setTimeout(resolve, 120));
    assert.equal(timeoutClient.lateResponseCount, 1);
    const second = timeoutClient.buildControlRequest("handshake", budgets, { requestId: "request:after-late", nonce: "nonce:after-late" });
    assert.equal((await timeoutClient.send(second, { timeoutMs: 200 })).status, "passed");
  } finally {
    await timeoutClient.close();
  }
}

{
  const cancelClient = fakeClient("never");
  try {
    await cancelClient.start();
    const controller = new AbortController();
    const request = cancelClient.buildControlRequest("handshake", budgets);
    const pending = cancelClient.send(request, { signal: controller.signal });
    controller.abort();
    await assert.rejects(pending, WorkerClientCancelledError);
    assert.equal(cancelClient.pendingRequestCount, 0);
  } finally {
    await cancelClient.close();
  }
}

{
  const boundedClient = fakeClient("never", { requestTimeoutMs: 5, maxTombstones: 32 });
  try {
    await boundedClient.start();
    const waits = Array.from({ length: 300 }, (_, index) => {
      const request = boundedClient.buildControlRequest("handshake", budgets, {
        requestId: `request:bounded-${index}`,
        nonce: `nonce:bounded-${index}`,
      });
      return boundedClient.send(request);
    });
    const results = await Promise.allSettled(waits);
    assert(results.every((result) => result.status === "rejected" && result.reason instanceof WorkerClientTimeoutError));
    assert.equal(boundedClient.pendingRequestCount, 0);
    assert.equal(boundedClient.tombstoneCount, 32);
  } finally {
    await boundedClient.close();
  }
}

for (const mode of ["malformed", "oversized", "exit"]) {
  const corruptClient = fakeClient(mode, mode === "oversized" ? { maxMessageBytes: 2048 } : {});
  try {
    await corruptClient.start();
    await new Promise((resolve) => setTimeout(resolve, 100));
    const request = corruptClient.buildControlRequest("handshake", budgets);
    let failure;
    try {
      await corruptClient.send(request, { timeoutMs: 200 });
    } catch (error) {
      failure = error;
    }
    assert(failure instanceof WorkerClientClosedError, `${mode} stream failure was not fatal`);
  } finally {
    await corruptClient.close();
  }
}

{
  const duplicateClient = fakeClient("duplicate");
  try {
    await duplicateClient.start();
    const request = duplicateClient.buildControlRequest("handshake", budgets);
    assert.equal((await duplicateClient.send(request)).status, "passed");
    await new Promise((resolve) => setTimeout(resolve, 100));
    let failure;
    try {
      await duplicateClient.send(duplicateClient.buildControlRequest("handshake", budgets), { timeoutMs: 200 });
    } catch (error) {
      failure = error;
    }
    assert(failure instanceof WorkerClientClosedError, "duplicate response was not fatal");
  } finally {
    await duplicateClient.close();
  }
}

{
  const invalidHandshakeClient = fakeClient("bad_handshake");
  try {
    await invalidHandshakeClient.start();
    await assert.rejects(invalidHandshakeClient.handshake(budgets), WorkerClientClosedError);
  } finally {
    await invalidHandshakeClient.close();
  }
}

{
  const exitClient = fakeClient("exit_pending");
  try {
    await exitClient.start();
    const waits = [0, 1].map((index) => exitClient.send(exitClient.buildControlRequest("handshake", budgets, {
      requestId: `request:exit-${index}`,
      nonce: `nonce:exit-${index}`,
    })));
    const results = await Promise.allSettled(waits);
    assert(results.every((result) => result.status === "rejected" && result.reason instanceof WorkerClientClosedError));
    assert.equal(exitClient.pendingRequestCount, 0);
  } finally {
    await exitClient.close();
  }
}

{
  const outboundClient = fakeClient("never", { maxMessageBytes: 2048 });
  try {
    await outboundClient.start();
    const request = outboundClient.buildControlRequest("handshake", budgets);
    request.payload = { oversized: "x".repeat(4096) };
    assert.throws(() => outboundClient.send(request), WorkerClientProtocolError);
  } finally {
    await outboundClient.close();
  }
}

console.log("node worker client adversarial lifecycle matrix passed");
