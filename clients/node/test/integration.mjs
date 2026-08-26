/** Clean-environment lifecycle proof for the packed Node worker client. */

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { PROTOCOL, WorkerClient, ZERO_DIGEST } from "@sley/worker-client";

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
  client.close();
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
  failedClient.close();
}
console.log("node worker client post-reader-error rejection passed");
