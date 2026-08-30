import {
  chmodSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(repoRoot, "bin/sley-audit");

function run(args, { cwd = repoRoot, env = {} } = {}) {
  return spawnSync("bash", [cli, ...args], {
    cwd,
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

function cleanFixture(prefix = "sley audit clean ") {
  const root = mkdtempSync(join(tmpdir(), prefix));
  mkdirSync(join(root, "src"), { recursive: true });
  writeFileSync(join(root, "src/example.rs"), 'fn main() { println!("safe"); }\n');
  return root;
}

const noArgsRoot = cleanFixture();
const noArgs = run([], { cwd: noArgsRoot });
if (noArgs.status !== 0 || !noArgs.stdout.includes("1/1 eligible file(s) read")) {
  throw new Error(`no-argument scan failed: ${noArgs.status} ${noArgs.stdout} ${noArgs.stderr}`);
}

const findingRoot = cleanFixture("sley audit finding ");
const dangerous = "println!" + '("{:?}", secret);\n';
writeFileSync(join(findingRoot, "src/leak.rs"), dangerous);
const finding = run(["--json", "--path", findingRoot]);
if (finding.status !== 1) throw new Error(`high finding did not fail: ${finding.status}`);
const findingReport = JSON.parse(finding.stdout);
if (findingReport.schema !== "sley.audit.report.v1" || findingReport.blockingFindings < 1) {
  throw new Error("finding report schema or severity mismatch");
}
if (findingReport.findings.some((entry) => entry.path.startsWith("/") || entry.path.includes(tmpdir()))) {
  throw new Error("machine report exposed an absolute local path");
}
if (!findingReport.findings.every((entry) => /^[a-f0-9]{64}$/.test(entry.fingerprint))) {
  throw new Error("finding fingerprint is not SHA-256");
}

const reportOnly = run(["--json", "--report-only", "--path", findingRoot]);
if (reportOnly.status !== 0 || JSON.parse(reportOnly.stdout).status !== "fail") {
  throw new Error("explicit report-only mode did not preserve finding status");
}

const emptyRoot = mkdtempSync(join(tmpdir(), "sley-audit-empty-"));
const empty = run(["--json", "--path", emptyRoot]);
if (empty.status !== 2 || JSON.parse(empty.stdout).coverageComplete) {
  throw new Error("empty audit passed without --allow-empty");
}
const allowedEmpty = run(["--json", "--allow-empty", "--path", emptyRoot]);
if (allowedEmpty.status !== 0) throw new Error("explicit --allow-empty failed");

const unreadableRoot = cleanFixture("sley audit unreadable ");
const unreadableFile = join(unreadableRoot, "src/unreadable.rs");
writeFileSync(unreadableFile, "fn unreadable() {}\n");
chmodSync(unreadableFile, 0o000);
const unreadable = run(["--json", "--path", unreadableRoot]);
chmodSync(unreadableFile, 0o600);
if (unreadable.status !== 2 || !JSON.parse(unreadable.stdout).omissions.some((item) => item.reason === "file_unreadable")) {
  throw new Error("unreadable mandatory file did not fail coverage");
}

const symlinkRoot = cleanFixture("sley audit symlink ");
const outside = cleanFixture("sley audit outside ");
symlinkSync(join(outside, "src/example.rs"), join(symlinkRoot, "src/escape.rs"));
const symlink = run(["--json", "--path", symlinkRoot]);
if (symlink.status !== 2 || !JSON.parse(symlink.stdout).omissions.some((item) => item.reason === "symlink_rejected")) {
  throw new Error("symlink escape did not fail coverage");
}

const specialRoot = cleanFixture("sley audit special ");
const fifoPath = join(specialRoot, "src/input.rs");
const fifo = spawnSync("mkfifo", [fifoPath], { encoding: "utf8" });
if (fifo.status === 0) {
  const special = run(["--json", "--path", specialRoot]);
  if (special.status !== 2 || !JSON.parse(special.stdout).omissions.some((item) => item.reason === "special_file_rejected")) {
    throw new Error("special file did not fail coverage");
  }
}

const invalidUtf8Root = cleanFixture("sley audit utf8 ");
writeFileSync(join(invalidUtf8Root, "src/invalid.rs"), Buffer.from([0xc3, 0x28]));
const invalidUtf8 = run(["--json", "--path", invalidUtf8Root]);
if (invalidUtf8.status !== 2 || !JSON.parse(invalidUtf8.stdout).omissions.some((item) => item.reason === "invalid_utf8")) {
  throw new Error("invalid UTF-8 did not fail coverage");
}

const boundedRoot = cleanFixture("sley audit bounded ");
writeFileSync(join(boundedRoot, "src/large.rs"), "x".repeat(64));
const oversized = run(["--json", "--max-file-bytes", "8", "--path", boundedRoot]);
if (oversized.status !== 2 || JSON.parse(oversized.stdout).coverage.oversized < 1) {
  throw new Error("oversized file did not fail coverage");
}
const tooMany = run(["--json", "--max-files", "1", "--path", boundedRoot]);
if (tooMany.status !== 2 || !JSON.parse(tooMany.stdout).coverage.truncated) {
  throw new Error("file-count limit did not stop traversal");
}

const firstStableRoot = mkdtempSync(join(tmpdir(), "sley-audit-stable-a-"));
const secondStableRoot = mkdtempSync(join(tmpdir(), "sley-audit-stable-b-"));
for (const stableRoot of [firstStableRoot, secondStableRoot]) {
  mkdirSync(join(stableRoot, "src"));
  writeFileSync(join(stableRoot, "src/leak.rs"), dangerous);
}
const firstStable = JSON.parse(run(["--json", "--path", firstStableRoot]).stdout);
const secondStable = JSON.parse(run(["--json", "--path", secondStableRoot]).stdout);
if (firstStable.findings[0].fingerprint !== secondStable.findings[0].fingerprint) {
  throw new Error("equivalent findings were not stable across roots");
}

const sarifPath = join(findingRoot, "audit result.sarif");
const sarifRun = run(["--report-only", "--sarif", sarifPath, "--path", findingRoot]);
if (sarifRun.status !== 0) throw new Error("SARIF generation failed");
const sarif = JSON.parse(readFileSync(sarifPath, "utf8"));
if (sarif.runs[0].tool.driver.semanticVersion !== "0.2.0") throw new Error("SARIF version drift");

const missing = run(["--json", "--path", join(emptyRoot, "missing")]);
if (missing.status !== 2 || !missing.stderr.includes("does not exist")) {
  throw new Error("missing target did not produce a stable usage error");
}

const action = readFileSync(join(repoRoot, "action.yml"), "utf8");
if (!action.startsWith("name: Sley Audit\n") || !action.includes("using: composite")) {
  throw new Error("action metadata is not a valid composite action shape");
}
const runBlock = action.split(/\n\s*run:\s*\|\s*\n/, 2)[1] ?? "";
if (/\$\{\{\s*inputs\./.test(runBlock)) throw new Error("action input expression appears in shell source");
if (!action.includes("SLEY_AUDIT_TARGET: ${{ inputs.target }}")) {
  throw new Error("action target is not transported through the environment");
}

console.log("sley-audit smoke ok");
