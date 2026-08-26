import {
  PROTOCOL,
  WorkerClient,
  type WorkerEvent,
  type WorkerRequest,
  type WorkerRequestBudgets,
  type WorkerResponse,
} from "@sley/worker-client";

const budgets: WorkerRequestBudgets = {
  wall_clock_ms: 120_000,
  max_steps: 1_000,
  max_call_depth: 16,
  max_collection_items: 10_000,
  max_output_bytes: 1_048_576,
  max_memory_bytes: 536_870_912,
};

const client = new WorkerClient({ command: ["sley"] });
const request: WorkerRequest = client.buildControlRequest("handshake", budgets);
const response: Promise<WorkerResponse> = client.send(request);
const event: Promise<WorkerEvent> = client.waitForEvent({ code: "WORKER_READY" });

void PROTOCOL;
void response;
void event;
