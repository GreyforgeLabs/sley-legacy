/** Reference Node client for the canonical sley.worker.v1 stdio protocol. */

import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { TextDecoder } from "node:util";
import {
  CONTRACT_VERSIONS,
  OPERATIONS,
  PROTOCOL,
  SCHEMA_IDS,
  ZERO_DIGEST,
} from "./generated/constants.mjs";

export { CONTRACT_VERSIONS, OPERATIONS, PROTOCOL, SCHEMA_IDS, ZERO_DIGEST };

export const DEFAULT_MAX_MESSAGE_BYTES = 20 * 1024 * 1024;
export const DEFAULT_MAX_RETAINED_MESSAGES = 8192;

export class WorkerClientError extends Error {}
export class WorkerClientClosedError extends WorkerClientError {}
export class WorkerClientProtocolError extends WorkerClientError {}

export class WorkerClient {
  constructor({
    command = ["sley"],
    cwd,
    env = {},
    maxRequests = 128,
    idleTimeoutMs = 300_000,
    maxMessageBytes = DEFAULT_MAX_MESSAGE_BYTES,
    maxRetainedMessages = DEFAULT_MAX_RETAINED_MESSAGES,
  } = {}) {
    if (!Array.isArray(command) || command.length === 0) {
      throw new TypeError("command must be a non-empty array");
    }
    if (!Number.isInteger(maxRequests) || maxRequests < 1 || maxRequests > 1_000_000) {
      throw new RangeError("maxRequests must be between 1 and 1000000");
    }
    if (!Number.isInteger(idleTimeoutMs) || idleTimeoutMs < 0 || idleTimeoutMs > 86_400_000) {
      throw new RangeError("idleTimeoutMs must be between 0 and 86400000");
    }
    if (!Number.isInteger(maxMessageBytes) || maxMessageBytes < 1 || !Number.isInteger(maxRetainedMessages) || maxRetainedMessages < 1) {
      throw new RangeError("client stream bounds must be positive integers");
    }
    this.command = [...command];
    this.cwd = cwd;
    this.extraEnv = { ...env };
    this.maxRequests = maxRequests;
    this.idleTimeoutMs = idleTimeoutMs;
    this.maxMessageBytes = maxMessageBytes;
    this.maxRetainedMessages = maxRetainedMessages;
    this.process = null;
    this.workerDigest = null;
    this.runtimeDigest = null;
    this._messages = [];
    this.stderr = "";
    this.pending = new Map();
    this.eventWaiters = new Set();
    this.buffer = Buffer.alloc(0);
    this.lastEventSequence = -1;
    this.readerError = null;
    this.decoder = new TextDecoder("utf-8", { fatal: true });
  }

  get messages() {
    return this._messages.slice();
  }

  async start({ timeoutMs = 15_000 } = {}) {
    if (this.process !== null) {
      throw new WorkerClientError("worker client is already started");
    }
    const [executable, ...prefix] = this.command;
    const argv = [
      ...prefix,
      "worker",
      "start",
      "--json",
      "--max-requests",
      String(this.maxRequests),
      "--idle-timeout-ms",
      String(this.idleTimeoutMs),
    ];
    this.process = spawn(executable, argv, {
      cwd: this.cwd,
      env: { ...process.env, ...this.extraEnv },
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    this.process.stdout.on("data", (chunk) => this.#readChunk(chunk));
    this.process.stdout.on("end", () => {
      if (this.buffer.length !== 0) {
        this.#failReader(new WorkerClientProtocolError("worker output ended with an incomplete JSONL message"));
      }
    });
    this.process.stderr.on("data", (chunk) => {
      if (this.stderr.length < 65_536) {
        this.stderr += chunk.toString("utf8", 0, Math.max(0, 65_536 - this.stderr.length));
      }
    });
    this.process.on("error", (error) => this.#failReader(error));
    this.process.on("close", () => {
      if ([...this.pending.values()].some((entry) => entry.settled === false)) {
        this.#failReader(new WorkerClientClosedError("worker output closed before every response arrived"));
      }
    });
    return this.waitForEvent({ code: "WORKER_READY", timeoutMs });
  }

  #readChunk(chunk) {
    if (this.readerError !== null) return;
    this.buffer = Buffer.concat([this.buffer, chunk]);
    try {
      while (true) {
        const newline = this.buffer.indexOf(0x0a);
        if (newline < 0) {
          if (this.buffer.length > this.maxMessageBytes) {
            throw new WorkerClientProtocolError("worker message exceeds the client byte bound");
          }
          return;
        }
        if (newline > this.maxMessageBytes) {
          throw new WorkerClientProtocolError("worker message exceeds the client byte bound");
        }
        const line = this.buffer.subarray(0, newline);
        this.buffer = this.buffer.subarray(newline + 1);
        let value;
        try {
          value = JSON.parse(this.decoder.decode(line));
        } catch (error) {
          throw new WorkerClientProtocolError(`worker emitted invalid UTF-8 JSON: ${error.message}`);
        }
        this.#acceptMessage(value);
      }
    } catch (error) {
      this.#failReader(error);
    }
  }

  #acceptMessage(value) {
    if (value === null || typeof value !== "object" || Array.isArray(value) || value.protocol !== PROTOCOL) {
      throw new WorkerClientProtocolError("worker emitted a message outside sley.worker.v1");
    }
    if (value.schema !== "sley.worker.event.v1" && value.schema !== "sley.worker.response.v1") {
      throw new WorkerClientProtocolError("worker emitted an unknown stream schema");
    }
    if (this._messages.length >= this.maxRetainedMessages) {
      throw new WorkerClientProtocolError("worker stream exceeded the configured retained-message bound");
    }
    if (value.schema === "sley.worker.event.v1") {
      if (!Number.isInteger(value.sequence) || value.sequence <= this.lastEventSequence) {
        throw new WorkerClientProtocolError("worker event sequence is not strictly increasing");
      }
      this.lastEventSequence = value.sequence;
    }
    this._messages.push(value);
    if (value.schema === "sley.worker.response.v1" && typeof value.request_id === "string") {
      const pending = this.pending.get(value.request_id);
      if (!pending || pending.settled !== false) {
        throw new WorkerClientProtocolError("worker response does not match one pending request");
      }
      pending.settled = true;
      this.pending.delete(value.request_id);
      pending.resolve(value);
    } else if (value.schema === "sley.worker.response.v1") {
      throw new WorkerClientProtocolError("worker response does not match one pending request");
    }
    if (value.schema === "sley.worker.event.v1") {
      for (const waiter of [...this.eventWaiters]) {
        if (waiter.predicate(value)) {
          clearTimeout(waiter.timer);
          this.eventWaiters.delete(waiter);
          waiter.resolve(value);
        }
      }
    }
  }

  #failReader(error) {
    if (this.readerError !== null) return;
    this.readerError = error instanceof Error ? error : new WorkerClientError(String(error));
    for (const [requestId, pending] of this.pending) {
      if (pending.settled === false) {
        pending.settled = true;
        pending.reject(new WorkerClientClosedError(`${requestId}: ${this.readerError.message}`));
      }
    }
    this.pending.clear();
    for (const waiter of this.eventWaiters) {
      clearTimeout(waiter.timer);
      waiter.reject(new WorkerClientClosedError(this.readerError.message));
    }
    this.eventWaiters.clear();
  }

  send(request) {
    if (request?.schema !== "sley.worker.request.v1" || request?.protocol !== PROTOCOL) {
      throw new WorkerClientProtocolError("request does not use the canonical worker envelope");
    }
    if (typeof request.request_id !== "string" || !/^request:[A-Za-z0-9._-]+$/.test(request.request_id)) {
      throw new WorkerClientProtocolError("request_id is not a canonical worker request identity");
    }
    if (typeof request.nonce !== "string" || !/^nonce:[A-Za-z0-9._-]+$/.test(request.nonce)) {
      throw new WorkerClientProtocolError("nonce is not a canonical worker request nonce");
    }
    if (!OPERATIONS.includes(request.operation)) {
      throw new WorkerClientProtocolError("request operation is outside the canonical worker contract");
    }
    if (this.readerError !== null) {
      throw new WorkerClientClosedError(this.readerError.message);
    }
    if (this.process === null || this.process.exitCode !== null || !this.process.stdin.writable) {
      throw new WorkerClientClosedError("worker process is not running");
    }
    if (this.pending.has(request.request_id)) {
      throw new WorkerClientProtocolError("request_id is already pending in this client");
    }
    const encoded = JSON.stringify(request) + "\n";
    return new Promise((resolve, reject) => {
      const entry = { resolve, reject, settled: false };
      this.pending.set(request.request_id, entry);
      this.process.stdin.write(encoded, "utf8", (error) => {
        if (error && entry.settled === false) {
          entry.settled = true;
          this.pending.delete(request.request_id);
          reject(error);
        }
      });
    });
  }

  waitForEvent({ code, requestId, predicate, timeoutMs = 15_000 } = {}) {
    const matches = (event) =>
      (code === undefined || event.code === code) &&
      (requestId === undefined || event.request_id === requestId) &&
      (predicate === undefined || predicate(event));
    const existing = this._messages.find((message) => message.schema === "sley.worker.event.v1" && matches(message));
    if (existing) return Promise.resolve(existing);
    if (this.readerError !== null) return Promise.reject(new WorkerClientClosedError(this.readerError.message));
    return new Promise((resolve, reject) => {
      const waiter = { predicate: matches, resolve, reject, timer: null };
      waiter.timer = setTimeout(() => {
        this.eventWaiters.delete(waiter);
        reject(new Error("timed out waiting for a worker event"));
      }, timeoutMs);
      this.eventWaiters.add(waiter);
    });
  }

  static identity(prefix) {
    return `${prefix}:${randomBytes(12).toString("hex")}`;
  }

  #boundDigests() {
    if (this.workerDigest === null || this.runtimeDigest === null) {
      throw new WorkerClientProtocolError("handshake must pass before bound worker requests");
    }
    return [this.workerDigest, this.runtimeDigest];
  }

  buildControlRequest(operation, budgets, { targetRequestId, requestId, nonce } = {}) {
    if (!["handshake", "cancel", "reset", "drain", "health", "shutdown"].includes(operation)) {
      throw new TypeError("operation is not a worker control operation");
    }
    const [workerDigest, runtimeDigest] = operation === "handshake" ? [ZERO_DIGEST, ZERO_DIGEST] : this.#boundDigests();
    let payload = {};
    if (operation === "cancel") {
      if (targetRequestId === undefined) throw new TypeError("cancel requires targetRequestId");
      payload = { target_request_id: targetRequestId };
    } else if (targetRequestId !== undefined) {
      throw new TypeError("targetRequestId is accepted only by cancel");
    }
    return {
      schema: "sley.worker.request.v1",
      protocol: PROTOCOL,
      request_id: requestId ?? WorkerClient.identity("request"),
      operation,
      nonce: nonce ?? WorkerClient.identity("nonce"),
      bindings: {
        worker_digest: workerDigest,
        runtime_digest: runtimeDigest,
        package_digest: ZERO_DIGEST,
        source_digest: ZERO_DIGEST,
        contract_versions: [...CONTRACT_VERSIONS],
        entry_point: `worker.${operation}`,
        authority: { mode: "absent", grant_digest: null, idempotency_key: null },
      },
      budgets: { ...budgets },
      payload,
    };
  }

  buildAdapterRequest(operation, {
    budgets,
    manifest,
    packageDigest,
    sourceDigest,
    record = null,
    idempotencyKey,
    requestId,
    nonce,
  }) {
    if (!["load", "capabilities", "invoke"].includes(operation)) {
      throw new TypeError("operation is not an adapter worker operation");
    }
    const [workerDigest, runtimeDigest] = this.#boundDigests();
    if (operation === "invoke") {
      idempotencyKey ??= WorkerClient.identity("idempotency");
    } else if (idempotencyKey !== undefined) {
      throw new TypeError("idempotencyKey is accepted only by invoke");
    }
    return {
      schema: "sley.worker.request.v1",
      protocol: PROTOCOL,
      request_id: requestId ?? WorkerClient.identity("request"),
      operation,
      nonce: nonce ?? WorkerClient.identity("nonce"),
      bindings: {
        worker_digest: workerDigest,
        runtime_digest: runtimeDigest,
        package_digest: packageDigest,
        source_digest: sourceDigest,
        contract_versions: [...CONTRACT_VERSIONS],
        entry_point: "adapter.replay",
        authority: {
          mode: "local_replay_only",
          grant_digest: null,
          idempotency_key: idempotencyKey ?? null,
        },
      },
      budgets: { ...budgets },
      payload: { manifest, record, target_request_id: null },
    };
  }

  async handshake(budgets) {
    const response = await this.send(this.buildControlRequest("handshake", budgets));
    if (response.status === "passed") {
      if (response.result?.protocol !== PROTOCOL || JSON.stringify(response.result.contract_versions) !== JSON.stringify(CONTRACT_VERSIONS)) {
        throw new WorkerClientProtocolError("handshake response disagrees with canonical client contracts");
      }
      this.workerDigest = response.worker.worker_digest;
      this.runtimeDigest = response.worker.runtime_digest;
    }
    return response;
  }

  load(options) { return this.send(this.buildAdapterRequest("load", options)); }
  capabilities(options) { return this.send(this.buildAdapterRequest("capabilities", options)); }
  invoke(options) { return this.send(this.buildAdapterRequest("invoke", options)); }
  cancel(targetRequestId, budgets) { return this.send(this.buildControlRequest("cancel", budgets, { targetRequestId })); }
  reset(budgets) { return this.send(this.buildControlRequest("reset", budgets)); }
  drain(budgets) { return this.send(this.buildControlRequest("drain", budgets)); }
  health(budgets) { return this.send(this.buildControlRequest("health", budgets)); }

  async shutdown(budgets, { timeoutMs = 15_000 } = {}) {
    const stopped = this.waitForEvent({ code: "WORKER_STOPPED", timeoutMs });
    const response = await this.send(this.buildControlRequest("shutdown", budgets));
    await stopped;
    await this.waitForExit(timeoutMs);
    return response;
  }

  waitForExit(timeoutMs = 15_000) {
    if (this.process === null || this.process.exitCode !== null) return Promise.resolve(this.process?.exitCode ?? null);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("timed out waiting for worker exit")), timeoutMs);
      this.process.once("close", (code) => {
        clearTimeout(timer);
        resolve(code);
      });
    });
  }

  close() {
    if (this.process !== null && this.process.exitCode === null) this.process.kill("SIGTERM");
  }
}
