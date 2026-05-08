#!/usr/bin/env node

const args = process.argv.slice(2);
const has = (flag) => args.includes(flag);

function usage() {
  return `Sley self-hosted CLI scaffold\n\nUsage:\n  node self-hosted/cli.mjs [global options] <command> [command args]\n\nCommands:\n  doctor [--json]\n  ast [--json] <path>\n  check [--json] <path>\n\nGlobal:\n  --help     show this message\n  --version  show CLI version\n\nThis is a migration scaffold for the self-hosted transition.`;
}

function printJson(value) {
  process.stdout.write(JSON.stringify(value, null, 2));
}

const command = args[0];

if (has("--help") || args.length === 0) {
  process.stdout.write(usage() + "\n");
  process.exit(0);
}

if (has("--version")) {
  process.stdout.write("sley 0.1.0-self-hosted-skeleton\n");
  process.exit(0);
}

const jsonMode = has("--json");

function payload(label, file) {
  const base = {
    schema: "sley.cli.skeleton.v0",
    command,
    file: file || null,
    status: "ok",
    notes: [
      "Self-hosted migration scaffold output.",
      "Replace this with parity implementation before release-blocker lift.",
    ],
  };
  if (!label) return base;
  return { ...base, label };
}

if (command === "doctor") {
  const out = payload("doctor-surface");
  if (jsonMode) {
    printJson(out);
    process.stdout.write("\n");
  } else {
    process.stdout.write("Doctor: ok (self-hosted scaffold)\n");
  }
  process.exit(0);
}

if (command === "ast") {
  const file = args.find((arg) => !arg.startsWith("--") && arg !== command);
  const out = {
    schema: "sley.ast.program.v0",
    command: "ast",
    file: file || null,
    status: "todo",
    warnings: ["parser implementation pending migration"],
  };
  if (jsonMode) {
    printJson(out);
    process.stdout.write("\n");
  } else {
    process.stdout.write("AST scaffold ready. JSON mode required for machine checks.\n");
  }
  process.exit(0);
}

if (command === "check") {
  const file = args.find((arg) => !arg.startsWith("--") && arg !== command);
  const out = {
    schema: "sley.check.report.v0",
    command: "check",
    file: file || null,
    status: "todo",
    errors: [],
    warnings: ["full checker migration pending"],
  };
  if (jsonMode) {
    printJson(out);
    process.stdout.write("\n");
  } else {
    process.stdout.write("Check scaffold ready. JSON mode recommended.\n");
  }
  process.exit(0);
}

process.stderr.write(`Unknown command: ${command}. Use --help.\n`);
process.exit(2);
