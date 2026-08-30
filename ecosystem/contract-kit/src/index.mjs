import { lstatSync, readdirSync, readFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

export const DEFAULT_LIMITS = Object.freeze({
  maxBytes: 1024 * 1024,
  maxDepth: 64,
  maxFiles: 2048,
});

export const SAFE_MAXIMA = Object.freeze({
  maxBytes: 16 * 1024 * 1024,
  maxDepth: 256,
  maxFiles: 10000,
});

export class ContractError extends Error {
  constructor(code, message, { file = null, exitCode = 2 } = {}) {
    super(message);
    this.name = "ContractError";
    this.contractCode = code;
    this.file = file;
    this.exitCode = exitCode;
  }
}

function boundedOptions(options = {}) {
  const limits = { ...DEFAULT_LIMITS };
  for (const key of Object.keys(limits)) {
    if (options[key] === undefined) continue;
    const value = Number(options[key]);
    if (!Number.isSafeInteger(value) || value < 1 || value > SAFE_MAXIMA[key]) {
      throw new ContractError(
        "INVALID_LIMIT",
        `${key} must be an integer between 1 and ${SAFE_MAXIMA[key]}`,
      );
    }
    limits[key] = value;
  }
  return limits;
}

function assertJsonDepth(value, maxDepth, file) {
  const stack = [{ value, depth: 1 }];
  let visited = 0;
  while (stack.length > 0) {
    const current = stack.pop();
    visited += 1;
    if (visited > 500000) {
      throw new ContractError("JSON_TOO_COMPLEX", "JSON node count exceeds the safety limit", { file });
    }
    if (current.depth > maxDepth) {
      throw new ContractError("JSON_TOO_DEEP", `JSON nesting exceeds ${maxDepth}`, { file });
    }
    if (current.value && typeof current.value === "object") {
      for (const child of Object.values(current.value)) {
        stack.push({ value: child, depth: current.depth + 1 });
      }
    }
  }
}

export function readJson(filePath, options = {}) {
  const limits = boundedOptions(options);
  const file = resolve(filePath);
  let metadata;
  try {
    metadata = lstatSync(file);
  } catch (error) {
    throw new ContractError("READ_ERROR", `cannot inspect contract file: ${error.code || "UNKNOWN"}`, {
      file,
      exitCode: 3,
    });
  }
  if (!metadata.isFile()) {
    throw new ContractError("NOT_REGULAR_FILE", "contract input must be a regular file", { file });
  }
  if (metadata.size > limits.maxBytes) {
    throw new ContractError(
      "FILE_TOO_LARGE",
      `contract file exceeds ${limits.maxBytes} bytes`,
      { file },
    );
  }
  let text;
  try {
    text = readFileSync(file, "utf8");
  } catch (error) {
    throw new ContractError("READ_ERROR", `cannot read contract file: ${error.code || "UNKNOWN"}`, {
      file,
      exitCode: 3,
    });
  }
  if (Buffer.byteLength(text, "utf8") > limits.maxBytes) {
    throw new ContractError(
      "FILE_TOO_LARGE",
      `contract file exceeds ${limits.maxBytes} bytes`,
      { file },
    );
  }
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new ContractError("MALFORMED_JSON", "contract file is not valid JSON", { file });
  }
  assertJsonDepth(parsed, limits.maxDepth, file);
  return parsed;
}

function contractFiles(root, suffix, options = {}) {
  const limits = boundedOptions(options);
  const sourceRoot = resolve(root);
  let entries;
  try {
    entries = readdirSync(sourceRoot, { withFileTypes: true });
  } catch (error) {
    throw new ContractError("READ_ERROR", `cannot read contract directory: ${error.code || "UNKNOWN"}`, {
      file: sourceRoot,
      exitCode: 3,
    });
  }
  const candidates = entries.filter((entry) => entry.name.endsWith(suffix));
  if (candidates.length > limits.maxFiles) {
    throw new ContractError(
      "TOO_MANY_FILES",
      `contract directory exceeds ${limits.maxFiles} matching files`,
      { file: sourceRoot },
    );
  }
  for (const entry of candidates) {
    if (!entry.isFile()) {
      throw new ContractError("NOT_REGULAR_FILE", "contract entry must be a regular file", {
        file: join(sourceRoot, entry.name),
      });
    }
  }
  return candidates.map((entry) => join(sourceRoot, entry.name)).sort();
}

function requireEvidence(files, root, allowEmpty) {
  if (files.length === 0 && !allowEmpty) {
    throw new ContractError("EMPTY_EVIDENCE", "no contract evidence files were found", {
      file: resolve(root),
    });
  }
}

function requireObject(value, file) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new ContractError("INVALID_SHAPE", "contract root must be a JSON object", { file });
  }
}

export function schemaInventory(schemaDir, options = {}) {
  const files = contractFiles(schemaDir, ".schema.json", options);
  requireEvidence(files, schemaDir, Boolean(options.allowEmpty));
  const seenIds = new Map();
  const inventory = files.map((file) => {
    const schema = readJson(file, options);
    requireObject(schema, file);
    const id = typeof schema.$id === "string" && schema.$id ? schema.$id : null;
    if (id && seenIds.has(id)) {
      throw new ContractError(
        "DUPLICATE_ID",
        `duplicate schema id ${id}; first declared in ${seenIds.get(id)}`,
        { file },
      );
    }
    if (id) seenIds.set(id, file);
    return {
      file: basename(file),
      path: file,
      id,
      title: schema.title ?? null,
      rootSchemaConst: schema.properties?.schema?.const ?? null,
      requiredTopLevelProps: Array.isArray(schema.required) ? [...schema.required] : [],
      hasDefinitions: Boolean(schema.definitions || schema.$defs),
    };
  });
  return inventory.sort((a, b) => a.file.localeCompare(b.file));
}

export function validateRoot(expected, reportPath, options = {}) {
  const normalized = resolve(reportPath);
  const report = readJson(normalized, options);
  requireObject(report, normalized);
  const actual = typeof report.schema === "string" ? report.schema : null;
  const reasons = [];
  if (!actual) reasons.push("missing schema marker");
  if (actual !== expected) reasons.push(`schema expected ${expected}`);
  return {
    ok: actual === expected,
    expected,
    actual,
    hasReportId: Boolean(actual),
    path: normalized,
    reasons,
  };
}

export function fixtureFiles(root, options = {}) {
  return contractFiles(root, ".json", options);
}

function fixtureRecords(root, options = {}) {
  const files = fixtureFiles(root, options);
  requireEvidence(files, root, Boolean(options.allowEmpty));
  const seenIds = new Map();
  const records = new Map();
  for (const file of files) {
    const json = readJson(file, options);
    requireObject(json, file);
    const schema = typeof json.schema === "string" ? json.schema : null;
    const id = json.id === undefined ? null : json.id;
    if (id !== null && (typeof id !== "string" || !id)) {
      throw new ContractError("INVALID_ID", "fixture id must be a non-empty string", { file });
    }
    if (id && seenIds.has(id)) {
      throw new ContractError(
        "DUPLICATE_ID",
        `duplicate fixture id ${id}; first declared in ${seenIds.get(id)}`,
        { file },
      );
    }
    if (id) seenIds.set(id, file);
    const rootMarker = json.root === undefined ? null : json.root;
    if (rootMarker !== null && typeof rootMarker !== "string") {
      throw new ContractError("INVALID_ROOT_MARKER", "fixture root marker must be a string", { file });
    }
    records.set(basename(file), { file, schema, id, root: rootMarker, json });
  }
  return records;
}

export function readFixtureIds(root, options = {}) {
  const records = fixtureRecords(root, options);
  return new Map([...records.values()].map((record) => [record.file, record.schema]));
}

export function checkFixtures(root, options = {}) {
  const sourceRoot = resolve(root);
  const records = fixtureRecords(root, options);
  const results = [...records.values()].map((record) => ({
    path: record.file,
    id: record.id,
    schema: record.schema,
    status: record.json.status ?? null,
    ok: record.schema?.startsWith("sley.") ?? false,
    rootMarker: record.root,
  }));
  return {
    root: sourceRoot,
    schemaPrefix: "sley.",
    ok: results.length > 0 ? results.every((result) => result.ok) : Boolean(options.allowEmpty),
    count: results.length,
    results,
  };
}

export function compareFixtureSets(leftRoot, rightRoot, options = {}) {
  const left = fixtureRecords(leftRoot, options);
  const right = fixtureRecords(rightRoot, options);
  const missing = [];
  const extra = [];
  const drift = [];
  let comparableCount = 0;

  for (const [file, leftRecord] of left.entries()) {
    const rightRecord = right.get(file);
    if (!rightRecord) {
      missing.push(file);
      continue;
    }
    comparableCount += 1;
    for (const field of ["schema", "root", "id"]) {
      if (leftRecord[field] !== rightRecord[field]) {
        drift.push({ file, field, left: leftRecord[field], right: rightRecord[field] });
      }
    }
  }

  for (const file of right.keys()) {
    if (!left.has(file)) extra.push(file);
  }

  const emptyAllowed = Boolean(options.allowEmpty) && left.size === 0 && right.size === 0;
  return {
    left: resolve(leftRoot),
    right: resolve(rightRoot),
    leftCount: left.size,
    rightCount: right.size,
    comparableCount,
    drift,
    missing: missing.sort(),
    extra: extra.sort(),
    ok:
      (comparableCount > 0 || emptyAllowed) &&
      drift.length === 0 &&
      missing.length === 0 &&
      extra.length === 0,
  };
}
