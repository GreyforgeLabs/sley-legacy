import { existsSync, lstatSync, readdirSync, readFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";

export function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8"));
}

export function listJsonFiles(root, predicate = () => true) {
  const sourceRoot = resolve(root);
  if (!existsSync(sourceRoot) || !lstatSync(sourceRoot).isDirectory()) return [];
  return readdirSync(sourceRoot)
    .filter(predicate)
    .filter((file) => file.endsWith(".json"))
    .map((file) => join(sourceRoot, file))
    .filter((filePath) => {
      try {
        return lstatSync(filePath).isFile();
      } catch {
        return false;
      }
    })
    .sort();
}

function displayPath(root, filePath) {
  return relative(root, filePath).replaceAll("\\", "/");
}

function stableIdentity(value) {
  if (value && typeof value === "object") {
    for (const key of ["$id", "id", "name", "path"]) {
      if (typeof value[key] === "string" && value[key].trim()) return `${key}:${value[key]}`;
    }
  }
  return `value:${JSON.stringify(value)}`;
}

function duplicateValues(values) {
  const seen = new Set();
  const duplicates = new Set();
  for (const value of values) {
    const identity = stableIdentity(value);
    if (seen.has(identity)) duplicates.add(identity);
    seen.add(identity);
  }
  return [...duplicates].sort();
}

function readMandatoryManifest(root, filePath, evidenceClass, errors) {
  const accounting = { discovered: 0, parsed: 0, malformed: 0, duplicate: 0, validated: 0 };
  if (!existsSync(filePath)) {
    errors.push({ class: evidenceClass, path: displayPath(root, filePath), code: "missing_manifest" });
    return { value: null, accounting };
  }
  accounting.discovered = 1;
  try {
    const value = readJson(filePath);
    accounting.parsed = 1;
    return { value, accounting };
  } catch (error) {
    accounting.malformed = 1;
    errors.push({
      class: evidenceClass,
      path: displayPath(root, filePath),
      code: "malformed_json",
      message: error instanceof Error ? error.message : "invalid JSON",
    });
    return { value: null, accounting };
  }
}

function readJsonEvidence(root, directory, predicate, evidenceClass, validator, errors) {
  const files = listJsonFiles(directory, predicate);
  const accounting = { discovered: files.length, parsed: 0, malformed: 0, duplicate: 0, validated: 0 };
  const records = [];
  for (const filePath of files) {
    let value;
    try {
      value = readJson(filePath);
      accounting.parsed += 1;
    } catch (error) {
      accounting.malformed += 1;
      errors.push({
        class: evidenceClass,
        path: displayPath(root, filePath),
        code: "malformed_json",
        message: error instanceof Error ? error.message : "invalid JSON",
      });
      continue;
    }
    const validationError = validator(value);
    if (validationError) {
      accounting.malformed += 1;
      errors.push({ class: evidenceClass, path: displayPath(root, filePath), ...validationError });
      continue;
    }
    accounting.validated += 1;
    records.push({ filePath, value });
  }
  return { records, accounting };
}

function recordDuplicates(root, records, evidenceClass, identityFor, errors, accounting) {
  const seen = new Map();
  for (const record of records) {
    const identity = identityFor(record.value);
    if (!identity) continue;
    const first = seen.get(identity);
    if (first) {
      accounting.duplicate += 1;
      errors.push({
        class: evidenceClass,
        path: displayPath(root, record.filePath),
        code: "duplicate_identifier",
        identifier: identity,
        firstPath: displayPath(root, first.filePath),
      });
    } else {
      seen.set(identity, record);
    }
  }
}

export function loadConformance(root, { allowEmpty = false } = {}) {
  const rootPath = resolve(root);
  const errors = [];
  if (!existsSync(rootPath) || !lstatSync(rootPath).isDirectory()) {
    errors.push({ class: "root", path: ".", code: "missing_root" });
  }

  const corpusResult = readMandatoryManifest(
    rootPath,
    join(rootPath, "fixtures/corpus/manifest.json"),
    "corpus",
    errors,
  );
  const smokeResult = readMandatoryManifest(
    rootPath,
    join(rootPath, "fixtures/cli_smokes/manifest.json"),
    "smoke",
    errors,
  );

  const corpus = corpusResult.value;
  const accepted = Array.isArray(corpus?.accepted) ? corpus.accepted : [];
  const rejected = Array.isArray(corpus?.rejected) ? corpus.rejected : [];
  if (corpusResult.accounting.parsed && (!Array.isArray(corpus?.accepted) || !Array.isArray(corpus?.rejected))) {
    corpusResult.accounting.malformed += 1;
    errors.push({ class: "corpus", path: "fixtures/corpus/manifest.json", code: "invalid_manifest_shape" });
  } else if (corpusResult.accounting.parsed) {
    const duplicates = duplicateValues([...accepted, ...rejected]);
    corpusResult.accounting.duplicate = duplicates.length;
    for (const identifier of duplicates) {
      errors.push({ class: "corpus", path: "fixtures/corpus/manifest.json", code: "duplicate_identifier", identifier });
    }
    corpusResult.accounting.validated = accepted.length + rejected.length;
    if (!allowEmpty && (accepted.length === 0 || rejected.length === 0)) {
      errors.push({ class: "corpus", path: "fixtures/corpus/manifest.json", code: "empty_inventory" });
    }
  }

  const smoke = smokeResult.value;
  const smokeCases = Array.isArray(smoke?.cases) ? smoke.cases : [];
  if (smokeResult.accounting.parsed && !Array.isArray(smoke?.cases)) {
    smokeResult.accounting.malformed += 1;
    errors.push({ class: "smoke", path: "fixtures/cli_smokes/manifest.json", code: "invalid_manifest_shape" });
  } else if (smokeResult.accounting.parsed) {
    const duplicates = duplicateValues(smokeCases);
    smokeResult.accounting.duplicate = duplicates.length;
    for (const identifier of duplicates) {
      errors.push({ class: "smoke", path: "fixtures/cli_smokes/manifest.json", code: "duplicate_identifier", identifier });
    }
    smokeResult.accounting.validated = smokeCases.length;
    if (!allowEmpty && smokeCases.length === 0) {
      errors.push({ class: "smoke", path: "fixtures/cli_smokes/manifest.json", code: "empty_inventory" });
    }
  }

  const schemas = readJsonEvidence(
    rootPath,
    join(rootPath, "docs/schemas"),
    (file) => file.endsWith(".schema.json"),
    "schemas",
    (schema) => typeof schema?.$id === "string" && schema.$id.trim()
      ? null
      : { code: "missing_schema_id" },
    errors,
  );
  recordDuplicates(rootPath, schemas.records, "schemas", (schema) => schema.$id, errors, schemas.accounting);

  const contracts = readJsonEvidence(
    rootPath,
    join(rootPath, "fixtures/contracts"),
    () => true,
    "contracts",
    (contract) => typeof contract?.schema === "string" && contract.schema.trim()
      ? null
      : { code: "missing_schema_reference" },
    errors,
  );
  recordDuplicates(
    rootPath,
    contracts.records,
    "contracts",
    (contract) => typeof contract.id === "string" ? contract.id : null,
    errors,
    contracts.accounting,
  );

  if (!allowEmpty && schemas.accounting.validated === 0) {
    errors.push({ class: "schemas", path: "docs/schemas", code: "empty_inventory" });
  }
  if (!allowEmpty && contracts.accounting.validated === 0) {
    errors.push({ class: "contracts", path: "fixtures/contracts", code: "empty_inventory" });
  }

  const smokeTags = new Map();
  for (const testCase of smokeCases) {
    if (!Array.isArray(testCase?.covers)) continue;
    for (const tag of testCase.covers) {
      if (typeof tag === "string" && tag) smokeTags.set(tag, (smokeTags.get(tag) ?? 0) + 1);
    }
  }

  const schemaIds = new Set(schemas.records.map((record) => record.value.$id));
  const contractSchemas = new Set(contracts.records.map((record) => record.value.schema));
  const missingReferences = [...contractSchemas].filter((schema) => !schemaIds.has(schema)).sort();
  for (const schema of missingReferences) {
    errors.push({ class: "contracts", path: "fixtures/contracts", code: "missing_schema_target", identifier: schema });
  }

  return {
    schema: "sley.conformance.report.v1",
    root: rootPath,
    allowEmpty,
    accounting: {
      corpus: corpusResult.accounting,
      smoke: smokeResult.accounting,
      schemas: schemas.accounting,
      contracts: contracts.accounting,
    },
    errors,
    corpus: {
      accepted: accepted.length,
      rejected: rejected.length,
      valid: corpusResult.accounting.malformed === 0 && corpusResult.accounting.duplicate === 0,
    },
    smoke: {
      cases: smokeCases.length,
      tag_count: smokeTags.size,
      tags: Object.fromEntries([...smokeTags.entries()].sort((a, b) => a[0].localeCompare(b[0]))),
      valid: smokeResult.accounting.malformed === 0 && smokeResult.accounting.duplicate === 0,
    },
    schemas: {
      count: schemas.accounting.validated,
      files: schemas.records.map((record) => displayPath(rootPath, record.filePath)),
      schemaIds: [...schemaIds].sort(),
    },
    contracts: {
      count: contracts.accounting.validated,
      files: contracts.records.map((record) => displayPath(rootPath, record.filePath)),
      schemaRefs: [...contractSchemas].sort(),
      missingSchemaRefs: missingReferences,
    },
  };
}

export function requireCoverage(report, tag) {
  return {
    schema: "sley.conformance.coverage.v1",
    tag,
    ok: Boolean(report.smoke.tags[tag]),
    count: report.smoke.tags[tag] ?? 0,
  };
}

export function requireCoverageThreshold(report, tag, minimum) {
  if (!Number.isInteger(minimum) || minimum < 1) {
    throw new RangeError("minimum must be an integer greater than or equal to 1");
  }
  const count = report.smoke.tags[tag] ?? 0;
  return {
    schema: "sley.conformance.coverage_threshold.v1",
    tag,
    minimum,
    count,
    ok: count >= minimum,
  };
}

export function reportHealth(report) {
  const mandatoryNonEmpty = report.allowEmpty || (
    report.corpus.accepted > 0
    && report.corpus.rejected > 0
    && report.smoke.cases > 0
    && report.schemas.count > 0
    && report.contracts.count > 0
  );
  return {
    schema: "sley.conformance.health.v1",
    ok: report.errors.length === 0 && mandatoryNonEmpty,
    issues: {
      errorCount: report.errors.length,
      mandatoryNonEmpty,
      missingSchemaRefs: report.contracts.missingSchemaRefs.length,
      smokeCases: report.smoke.cases,
      validatedSchemas: report.schemas.count,
      validatedContracts: report.contracts.count,
    },
  };
}
