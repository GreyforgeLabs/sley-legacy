#!/usr/bin/env node
import {
  closeSync,
  constants,
  existsSync,
  fstatSync,
  lstatSync,
  openSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { TextDecoder } from "node:util";

const VERSION = "0.2.0";
const REPORT_SCHEMA = "sley.audit.report.v1";
const FINGERPRINT_DOMAIN = "sley-audit-finding-v1";
const DEFAULT_EXTENSIONS = new Set([
  ".rs", ".sley", ".js", ".mjs", ".ts", ".json", ".toml", ".md", ".yml", ".yaml",
]);
const SKIP_DIRS = new Set([".git", "node_modules", "target", ".cache", "dist"]);
const DEFAULT_LIMITS = Object.freeze({
  maxFiles: 10_000,
  maxFileBytes: 1024 * 1024,
  maxTotalBytes: 50 * 1024 * 1024,
  maxDepth: 64,
});

function usage() {
  return [
    "sley-audit [--path <dir>] [--json] [--sarif <file>] [--allowlist <file>]",
    "           [--ci|--fail-on-error|--report-only] [--allow-empty]",
    "           [--max-files <n>] [--max-file-bytes <n>]",
    "           [--max-total-bytes <n>] [--max-depth <n>]",
    "",
    "No arguments scans the current directory and fails on high-severity findings.",
    "Coverage failures always exit 2. --report-only suppresses finding exit 1,",
    "but never suppresses incomplete coverage.",
    "",
  ].join("\n");
}

function usageError(message) {
  console.error("sley-audit: " + message);
  process.stderr.write(usage());
  process.exit(2);
}

function readPositiveInteger(args, index, flag) {
  const value = args[index + 1];
  if (value === undefined || !/^[1-9][0-9]*$/.test(value)) {
    usageError(flag + " requires a positive integer");
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) usageError(flag + " exceeds the safe integer range");
  return parsed;
}

const args = process.argv.slice(2);
if (args.includes("-h") || args.includes("--help")) {
  process.stdout.write(usage());
  process.exit(0);
}

const options = {
  path: ".",
  json: false,
  sarif: null,
  allowlistPath: null,
  reportOnly: false,
  allowEmpty: false,
  debugAbsolutePaths: false,
  ...DEFAULT_LIMITS,
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === "--json") {
    options.json = true;
  } else if (arg === "--ci" || arg === "--fail-on-error") {
    options.reportOnly = false;
  } else if (arg === "--report-only") {
    options.reportOnly = true;
  } else if (arg === "--allow-empty") {
    options.allowEmpty = true;
  } else if (arg === "--debug-absolute-paths") {
    options.debugAbsolutePaths = true;
  } else if (arg === "--path" || arg === "--sarif" || arg === "--allowlist") {
    const value = args[++index];
    if (value === undefined || value.startsWith("--")) usageError(arg + " requires a value");
    if (arg === "--path") options.path = value;
    if (arg === "--sarif") options.sarif = value;
    if (arg === "--allowlist") options.allowlistPath = value;
  } else if (["--max-files", "--max-file-bytes", "--max-total-bytes", "--max-depth"].includes(arg)) {
    const value = readPositiveInteger(args, index, arg);
    index += 1;
    if (arg === "--max-files") options.maxFiles = value;
    if (arg === "--max-file-bytes") options.maxFileBytes = value;
    if (arg === "--max-total-bytes") options.maxTotalBytes = value;
    if (arg === "--max-depth") options.maxDepth = value;
  } else {
    usageError("unknown option: " + arg);
  }
}

const requestedRoot = path.resolve(options.path);
if (!existsSync(requestedRoot)) usageError("scan root does not exist: " + options.path);
const rootStat = lstatSync(requestedRoot);
if (rootStat.isSymbolicLink() || !rootStat.isDirectory()) {
  usageError("scan root must be a real directory, not a symlink or special file: " + options.path);
}
const root = realpathSync(requestedRoot);
const allowlist = loadAllowlist(options.allowlistPath);

const rules = [
  {
    id: "SLEY-AUDIT-DEBUG-FORMAT",
    severity: "high",
    message: "Debug formatter in logging-like macro can leak sensitive value shapes/contents",
    matcher: (line) => {
      const hasLoggerLikeCall = /\b(?:println|eprintln|format|dbg|panic|assert|error|warn|info|trace|debug)\s*!(?:\s*\()/.test(line);
      const hasDebugPlaceholder = /\{\s*[^{}]*:\s*#?\?[^{}]*\}/.test(line);
      return hasLoggerLikeCall && hasDebugPlaceholder;
    },
  },
  {
    id: "SLEY-AUDIT-EXPLICIT-DEBUG-RECORD",
    severity: "high",
    message: "Explicit debug-record interpolation may output cleartext error fields",
    matcher: (line) => /\{\s*\w+(?:\.\w+)*(?:\{)?\s*:\s*#\?\s*\}?/.test(line),
  },
  {
    id: "SLEY-AUDIT-SUSPECT-PANIC-LITERAL",
    severity: "medium",
    message: "panic-like message interpolates full runtime objects with debug format",
    matcher: (line) => /\bpanic\s*!\s*\([^)]*\{[^}]*#?\?[^}]*\}/.test(line),
  },
];

const coverage = {
  discovered: 0,
  eligible: 0,
  read: 0,
  skippedByPolicy: 0,
  failed: 0,
  oversized: 0,
  totalBytes: 0,
  truncated: false,
};
const omissions = [];
const findings = [];

walk(root, 0);

if (coverage.eligible === 0 && !options.allowEmpty) {
  coverage.failed += 1;
  omissions.push({ path: ".", reason: "empty_eligible_set" });
}

for (const finding of findings) {
  finding.fingerprint = findingFingerprint(finding);
}
const sorted = findings.sort((left, right) => {
  if (left.path !== right.path) return left.path.localeCompare(right.path);
  if (left.line !== right.line) return left.line - right.line;
  return left.ruleId.localeCompare(right.ruleId);
});
omissions.sort((left, right) => {
  if (left.path !== right.path) return left.path.localeCompare(right.path);
  return left.reason.localeCompare(right.reason);
});

const coverageComplete = coverage.failed === 0 && !coverage.truncated;
const blockingFindings = sorted.filter((finding) => finding.severity === "high").length;
const status = !coverageComplete ? "error" : blockingFindings > 0 ? "fail" : sorted.length > 0 ? "warn" : "pass";
const report = {
  schema: REPORT_SCHEMA,
  schemaVersion: 1,
  fingerprintAlgorithm: "sha256",
  generatedAt: new Date().toISOString(),
  status,
  root: ".",
  coverageComplete,
  coverage,
  limits: {
    maxFiles: options.maxFiles,
    maxFileBytes: options.maxFileBytes,
    maxTotalBytes: options.maxTotalBytes,
    maxDepth: options.maxDepth,
  },
  totalFindings: sorted.length,
  blockingFindings,
  findings: sorted,
  omissions,
};
if (options.debugAbsolutePaths) report.absoluteRoot = root.replaceAll("\\", "/");

if (options.json) process.stdout.write(JSON.stringify(report, null, 2) + "\n");
if (options.sarif) writeSarif(sorted, options.sarif, coverageComplete);
if (!options.json) printHuman(report);

if (!coverageComplete) process.exit(2);
if (blockingFindings > 0 && !options.reportOnly) process.exit(1);
process.exit(0);

function displayPath(filePath) {
  const rel = path.relative(root, filePath);
  if (rel === "") return ".";
  if (path.isAbsolute(rel) || rel === ".." || rel.startsWith(".." + path.sep)) {
    throw new Error("path escaped the audited root");
  }
  return rel.replaceAll("\\", "/");
}

function omit(filePath, reason, { failed = true, oversized = false } = {}) {
  if (failed) coverage.failed += 1;
  if (oversized) coverage.oversized += 1;
  omissions.push({ path: displayPath(filePath), reason });
}

function walk(current, depth) {
  if (coverage.truncated) return;
  let entries;
  try {
    entries = readdirSync(current, { withFileTypes: true })
      .sort((left, right) => left.name.localeCompare(right.name));
  } catch {
    omit(current, "directory_unreadable");
    return;
  }
  for (const entry of entries) {
    if (coverage.truncated) return;
    const fullPath = path.join(current, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      if (depth >= options.maxDepth) {
        omit(fullPath, "max_depth_exceeded");
        continue;
      }
      walk(fullPath, depth + 1);
      continue;
    }

    coverage.discovered += 1;
    if (coverage.discovered > options.maxFiles) {
      coverage.truncated = true;
      omit(fullPath, "max_file_count_exceeded");
      return;
    }

    let metadata;
    try {
      metadata = lstatSync(fullPath);
    } catch {
      omit(fullPath, "metadata_unreadable");
      continue;
    }
    if (metadata.isSymbolicLink()) {
      omit(fullPath, "symlink_rejected");
      continue;
    }
    if (!metadata.isFile()) {
      omit(fullPath, "special_file_rejected");
      continue;
    }
    if (!isEligible(fullPath)) {
      coverage.skippedByPolicy += 1;
      omissions.push({ path: displayPath(fullPath), reason: "unsupported_extension" });
      continue;
    }

    coverage.eligible += 1;
    if (metadata.size > options.maxFileBytes) {
      omit(fullPath, "file_size_limit_exceeded", { oversized: true });
      continue;
    }
    if (coverage.totalBytes + metadata.size > options.maxTotalBytes) {
      omit(fullPath, "aggregate_byte_limit_exceeded", { oversized: true });
      continue;
    }

    let descriptor;
    try {
      descriptor = openSync(fullPath, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
      const opened = fstatSync(descriptor);
      if (!opened.isFile()) throw new Error("not a regular file");
      if (opened.size > options.maxFileBytes) {
        omit(fullPath, "file_size_limit_exceeded", { oversized: true });
        continue;
      }
      if (coverage.totalBytes + opened.size > options.maxTotalBytes) {
        omit(fullPath, "aggregate_byte_limit_exceeded", { oversized: true });
        continue;
      }
      const buffer = readFileSync(descriptor);
      coverage.totalBytes += buffer.length;
      if (buffer.includes(0)) {
        omit(fullPath, "binary_content_rejected");
        continue;
      }
      let content;
      try {
        content = new TextDecoder("utf-8", { fatal: true }).decode(buffer);
      } catch {
        omit(fullPath, "invalid_utf8");
        continue;
      }
      coverage.read += 1;
      inspectFile(fullPath, content);
    } catch {
      omit(fullPath, "file_unreadable");
    } finally {
      if (descriptor !== undefined) closeSync(descriptor);
    }
  }
}

function isEligible(filePath) {
  const base = path.basename(filePath).toLowerCase();
  const ext = path.extname(base);
  return DEFAULT_EXTENSIONS.has(ext) || base.startsWith(".") || ext.length === 0 || base === "cargo.lock";
}

function inspectFile(filePath, content) {
  const relativePath = displayPath(filePath);
  const lines = content.split(/\r?\n/);
  for (let lineNumber = 0; lineNumber < lines.length; lineNumber += 1) {
    const line = lines[lineNumber];
    for (const rule of rules) {
      if (!rule.matcher(line)) continue;
      const finding = {
        ruleId: rule.id,
        severity: rule.severity,
        message: rule.message,
        path: relativePath,
        line: lineNumber + 1,
        column: 1,
        snippet: line.trim(),
      };
      if (!isAllowlisted(finding, allowlist)) findings.push(finding);
    }
  }
}

function loadAllowlist(filePath) {
  if (!filePath) return [];
  const fullPath = path.resolve(filePath);
  if (!existsSync(fullPath)) usageError("allowlist file not found: " + filePath);
  let content;
  try {
    content = readFileSync(fullPath, "utf8");
  } catch {
    usageError("allowlist file is unreadable: " + filePath);
  }
  if (Buffer.byteLength(content, "utf8") > 64 * 1024) usageError("allowlist exceeds 64 KiB");
  const entries = [];
  for (const line of content.split(/\r?\n/)) {
    const value = line.trim();
    if (!value || value.startsWith("#")) continue;
    if (entries.length >= 256) usageError("allowlist exceeds 256 patterns");
    if (value.length > 512) usageError("allowlist pattern exceeds 512 characters");
    if (/\\[1-9]/.test(value) || /\([^)]*[+*][^)]*\)[+*{]/.test(value)) {
      usageError("allowlist pattern uses a disallowed high-risk regex construct: " + value);
    }
    try {
      entries.push(new RegExp(value, "u"));
    } catch {
      usageError("invalid allowlist regex: " + value);
    }
  }
  return entries;
}

function isAllowlisted(finding, entries) {
  if (entries.length === 0) return false;
  const haystack = finding.path + ":" + finding.line + ":" + finding.snippet;
  return entries.some((expression) => (
    expression.test(finding.path)
    || expression.test(haystack)
    || expression.test(finding.message)
  ));
}

function findingFingerprint(finding) {
  const hash = crypto.createHash("sha256");
  for (const field of [
    FINGERPRINT_DOMAIN,
    finding.ruleId,
    finding.severity,
    finding.path,
    String(finding.line),
    String(finding.column),
    finding.snippet,
  ]) {
    hash.update(field);
    hash.update("\0");
  }
  return hash.digest("hex");
}

function printHuman(result) {
  console.log(
    "sley-audit: "
      + result.status.toUpperCase()
      + " ("
      + result.totalFindings
      + " finding(s), "
      + result.coverage.read
      + "/"
      + result.coverage.eligible
      + " eligible file(s) read)",
  );
  for (const finding of result.findings) {
    console.log(
      finding.path
        + ":"
        + finding.line
        + " "
        + finding.ruleId
        + " "
        + finding.severity
        + ": "
        + finding.message,
    );
    if (finding.snippet) console.log("  " + finding.snippet);
  }
  for (const omission of result.omissions) {
    if (omission.reason !== "unsupported_extension") {
      console.log(omission.path + ": omitted (" + omission.reason + ")");
    }
  }
}

function writeSarif(items, outFile, coverageComplete) {
  const grouped = new Map();
  for (const finding of items) {
    if (!grouped.has(finding.ruleId)) {
      grouped.set(finding.ruleId, {
        id: finding.ruleId,
        shortDescription: { text: finding.message },
        fullDescription: { text: finding.message },
        helpUri: "https://sley.greyforge.tech/tools/sley-audit",
      });
    }
  }
  const sarif = {
    version: "2.1.0",
    $schema: "https://json.schemastore.org/sarif-2.1.0-rtm.5.json",
    runs: [{
      tool: {
        driver: {
          name: "sley-audit",
          semanticVersion: VERSION,
          informationUri: "https://sley.greyforge.tech/tools/sley-audit",
          rules: [...grouped.values()],
        },
      },
      invocations: [{ executionSuccessful: coverageComplete }],
      results: items.map((finding) => ({
        ruleId: finding.ruleId,
        level: finding.severity === "high" ? "error" : "warning",
        message: { text: finding.message },
        locations: [{
          physicalLocation: {
            artifactLocation: { uri: finding.path },
            region: { startLine: finding.line, startColumn: finding.column },
          },
        }],
        properties: {
          snippet: finding.snippet,
          fingerprintAlgorithm: "sha256",
        },
        partialFingerprints: {
          primaryLocationLineHash: finding.fingerprint,
        },
      })),
    }],
  };
  writeFileSync(outFile, JSON.stringify(sarif, null, 2) + "\n", "utf8");
}
