import { chmodSync, existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(repoRoot, "bin/sley-ci");
const fixtureRoot = mkdtempSync(join(tmpdir(), "sley-ci-test-"));
const fakeSley = join(fixtureRoot, "fake-sley");
const argsLog = join(fixtureRoot, "args.jsonl");
const marker = join(fixtureRoot, "injected-marker");
const manifest = join(fixtureRoot, "manifest.json");

writeFileSync(fakeSley, `#!/usr/bin/env node
import { appendFileSync } from "node:fs";
appendFileSync(process.env.SLEY_TEST_ARGS_LOG, JSON.stringify(process.argv.slice(2)) + "\\n");
process.stdout.write('quote=" slash=\\\\ tab=\\t cr=\\r lf=\\n control=\\u0001');
const shouldFail = process.env.SLEY_TEST_FAIL === "1" || process.env.SLEY_TEST_FAIL_COMMAND === process.argv[2];
process.exit(shouldFail ? 9 : 0);
`);
chmodSync(fakeSley, 0o755);
writeFileSync(manifest, '{"cases":[]}\n');

function run(args, env = {}) {
  return spawnSync("bash", [cli, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      SLEY_BIN: fakeSley,
      SLEY_TEST_ARGS_LOG: argsLog,
      ...env,
    },
  });
}

const help = run(["help"]);
if (help.status !== 0 || !help.stdout.includes("usage: sley-ci")) {
  throw new Error(`help failed: ${help.status} ${help.stderr}`);
}

const hostileTarget = `path with spaces;$(touch ${marker})\n\"quoted\" --help`;
const literal = run(["check", hostileTarget]);
if (literal.status !== 0) throw new Error(`literal target failed: ${literal.stderr}`);
if (existsSync(marker)) throw new Error("action target executed as shell source");
const calls = readFileSync(argsLog, "utf8").trim().split("\n").map((line) => JSON.parse(line));
if (calls.length !== 3 || calls.some((call) => call.at(-1) !== hostileTarget)) {
  throw new Error(`target was not preserved as one literal argument: ${JSON.stringify(calls)}`);
}

const failedJson = run(["--json", "check", "."], { SLEY_TEST_FAIL: "1" });
if (failedJson.status !== 9) throw new Error(`failed check exit mismatch: ${failedJson.status}`);
const failedReport = JSON.parse(failedJson.stdout);
if (failedReport.success || failedReport.status !== 9 || failedReport.results.length !== 3) {
  throw new Error(`failed JSON report mismatch: ${failedJson.stdout}`);
}
if (!failedReport.results.every((entry) => entry.output.includes("control=\u0001"))) {
  throw new Error("control-character output did not survive JSON serialization");
}

for (const failedCommand of ["format", "check", "lint"]) {
  const oneFailure = run(["--json", "check", "."], { SLEY_TEST_FAIL_COMMAND: failedCommand });
  if (oneFailure.status !== 9) throw new Error(`${failedCommand} failure exit mismatch: ${oneFailure.status}`);
  const oneFailureReport = JSON.parse(oneFailure.stdout);
  if (oneFailureReport.results.length !== 3) {
    throw new Error(`${failedCommand} failure prevented the final JSON report`);
  }
}

const missingSley = run(["--json", "smoke", manifest], { SLEY_BIN: "/definitely/missing/sley" });
if (missingSley.status !== 127) throw new Error(`missing Sley passed: ${missingSley.status}`);
const missingSleyReport = JSON.parse(missingSley.stdout);
if (missingSleyReport.success || missingSleyReport.results[0]?.state !== "unavailable") {
  throw new Error(`missing Sley report mismatch: ${missingSley.stdout}`);
}

const missingJq = run(["--json", "smoke", manifest], { JQ_BIN: "/definitely/missing/jq" });
if (missingJq.status !== 127) throw new Error(`missing jq passed: ${missingJq.status}`);
JSON.parse(missingJq.stdout);

const allowedMissing = run(["--json", "--allow-missing", "smoke", manifest], {
  SLEY_BIN: "/definitely/missing/sley",
});
if (allowedMissing.status !== 0) throw new Error(`explicit allow-missing failed: ${allowedMissing.status}`);
const allowedReport = JSON.parse(allowedMissing.stdout);
if (!allowedReport.success || allowedReport.results[0]?.state !== "skipped") {
  throw new Error(`allow-missing report mismatch: ${allowedMissing.stdout}`);
}

const action = readFileSync(join(repoRoot, "action.yml"), "utf8");
const runBlock = action.split(/\n\s*run:\s*\|\s*\n/, 2)[1] ?? "";
if (/\$\{\{\s*inputs\./.test(runBlock)) {
  throw new Error("action input expression remains in the run script body");
}
if (!action.includes("SLEY_CI_TARGET: ${{ inputs.target }}")) {
  throw new Error("action does not transport target through the environment");
}

console.log("sley-ci smoke ok");
