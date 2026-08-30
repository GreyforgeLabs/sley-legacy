#!/usr/bin/env node
import { loadConformance, reportHealth, requireCoverage, requireCoverageThreshold } from "./index.mjs";

class UsageError extends Error {}

function usage() {
  return "usage: sley-conformance <report|coverage|guard> [--root <sley-root>] [options]";
}

function parseArgs(argv) {
  const [command, ...args] = argv;
  if (!command || command === "help" || command === "--help" || command === "-h") {
    return { help: true };
  }
  if (!["report", "coverage", "guard"].includes(command)) {
    throw new UsageError(`unknown command: ${command}`);
  }
  const options = {
    command,
    root: "../sley",
    output: "json",
    requireTag: null,
    minimum: null,
    allowEmpty: false,
  };
  const allowed = new Set(["--root", "--help", "-h", "--test-allow-empty"]);
  if (command === "report") {
    allowed.add("--json");
    allowed.add("--markdown");
  } else {
    allowed.add("--require-tag");
    allowed.add("--minimum");
  }

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (!arg.startsWith("--") && arg !== "-h") throw new UsageError(`unexpected argument: ${arg}`);
    if (!allowed.has(arg)) throw new UsageError(`unknown option for ${command}: ${arg}`);
    if (arg === "--help" || arg === "-h") return { help: true };
    if (arg === "--json") options.output = "json";
    if (arg === "--markdown") options.output = "markdown";
    if (arg === "--test-allow-empty") options.allowEmpty = true;
    if (["--root", "--require-tag", "--minimum"].includes(arg)) {
      const value = args[++index];
      if (value === undefined || value.startsWith("--")) throw new UsageError(`${arg} requires a value`);
      if (arg === "--root") options.root = value;
      if (arg === "--require-tag") options.requireTag = value;
      if (arg === "--minimum") options.minimum = value;
    }
  }

  if (options.allowEmpty && process.env.NODE_ENV !== "test") {
    throw new UsageError("--test-allow-empty requires NODE_ENV=test");
  }
  if (command === "coverage" && !options.requireTag) {
    throw new UsageError("coverage requires --require-tag <tag>");
  }
  if (options.minimum !== null) {
    if (!/^[0-9]+$/.test(options.minimum)) throw new UsageError("--minimum must be an integer >= 1");
    const minimum = Number(options.minimum);
    if (!Number.isSafeInteger(minimum) || minimum < 1) throw new UsageError("--minimum must be an integer >= 1");
    options.minimum = minimum;
    if (!options.requireTag) throw new UsageError("--minimum requires --require-tag");
  }
  return options;
}

function printMarkdown(report, health) {
  const errorLines = report.errors.length
    ? report.errors.map((error) => `- ${error.class}/${error.code}: ${error.path}`).join("\n")
    : "- none";
  console.log(`# Sley Conformance

- health: ${health.ok ? "pass" : "fail"}
- schemas: ${report.schemas.count}
- contracts: ${report.contracts.count}
- smoke cases: ${report.smoke.cases}
- corpus accepted: ${report.corpus.accepted}
- corpus rejected: ${report.corpus.rejected}

## Errors

${errorLines}
`);
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log(usage());
    process.exit(0);
  }
  const report = loadConformance(options.root, { allowEmpty: options.allowEmpty });
  const health = reportHealth(report);

  if (options.command === "report") {
    if (options.output === "markdown") printMarkdown(report, health);
    else console.log(JSON.stringify({ ...report, health }, null, 2));
    process.exit(health.ok ? 0 : 1);
  }

  const checks = [health];
  if (options.requireTag) {
    checks.push(options.minimum === null
      ? requireCoverage(report, options.requireTag)
      : requireCoverageThreshold(report, options.requireTag, options.minimum));
  }
  const ok = checks.every((entry) => entry.ok);
  const schema = options.command === "coverage"
    ? "sley.conformance.coverage_gate.v1"
    : "sley.conformance.guard.v1";
  console.log(JSON.stringify({ schema, ok, checks, errors: report.errors, accounting: report.accounting }, null, 2));
  process.exit(ok ? 0 : 1);
} catch (error) {
  if (error instanceof UsageError || error instanceof RangeError) {
    console.error(`sley-conformance: ${error.message}`);
    console.error(usage());
    process.exit(2);
  }
  console.error(`sley-conformance: ${error instanceof Error ? error.message : "unexpected failure"}`);
  process.exit(2);
}
