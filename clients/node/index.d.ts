import type {
  WorkerEvent,
  WorkerRequest,
  WorkerRequestBudgets,
  WorkerResponse,
} from "./generated/models.js";

export * from "./generated/models.js";

export declare const DEFAULT_MAX_MESSAGE_BYTES: number;
export declare const DEFAULT_MAX_RETAINED_MESSAGES: number;
export declare const DEFAULT_REQUEST_TIMEOUT_MS: number;
export declare const DEFAULT_MAX_TOMBSTONES: number;

export declare class WorkerClientError extends Error {}
export declare class WorkerClientClosedError extends WorkerClientError {}
export declare class WorkerClientProtocolError extends WorkerClientError {}
export declare class WorkerClientTimeoutError extends WorkerClientError {}
export declare class WorkerClientCancelledError extends WorkerClientError {}

export interface WorkerClientOptions {
  command?: readonly string[];
  cwd?: string;
  env?: Readonly<Record<string, string>>;
  maxRequests?: number;
  idleTimeoutMs?: number;
  maxMessageBytes?: number;
  maxRetainedMessages?: number;
  requestTimeoutMs?: number;
  maxTombstones?: number;
}

export interface ControlRequestOptions {
  targetRequestId?: string;
  requestId?: string;
  nonce?: string;
}

export interface AdapterRequestOptions {
  budgets: WorkerRequestBudgets;
  manifest: string;
  packageDigest: string;
  sourceDigest: string;
  record?: string | null;
  idempotencyKey?: string;
  requestId?: string;
  nonce?: string;
}

export interface EventWaitOptions {
  code?: string;
  requestId?: string;
  predicate?: (event: WorkerEvent) => boolean;
  timeoutMs?: number;
}

export interface SendOptions {
  timeoutMs?: number;
  signal?: AbortSignal;
}

export declare class WorkerClient {
  constructor(options?: WorkerClientOptions);
  readonly messages: ReadonlyArray<WorkerEvent | WorkerResponse>;
  readonly pendingRequestCount: number;
  readonly tombstoneCount: number;
  readonly lateResponseCount: number;
  stderr: string;
  workerDigest: string | null;
  runtimeDigest: string | null;
  start(options?: { timeoutMs?: number }): Promise<WorkerEvent>;
  send(request: WorkerRequest, options?: SendOptions): Promise<WorkerResponse>;
  waitForEvent(options?: EventWaitOptions): Promise<WorkerEvent>;
  static identity(prefix: "request" | "nonce" | "idempotency" | string): string;
  buildControlRequest(
    operation: "handshake" | "cancel" | "reset" | "drain" | "health" | "shutdown",
    budgets: WorkerRequestBudgets,
    options?: ControlRequestOptions,
  ): WorkerRequest;
  buildAdapterRequest(
    operation: "load" | "capabilities" | "invoke",
    options: AdapterRequestOptions,
  ): WorkerRequest;
  handshake(budgets: WorkerRequestBudgets): Promise<WorkerResponse>;
  load(options: AdapterRequestOptions): Promise<WorkerResponse>;
  capabilities(options: AdapterRequestOptions): Promise<WorkerResponse>;
  invoke(options: AdapterRequestOptions): Promise<WorkerResponse>;
  cancel(targetRequestId: string, budgets: WorkerRequestBudgets): Promise<WorkerResponse>;
  reset(budgets: WorkerRequestBudgets): Promise<WorkerResponse>;
  drain(budgets: WorkerRequestBudgets): Promise<WorkerResponse>;
  health(budgets: WorkerRequestBudgets): Promise<WorkerResponse>;
  shutdown(budgets: WorkerRequestBudgets, options?: { timeoutMs?: number }): Promise<WorkerResponse>;
  waitForExit(timeoutMs?: number): Promise<number | null>;
  close(options?: { timeoutMs?: number }): Promise<void>;
}
