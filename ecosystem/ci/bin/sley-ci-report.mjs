#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
let command = "unknown";
let status = 2;
const results = [];

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === "--command") {
    command = args[++index] ?? "unknown";
  } else if (arg === "--status") {
    status = Number.parseInt(args[++index] ?? "2", 10);
  } else if (arg === "--result") {
    const label = args[++index] ?? "unknown";
    const resultStatus = Number.parseInt(args[++index] ?? "2", 10);
    const state = args[++index] ?? "failed";
    const outputFile = args[++index];
    results.push({
      command: label,
      status: Number.isInteger(resultStatus) ? resultStatus : 2,
      state,
      output: outputFile ? readFileSync(outputFile, "utf8") : "",
    });
  } else {
    throw new Error(`unknown reporter option: ${arg}`);
  }
}

const normalizedStatus = Number.isInteger(status) ? status : 2;
process.stdout.write(`${JSON.stringify({
  schema: "sley.ci.report.v1",
  command,
  success: normalizedStatus === 0,
  status: normalizedStatus,
  results,
})}\n`);
