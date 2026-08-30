#!/usr/bin/env node
import { fileURLToPath } from "node:url";
import {
  ContractError,
  compareFixtureSets,
  checkFixtures,
  schemaInventory,
  validateRoot,
} from "./index.mjs";

const COMMANDS = new Set(["inventory", "validate", "check-fixtures", "compare", "help"]);
const VALUE_OPTIONS = new Set(["--schema", "--max-bytes", "--max-depth", "--max-files"]);
const BOOLEAN_OPTIONS = new Set(["--json", "--allow-empty", "--help"]);

function usage(stream = process.stdout) {
  stream.write("usage: sley-contract <inventory|validate|check-fixtures|compare|help> [options]\n");
  stream.write("  inventory [schemaDir] [--allow-empty] [--json]\n");
  stream.write("  validate --schema <schema-id> <report.json> [--json]\n");
  stream.write("  check-fixtures [fixtureDir] [--allow-empty] [--json]\n");
  stream.write("  compare <left-fixtures> <right-fixtures> [--allow-empty] [--json]\n");
  stream.write("  bounds: --max-bytes N --max-depth N --max-files N\n");
}

function usageError(message) {
  throw new ContractError("USAGE_ERROR", message, { exitCode: 2 });
}

export function parseArgs(args) {
  if (args.length === 0) return { command: "help", positional: [], options: {} };
  const command = args[0];
  if (command.startsWith("--")) usageError("command must be the first argument");
  if (!COMMANDS.has(command)) usageError(`unknown command ${command}`);
  const options = {};
  const positional = [];
  for (let index = 1; index < args.length; index += 1) {
    const arg = args[index];
    if (BOOLEAN_OPTIONS.has(arg)) {
      if (options[arg] !== undefined) usageError(`duplicate option ${arg}`);
      options[arg] = true;
      continue;
    }
    if (VALUE_OPTIONS.has(arg)) {
      if (options[arg] !== undefined) usageError(`duplicate option ${arg}`);
      const value = args[index + 1];
      if (value === undefined || value.startsWith("--")) usageError(`${arg} requires a value`);
      options[arg] = value;
      index += 1;
      continue;
    }
    if (arg.startsWith("--")) usageError(`unknown option ${arg}`);
    positional.push(arg);
  }
  if (options["--schema"] !== undefined && command !== "validate") {
    usageError("--schema is valid only with validate");
  }
  if (options["--allow-empty"] && !["inventory", "check-fixtures", "compare"].includes(command)) {
    usageError("--allow-empty is not valid for this command");
  }
  return { command, positional, options };
}

function executionOptions(options) {
  const mapped = { allowEmpty: Boolean(options["--allow-empty"]) };
  for (const [flag, key] of [
    ["--max-bytes", "maxBytes"],
    ["--max-depth", "maxDepth"],
    ["--max-files", "maxFiles"],
  ]) {
    if (options[flag] !== undefined) mapped[key] = Number(options[flag]);
  }
  return mapped;
}

function printJson(value) {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

export function run(args = process.argv.slice(2)) {
  let parsed;
  try {
    parsed = parseArgs(args);
    const { command, positional, options } = parsed;
    const asJson = Boolean(options["--json"]);
    if (command === "help" || options["--help"]) {
      if (positional.length > 0) usageError("help does not accept positional arguments");
      usage();
      return 0;
    }
    const limits = executionOptions(options);
    if (command === "inventory") {
      if (positional.length > 1) usageError("inventory accepts at most one directory");
      const schemas = schemaInventory(positional[0] ?? "docs/schemas", limits);
      if (asJson) printJson({ schema: "sley.contract.inventory.v1", ok: true, count: schemas.length, schemas });
      else {
        process.stdout.write(`schemas: ${schemas.length}\n`);
        for (const schema of schemas) process.stdout.write(`${schema.file}: ${schema.id ?? "(no id)"}\n`);
      }
      return 0;
    }
    if (command === "validate") {
      if (positional.length !== 1 || !options["--schema"]) {
        usageError("validate requires --schema <schema-id> <report.json>");
      }
      const result = validateRoot(options["--schema"], positional[0], limits);
      if (asJson) printJson({ schema: "sley.contract.validation.v1", ...result });
      else process.stdout.write(`ok=${result.ok} schema=${result.expected} actual=${result.actual ?? "(none)"}\n`);
      return result.ok ? 0 : 1;
    }
    if (command === "check-fixtures") {
      if (positional.length > 1) usageError("check-fixtures accepts at most one directory");
      const result = checkFixtures(positional[0] ?? "fixtures/contracts", limits);
      if (asJson) printJson({ schema: "sley.contract.fixture_check.v1", ...result });
      else process.stdout.write(`fixtures: ${result.count} ok=${result.ok}\n`);
      return result.ok ? 0 : 1;
    }
    if (command === "compare") {
      if (positional.length !== 2) usageError("compare requires <left-fixtures> <right-fixtures>");
      const result = compareFixtureSets(positional[0], positional[1], limits);
      if (asJson) printJson({ schema: "sley.contract.fixture_compare.v1", ...result });
      else process.stdout.write(`comparable=${result.comparableCount} ok=${result.ok}\n`);
      return result.ok ? 0 : 1;
    }
    usageError(`unknown command ${command}`);
  } catch (error) {
    const asJson = Boolean(parsed?.options?.["--json"] || args.includes("--json"));
    const known = error instanceof ContractError;
    const payload = {
      schema: "sley.contract.error.v1",
      ok: false,
      error: {
        code: known ? error.contractCode : "INTERNAL_ERROR",
        message: known ? error.message : "unexpected contract-kit failure",
        file: known ? error.file : null,
      },
    };
    if (asJson) printJson(payload);
    else process.stderr.write(`${payload.error.code}: ${payload.error.message}${payload.error.file ? ` (${payload.error.file})` : ""}\n`);
    return known ? error.exitCode : 3;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exitCode = run();
}
