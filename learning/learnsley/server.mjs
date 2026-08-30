import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { mkdtemp, mkdir, readdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = __dirname;
const publicDir = path.join(rootDir, "public");
const defaultWorkDir = path.join(rootDir, "workspaces");
const trialsPath = path.join(rootDir, "content", "trials.json");
const defaultSleyBin = path.join(rootDir, "..", "sley", "target", "debug", "sley");

const maxBodyBytes = 64 * 1024;
const maxCodeBytes = 24 * 1024;
const maxOutputBytes = 128 * 1024;
const commandTimeoutMs = 5000;
const staleWorkspaceMaxAgeMs = 6 * 60 * 60 * 1000;
const staleWorkspaceMaxCount = 64;
const staleWorkspaceMaxBytes = 32 * 1024 * 1024;

const mutationRoutes = new Set(["/api/run", "/api/format", "/api/graph", "/api/seal"]);
const loopbackNames = new Set(["localhost", "127.0.0.1", "::1"]);

const mimeTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"],
  [".png", "image/png"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".ico", "image/x-icon"]
]);

function jsonResponse(res, status, payload) {
  const body = JSON.stringify(payload, null, 2);
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store"
  });
  res.end(body);
}

function textResponse(res, status, body, contentType = "text/plain; charset=utf-8") {
  res.writeHead(status, { "content-type": contentType, "cache-control": "no-store" });
  res.end(body);
}

function httpError(statusCode, message) {
  return Object.assign(new Error(message), { statusCode });
}

function parseHostHeader(value) {
  const normalized = String(value || "").toLowerCase();
  const ipv6 = normalized.match(/^\[(::1)\](?::([0-9]{1,5}))?$/);
  const named = normalized.match(/^(localhost|127\.0\.0\.1)(?::([0-9]{1,5}))?$/);
  const match = ipv6 || named;
  if (!match) {
    return null;
  }
  const port = match[2];
  if (port && (Number(port) < 1 || Number(port) > 65535)) {
    return null;
  }
  return match[1];
}

function isLoopbackHostName(value) {
  return loopbackNames.has(String(value || "").toLowerCase());
}

function requestOrigin(req) {
  const protocol = req.socket.encrypted ? "https" : "http";
  return `${protocol}://${req.headers.host}`;
}

function validateHost(req) {
  const hostName = parseHostHeader(req.headers.host);
  if (!isLoopbackHostName(hostName)) {
    throw httpError(403, "host must be loopback");
  }
}

function validateMutationRequest(req, pathname, csrfToken) {
  if (!mutationRoutes.has(pathname)) {
    return;
  }

  const contentType = String(req.headers["content-type"] || "").toLowerCase();
  if (!contentType.split(";")[0].trim().match(/^application\/json$/)) {
    throw httpError(415, "mutations require application/json");
  }

  const origin = req.headers.origin;
  if (!origin || origin === "null" || origin !== requestOrigin(req)) {
    throw httpError(403, "mutations require same-origin requests");
  }

  if (req.headers["x-learnsley-csrf"] !== csrfToken) {
    throw httpError(403, "invalid csrf token");
  }
}

async function readJson(file, fallback) {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") {
      return fallback;
    }
    throw error;
  }
}

async function loadTrials() {
  const trials = await readJson(trialsPath, []);
  return trials.map((trial, index) => ({ ...trial, order: index + 1 }));
}

function publicTrial(trial) {
  const { seal, ...safeTrial } = trial;
  return safeTrial;
}

function baseProgress() {
  return {
    schema: "learnsley.progress.v0",
    xp: 0,
    shards: 0,
    level: 1,
    dailyPulse: 0,
    activeDays: [],
    cleanSealChain: 0,
    proofArmor: 2,
    completed: {},
    attempts: {},
    updatedAt: null
  };
}

async function loadProgress(progressPath) {
  try {
    const progress = await readJson(progressPath, baseProgress());
    return { ...baseProgress(), ...progress };
  } catch (error) {
    if (error instanceof SyntaxError) {
      const corruptPath = path.join(
        path.dirname(progressPath),
        `${path.basename(progressPath)}.corrupt.${Date.now()}.${randomUUID()}`
      );
      await rename(progressPath, corruptPath).catch(() => {});
      return baseProgress();
    }
    throw error;
  }
}

async function saveProgress(progressPath, progress) {
  progress.updatedAt = new Date().toISOString();
  const tmp = path.join(
    path.dirname(progressPath),
    `${path.basename(progressPath)}.${process.pid}.${Date.now()}.${randomUUID()}.tmp`
  );
  await mkdir(path.dirname(progressPath), { recursive: true });
  await writeFile(tmp, `${JSON.stringify(progress, null, 2)}\n`, "utf8");
  await rename(tmp, progressPath);
}

function todayKey() {
  return new Date().toISOString().slice(0, 10);
}

function updateDailyPulse(progress) {
  const today = todayKey();
  if (!progress.activeDays.includes(today)) {
    progress.activeDays.push(today);
    progress.activeDays.sort();
  }

  let pulse = 0;
  const active = new Set(progress.activeDays);
  const cursor = new Date(`${today}T00:00:00.000Z`);
  while (active.has(cursor.toISOString().slice(0, 10))) {
    pulse += 1;
    cursor.setUTCDate(cursor.getUTCDate() - 1);
  }
  progress.dailyPulse = pulse;
}

function updateLevel(progress) {
  progress.level = Math.max(1, Math.floor(progress.xp / 100) + 1);
}

async function parseBody(req) {
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > maxBodyBytes) {
      throw httpError(413, "request body is too large");
    }
    chunks.push(chunk);
  }
  if (chunks.length === 0) {
    return {};
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw httpError(400, "request body must be valid json");
  }
}

function findTrial(trials, trialId) {
  return trials.find((trial) => trial.id === trialId);
}

function validateSubmission(trial, code) {
  if (!trial) {
    throw httpError(404, "unknown trial");
  }
  if (typeof code !== "string") {
    throw httpError(400, "code must be a string");
  }
  if (Buffer.byteLength(code, "utf8") > maxCodeBytes) {
    throw httpError(413, "submission is too large");
  }
}

function redactedCleanupError(error) {
  return {
    name: error?.name || "Error",
    code: error?.code || "UNKNOWN",
    message: String(error?.message || "cleanup failed").replaceAll(rootDir, "<repo>")
  };
}

async function directorySize(dir) {
  let total = 0;
  const entries = await readdir(dir, { withFileTypes: true }).catch(() => []);
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      total += await directorySize(fullPath);
    } else if (entry.isFile()) {
      total += (await stat(fullPath)).size;
    }
  }
  return total;
}

async function pruneStaleWorkspaces(tmpDir, logger = console) {
  await mkdir(tmpDir, { recursive: true });
  const now = Date.now();
  const entries = [];
  for (const entry of await readdir(tmpDir, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith("run-")) {
      continue;
    }
    const fullPath = path.join(tmpDir, entry.name);
    const info = await stat(fullPath).catch(() => null);
    if (!info) {
      continue;
    }
    entries.push({ fullPath, mtimeMs: info.mtimeMs, size: await directorySize(fullPath) });
  }

  entries.sort((a, b) => a.mtimeMs - b.mtimeMs);
  let retainedCount = entries.length;
  let retainedBytes = entries.reduce((sum, entry) => sum + entry.size, 0);
  for (const entry of entries) {
    const expired = now - entry.mtimeMs > staleWorkspaceMaxAgeMs;
    const overCount = retainedCount > staleWorkspaceMaxCount;
    const overBytes = retainedBytes > staleWorkspaceMaxBytes;
    if (!expired && !overCount && !overBytes) {
      continue;
    }
    try {
      await rm(entry.fullPath, { recursive: true, force: true });
      retainedCount -= 1;
      retainedBytes -= entry.size;
    } catch (error) {
      logger.warn("learnsley workspace cleanup failed", redactedCleanupError(error));
    }
  }
}

async function withRunWorkspace(tmpDir, code, callback, logger) {
  await pruneStaleWorkspaces(tmpDir, logger);
  const runDir = await mkdtemp(path.join(tmpDir, "run-"));
  const sourcePath = path.join(runDir, "main.sley");
  try {
    await writeFile(sourcePath, code, "utf8");
    return await callback(sourcePath);
  } finally {
    await rm(runDir, { recursive: true, force: true }).catch((error) => {
      logger.warn("learnsley workspace cleanup failed", redactedCleanupError(error));
    });
  }
}

function terminateProcessGroup(child, signal = "SIGKILL") {
  if (!child.pid) {
    return;
  }
  try {
    process.kill(-child.pid, signal);
  } catch {
    child.kill(signal);
  }
}

function runSley(sleyBin, args, options = {}) {
  const { cwd = rootDir, timeoutMs = commandTimeoutMs, signal, env = process.env } = options;
  return new Promise((resolve) => {
    const child = spawn(sleyBin, args, {
      cwd,
      detached: process.platform !== "win32",
      shell: false,
      env: { ...env, NO_COLOR: "1" },
      stdio: ["ignore", "pipe", "pipe"]
    });

    let stdout = "";
    let stderr = "";
    let timedOut = false;
    let cancelled = false;
    let outputFlooded = false;
    let settled = false;
    const finish = (result) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      resolve(result);
    };
    const stop = (reason) => {
      if (reason === "timeout") {
        timedOut = true;
      }
      if (reason === "cancelled") {
        cancelled = true;
      }
      if (reason === "outputFlood") {
        outputFlooded = true;
      }
      terminateProcessGroup(child);
    };
    const timer = setTimeout(() => stop("timeout"), timeoutMs);
    const onAbort = () => stop("cancelled");
    if (signal?.aborted) {
      onAbort();
    } else {
      signal?.addEventListener("abort", onAbort, { once: true });
    }

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
      if (Buffer.byteLength(stdout) + Buffer.byteLength(stderr) > maxOutputBytes) {
        stop("outputFlood");
      }
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
      if (Buffer.byteLength(stdout) + Buffer.byteLength(stderr) > maxOutputBytes) {
        stop("outputFlood");
      }
    });
    child.on("error", (error) => {
      signal?.removeEventListener("abort", onAbort);
      finish({ ok: false, code: 127, stdout, stderr: `${stderr}${error.message}`, timedOut, cancelled, outputFlooded });
    });
    child.on("close", (code) => {
      signal?.removeEventListener("abort", onAbort);
      finish({ ok: code === 0 && !timedOut && !cancelled && !outputFlooded, code, stdout, stderr, timedOut, cancelled, outputFlooded });
    });
  });
}

function parseJsonOutput(commandResult) {
  try {
    return JSON.parse(commandResult.stdout);
  } catch {
    return null;
  }
}

async function compileAndRun(appState, code) {
  return withRunWorkspace(appState.tmpDir, code, async (sourcePath) => {
    const check = await runSley(appState.sleyBin, ["check", "--json", sourcePath], appState);
    const checkJson = parseJsonOutput(check);
    if (!check.ok) {
      return { ok: false, phase: "check", check, checkJson, output: null, graphJson: null, sealJson: null };
    }

    const run = await runSley(appState.sleyBin, ["run", "--json", sourcePath], appState);
    const output = parseJsonOutput(run);
    const graph = await runSley(appState.sleyBin, ["graph", "--json", sourcePath], appState);
    const seal = await runSley(appState.sleyBin, ["seal", "--json", sourcePath], appState);

    return {
      ok: run.ok,
      phase: "run",
      check,
      checkJson,
      run,
      output,
      graphJson: parseJsonOutput(graph),
      graphRaw: graph.stdout || graph.stderr,
      sealJson: parseJsonOutput(seal),
      sealRaw: seal.stdout || seal.stderr
    };
  }, appState.logger);
}

function valuesEqual(actual, expected) {
  return JSON.stringify(actual) === JSON.stringify(expected);
}

function keywordSeal(code, seal) {
  const text = code.toLowerCase();
  const missing = [];
  for (const group of seal.required || []) {
    const ok = group.some((term) => text.includes(term.toLowerCase()));
    if (!ok) {
      missing.push(group);
    }
  }
  return { ok: missing.length === 0, missing };
}

async function runTrial(appState, trial, code) {
  if (trial.mode === "explanation") {
    return {
      ok: true,
      phase: "explanation",
      explanation: code,
      checkJson: null,
      output: { kind: "Explanation", value: code },
      graphJson: null,
      sealJson: null
    };
  }
  return compileAndRun(appState, code);
}

async function sealTrial(appState, trial, code) {
  if (trial.seal.kind === "keywords") {
    const result = keywordSeal(code, trial.seal);
    return {
      passed: result.ok,
      reason: result.ok ? "Answer accepted." : "Answer is missing required concepts.",
      detail: result,
      run: await runTrial(appState, trial, code)
    };
  }

  const run = await compileAndRun(appState, code);
  if (!run.ok) {
    return { passed: false, reason: `Program failed during ${run.phase}.`, detail: null, run };
  }

  const passed = valuesEqual(run.output, trial.seal.expected);
  return {
    passed,
    reason: passed ? "Seal accepted." : "Runtime output did not match the seal.",
    detail: { expected: trial.seal.expected, actual: run.output },
    run
  };
}

function applySealProgress(progress, trial, passed) {
  const now = new Date().toISOString();
  const attempt = progress.attempts[trial.id] || { runs: 0, seals: 0, failedSeals: 0 };
  attempt.seals += 1;
  attempt.lastSealAt = now;
  progress.attempts[trial.id] = attempt;
  updateDailyPulse(progress);

  const awards = [];
  if (passed) {
    const alreadyCompleted = Boolean(progress.completed[trial.id]);
    progress.cleanSealChain += 1;
    if (!alreadyCompleted) {
      const xp = trial.rewards?.xp || 0;
      const shards = trial.rewards?.shards || 0;
      progress.xp += xp;
      progress.shards += shards;
      progress.completed[trial.id] = { completedAt: now, xp, shards };
      awards.push(`+${xp} XP`, `+${shards} shards`);
    } else {
      awards.push("practice seal");
    }
    if (progress.cleanSealChain > 0 && progress.cleanSealChain % 3 === 0) {
      progress.proofArmor += 1;
      awards.push("+1 Proof Armor");
    }
  } else {
    attempt.failedSeals += 1;
    if (progress.proofArmor > 0) {
      progress.proofArmor -= 1;
      awards.push("Proof Armor absorbed failure");
    } else {
      progress.cleanSealChain = 0;
      awards.push("Clean Seal Chain reset");
    }
  }

  updateLevel(progress);
  return awards;
}

async function recordRun(appState, trialId) {
  const progress = await loadProgress(appState.progressPath);
  const attempt = progress.attempts[trialId] || { runs: 0, seals: 0, failedSeals: 0 };
  attempt.runs += 1;
  attempt.lastRunAt = new Date().toISOString();
  progress.attempts[trialId] = attempt;
  updateDailyPulse(progress);
  await saveProgress(appState.progressPath, progress);
  return progress;
}

async function serveStatic(req, res, pathname) {
  const safePath = pathname === "/" ? "/index.html" : pathname;
  const filePath = path.normalize(path.join(publicDir, safePath));
  if (!filePath.startsWith(publicDir)) {
    textResponse(res, 403, "forbidden");
    return;
  }

  try {
    const info = await stat(filePath);
    if (!info.isFile()) {
      textResponse(res, 404, "not found");
      return;
    }
    const ext = path.extname(filePath);
    const body = await readFile(filePath);
    res.writeHead(200, {
      "content-type": mimeTypes.get(ext) || "application/octet-stream",
      "cache-control": "no-store"
    });
    res.end(body);
  } catch (error) {
    if (error.code === "ENOENT") {
      textResponse(res, 404, "not found");
      return;
    }
    throw error;
  }
}

async function handleApi(appState, req, res, pathname) {
  const trials = await loadTrials();

  if (req.method === "GET" && pathname === "/api/health") {
    jsonResponse(res, 200, { ok: true, host: appState.host, port: appState.port, compilerConfigured: Boolean(appState.sleyBin) });
    return;
  }

  if (req.method === "GET" && pathname === "/api/session") {
    jsonResponse(res, 200, { ok: true, csrfToken: appState.csrfToken });
    return;
  }

  if (req.method === "GET" && pathname === "/api/trials") {
    jsonResponse(res, 200, { trials: trials.map(publicTrial) });
    return;
  }

  if (req.method === "GET" && pathname === "/api/progress") {
    jsonResponse(res, 200, { progress: await loadProgress(appState.progressPath) });
    return;
  }

  if (req.method === "POST" && pathname === "/api/run") {
    const body = await parseBody(req);
    const trial = findTrial(trials, body.trialId);
    validateSubmission(trial, body.code);
    const run = await runTrial(appState, trial, body.code);
    const progress = await recordRun(appState, trial.id);
    jsonResponse(res, 200, { ok: run.ok, run, progress });
    return;
  }

  if (req.method === "POST" && pathname === "/api/format") {
    const body = await parseBody(req);
    const trial = findTrial(trials, body.trialId);
    validateSubmission(trial, body.code);
    if (trial.mode !== "code") {
      jsonResponse(res, 400, { ok: false, error: "format is only available for code trials" });
      return;
    }
    const result = await withRunWorkspace(appState.tmpDir, body.code, async (sourcePath) => {
      const formatted = await runSley(appState.sleyBin, ["format", sourcePath], appState);
      return { ok: formatted.ok, formatted: formatted.stdout, stderr: formatted.stderr };
    }, appState.logger);
    jsonResponse(res, 200, result);
    return;
  }

  if (req.method === "POST" && pathname === "/api/graph") {
    const body = await parseBody(req);
    const trial = findTrial(trials, body.trialId);
    validateSubmission(trial, body.code);
    if (trial.mode !== "code") {
      jsonResponse(res, 400, { ok: false, error: "graph is only available for code trials" });
      return;
    }
    const result = await withRunWorkspace(appState.tmpDir, body.code, async (sourcePath) => {
      const graph = await runSley(appState.sleyBin, ["graph", "--json", sourcePath], appState);
      return { ok: graph.ok, graph: parseJsonOutput(graph), stdout: graph.stdout, stderr: graph.stderr };
    }, appState.logger);
    jsonResponse(res, 200, result);
    return;
  }

  if (req.method === "POST" && pathname === "/api/seal") {
    const body = await parseBody(req);
    const trial = findTrial(trials, body.trialId);
    validateSubmission(trial, body.code);
    const seal = await sealTrial(appState, trial, body.code);
    const progress = await loadProgress(appState.progressPath);
    const awards = applySealProgress(progress, trial, seal.passed);
    await saveProgress(appState.progressPath, progress);
    jsonResponse(res, 200, { ok: seal.passed, seal, progress, awards });
    return;
  }

  jsonResponse(res, 404, { ok: false, error: "unknown api route" });
}

export function createLearnSleyServer(options = {}) {
  const workDir = options.workDir || process.env.LEARNSLEY_WORKDIR || defaultWorkDir;
  const appState = {
    host: options.host || process.env.LEARNSLEY_HOST || "127.0.0.1",
    port: Number.parseInt(String(options.port || process.env.PORT || "4179"), 10),
    sleyBin: options.sleyBin || process.env.SLEY_BIN || defaultSleyBin,
    progressPath: options.progressPath || process.env.LEARNSLEY_PROGRESS || path.join(workDir, "progress.json"),
    tmpDir: options.tmpDir || path.join(workDir, "tmp"),
    csrfToken: options.csrfToken || randomBytes(32).toString("base64url"),
    timeoutMs: options.timeoutMs || commandTimeoutMs,
    cwd: options.cwd || rootDir,
    logger: options.logger || console
  };

  if (!isLoopbackHostName(appState.host)) {
    throw new Error("LEARNSLEY_HOST must be a loopback address for the compiler-backed service");
  }

  const server = createServer(async (req, res) => {
    try {
      validateHost(req);
      const url = new URL(req.url || "/", requestOrigin(req));
      validateMutationRequest(req, url.pathname, appState.csrfToken);
      const requestState = { ...appState };
      if (mutationRoutes.has(url.pathname)) {
        const controller = new AbortController();
        req.on("aborted", () => controller.abort());
        requestState.signal = controller.signal;
      }
      if (url.pathname.startsWith("/api/")) {
        await handleApi(requestState, req, res, url.pathname);
        return;
      }
      await serveStatic(req, res, url.pathname);
    } catch (error) {
      const status = error.statusCode || 500;
      jsonResponse(res, status, { ok: false, error: error.message || "internal error" });
    }
  });

  return { server, appState };
}

export async function startLearnSley(options = {}) {
  const instance = createLearnSleyServer(options);
  await mkdir(path.dirname(instance.appState.progressPath), { recursive: true });
  await mkdir(instance.appState.tmpDir, { recursive: true });
  await pruneStaleWorkspaces(instance.appState.tmpDir, instance.appState.logger);
  await new Promise((resolve) => instance.server.listen(instance.appState.port, instance.appState.host, resolve));
  return instance;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  startLearnSley().then(({ appState }) => {
    console.log(`LearnSley local server: http://${appState.host}:${appState.port}`);
    console.log("Sley binary: <configured>");
  }).catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}

export const testInternals = {
  isLoopbackHostName,
  pruneStaleWorkspaces,
  runSley,
  staleWorkspaceMaxCount,
  withRunWorkspace
};
