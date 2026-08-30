import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { loadConformance, reportHealth, requireCoverage, requireCoverageThreshold } from "../src/index.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(repoRoot, "src/cli.mjs");

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "sley-conf-"));
  mkdirSync(join(root, "fixtures/corpus"), { recursive: true });
  mkdirSync(join(root, "fixtures/cli_smokes"), { recursive: true });
  mkdirSync(join(root, "fixtures/contracts"), { recursive: true });
  mkdirSync(join(root, "docs/schemas"), { recursive: true });
  writeFileSync(join(root, "fixtures/corpus/manifest.json"), JSON.stringify({
    accepted: [{ id: "accepted-1" }],
    rejected: [{ id: "rejected-1" }],
  }));
  writeFileSync(join(root, "fixtures/cli_smokes/manifest.json"), JSON.stringify({
    cases: [{ id: "smoke-1", covers: ["cli:verify"] }],
  }));
  writeFileSync(join(root, "fixtures/contracts/a.json"), JSON.stringify({ id: "contract-1", schema: "schema:a" }));
  writeFileSync(join(root, "docs/schemas/a.schema.json"), JSON.stringify({ $id: "schema:a" }));
  return root;
}

function run(args, env = {}) {
  return spawnSync(process.execPath, [cli, ...args], {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

const validRoot = fixture();
const validReport = loadConformance(validRoot);
if (!reportHealth(validReport).ok) throw new Error(`valid fixture failed: ${JSON.stringify(validReport.errors)}`);
if (!requireCoverage(validReport, "cli:verify").ok) throw new Error("missing coverage");
if (!requireCoverageThreshold(validReport, "cli:verify", 1).ok) throw new Error("threshold failed");

const malformedRoot = fixture();
writeFileSync(join(malformedRoot, "fixtures/contracts/malformed.json"), "{malformed");
const malformed = run(["guard", "--root", malformedRoot]);
if (malformed.status !== 1) throw new Error(`malformed contract passed: ${malformed.stdout} ${malformed.stderr}`);
const malformedResult = JSON.parse(malformed.stdout);
if (!malformedResult.errors.some((error) => error.code === "malformed_json")) {
  throw new Error("malformed contract was not accounted for");
}

const duplicateRoot = fixture();
writeFileSync(join(duplicateRoot, "docs/schemas/duplicate.schema.json"), JSON.stringify({ $id: "schema:a" }));
writeFileSync(join(duplicateRoot, "fixtures/contracts/duplicate.json"), JSON.stringify({
  id: "contract-1",
  schema: "schema:a",
}));
const duplicate = run(["guard", "--root", duplicateRoot]);
if (duplicate.status !== 1) throw new Error("duplicate identifiers passed");
const duplicateResult = JSON.parse(duplicate.stdout);
if (duplicateResult.errors.filter((error) => error.code === "duplicate_identifier").length < 2) {
  throw new Error("duplicate schema and contract IDs were not both reported");
}

const emptyRoot = mkdtempSync(join(tmpdir(), "sley-conf-empty-"));
for (const directory of ["fixtures/corpus", "fixtures/cli_smokes", "fixtures/contracts", "docs/schemas"]) {
  mkdirSync(join(emptyRoot, directory), { recursive: true });
}
writeFileSync(join(emptyRoot, "fixtures/corpus/manifest.json"), '{"accepted":[],"rejected":[]}');
writeFileSync(join(emptyRoot, "fixtures/cli_smokes/manifest.json"), '{"cases":[]}');
const empty = run(["report", "--root", emptyRoot]);
if (empty.status !== 1) throw new Error("empty mandatory evidence passed");

for (const invalid of ["0", "-1", "1.5", "NaN", "Infinity", ""]) {
  const args = ["coverage", "--root", validRoot, "--require-tag", "cli:verify", "--minimum"];
  if (invalid !== "") args.push(invalid);
  const threshold = run(args);
  if (threshold.status !== 2 || threshold.stderr.includes("Error:")) {
    throw new Error(`invalid threshold did not produce stable usage error: ${invalid}`);
  }
}

for (const args of [
  ["report", "--root"],
  ["report", "--unknown"],
  ["coverage", "--root", validRoot],
]) {
  const result = run(args);
  if (result.status !== 2 || result.stderr.includes("at file:")) {
    throw new Error(`invalid CLI input did not fail cleanly: ${JSON.stringify(args)}`);
  }
}

const markdown = run(["report", "--root", validRoot, "--markdown"]);
if (markdown.status !== 0 || !markdown.stdout.startsWith("# Sley Conformance")) {
  throw new Error("markdown report failed");
}
const json = run(["report", "--root", validRoot, "--json"]);
if (json.status !== 0 || JSON.parse(json.stdout).schema !== "sley.conformance.report.v1") {
  throw new Error("JSON report failed");
}
const help = run(["--help"]);
if (help.status !== 0 || !help.stdout.includes("usage:")) throw new Error("help failed");

console.log("sley-conformance smoke ok");
