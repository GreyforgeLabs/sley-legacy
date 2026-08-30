#!/usr/bin/env node
import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { createReadStream, lstatSync, readFileSync, realpathSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const moduleFile = fileURLToPath(import.meta.url);
const repositoryRoot = resolve(dirname(moduleFile), "..");
const packageMetadata = JSON.parse(readFileSync(join(repositoryRoot, "package.json"), "utf8"));
export const VERSION = packageMetadata.version;

const DEFAULT_PORT = 4173;
const DEFAULT_TIMEOUT_MS = 15_000;
const DEFAULT_MAX_OUTPUT_BYTES = 256 * 1024;
const MAX_BODY_BYTES = 16 * 1024;

const commandCommands = Object.freeze({
  doctor: { args: ["doctor", "--json"], mutates: false },
  verify: { args: ["verify", "--json"], mutates: false },
  query: { args: ["query", "--json", "--kind", "all"], mutates: false },
  lint: { args: ["lint", "--json"], mutates: false },
  graph: { args: ["graph", "--json"], mutates: false },
  plan: { args: ["plan", "--json", "--graft-templates"], mutates: false },
  seal: { args: ["seal", "--json"], mutates: false },
  zjx: { args: ["zjx", "--json"], mutates: false },
  check: { args: ["check", "--json"], mutates: false },
  format: { args: ["format"], mutates: true },
});

class WorkbenchError extends Error {
  constructor(code, message, status = 400) {
    super(message);
    this.code = code;
    this.status = status;
  }
}

function parseInteger(name, value, minimum, maximum) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new WorkbenchError("INVALID_ARGUMENT", `${name} must be between ${minimum} and ${maximum}`);
  }
  return parsed;
}

function isLoopbackHost(host) {
  return ["127.0.0.1", "::1", "localhost"].includes(String(host).toLowerCase());
}

function formatOriginHost(host, port) {
  return `${host.includes(":") && !host.startsWith("[") ? `[${host}]` : host}:${port}`;
}

function validateOrigin(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new WorkbenchError("INVALID_ARGUMENT", "--origin must be an absolute HTTP(S) origin");
  }
  if (
    !["http:", "https:"].includes(parsed.protocol) ||
    parsed.username ||
    parsed.password ||
    parsed.pathname !== "/" ||
    parsed.search ||
    parsed.hash
  ) {
    throw new WorkbenchError("INVALID_ARGUMENT", "--origin must contain only scheme, host, and port");
  }
  return parsed.origin;
}

export function parseCliArgs(argv, env = process.env) {
  const options = {
    host: env.SLEY_WORKBENCH_HOST || "127.0.0.1",
    port: parseInteger("port", env.PORT || DEFAULT_PORT, 1, 65535),
    target: env.SLEY_TARGET || ".",
    sleyBin: env.SLEY_BIN || "sley",
    timeoutMs: DEFAULT_TIMEOUT_MS,
    maxOutputBytes: DEFAULT_MAX_OUTPUT_BYTES,
    allowRemote: false,
    allowMutation: false,
    origin: null,
    help: false,
  };
  const valueFlags = new Map([
    ["--host", "host"],
    ["--port", "port"],
    ["--target", "target"],
    ["--sley-bin", "sleyBin"],
    ["--timeout-ms", "timeoutMs"],
    ["--max-output-bytes", "maxOutputBytes"],
    ["--origin", "origin"],
  ]);
  const seen = new Set();
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (["--allow-remote", "--allow-mutation", "--help"].includes(flag)) {
      if (seen.has(flag)) throw new WorkbenchError("INVALID_ARGUMENT", `duplicate option ${flag}`);
      seen.add(flag);
      if (flag === "--allow-remote") options.allowRemote = true;
      if (flag === "--allow-mutation") options.allowMutation = true;
      if (flag === "--help") options.help = true;
      continue;
    }
    const key = valueFlags.get(flag);
    if (!key) throw new WorkbenchError("INVALID_ARGUMENT", `unknown option ${flag}`);
    if (seen.has(flag)) throw new WorkbenchError("INVALID_ARGUMENT", `duplicate option ${flag}`);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new WorkbenchError("INVALID_ARGUMENT", `${flag} requires a value`);
    }
    seen.add(flag);
    options[key] = value;
    index += 1;
  }
  options.port = parseInteger("port", options.port, 1, 65535);
  options.timeoutMs = parseInteger("timeout-ms", options.timeoutMs, 100, 60_000);
  options.maxOutputBytes = parseInteger(
    "max-output-bytes",
    options.maxOutputBytes,
    1024,
    1024 * 1024,
  );
  if (!isLoopbackHost(options.host) && !options.allowRemote) {
    throw new WorkbenchError("REMOTE_BIND_DENIED", "non-loopback binding requires --allow-remote");
  }
  if (options.allowRemote && !options.origin) {
    throw new WorkbenchError("ORIGIN_REQUIRED", "remote binding requires an explicit --origin");
  }
  options.origin = validateOrigin(
    options.origin || `http://${formatOriginHost(options.host, options.port)}`,
  );
  return options;
}

function within(root, candidate) {
  const rel = relative(root, candidate);
  return rel === "" || (!rel.startsWith(`..${sep}`) && rel !== ".." && !isAbsolute(rel));
}

export function resolveWorkspaceTarget(workspaceRoot, candidate = ".") {
  if (typeof candidate !== "string" || !candidate || candidate.includes("\0") || isAbsolute(candidate)) {
    throw new WorkbenchError("TARGET_OUTSIDE_WORKSPACE", "target must be a relative workspace path", 403);
  }
  const root = realpathSync(resolve(workspaceRoot));
  const lexical = resolve(root, candidate);
  if (!within(root, lexical)) {
    throw new WorkbenchError("TARGET_OUTSIDE_WORKSPACE", "target escapes the configured workspace", 403);
  }
  let real;
  try {
    real = realpathSync(lexical);
  } catch {
    throw new WorkbenchError("TARGET_NOT_FOUND", "target does not exist", 404);
  }
  if (!within(root, real)) {
    throw new WorkbenchError("TARGET_OUTSIDE_WORKSPACE", "target resolves outside the configured workspace", 403);
  }
  const metadata = lstatSync(real);
  if (!metadata.isFile() && !metadata.isDirectory()) {
    throw new WorkbenchError("TARGET_TYPE_DENIED", "target must be a regular file or directory", 403);
  }
  return real;
}

function terminateChild(child) {
  if (!child?.pid) return;
  try {
    if (process.platform !== "win32") process.kill(-child.pid, "SIGKILL");
    else child.kill("SIGKILL");
  } catch {
    try {
      child.kill("SIGKILL");
    } catch {
      // The child may already have exited.
    }
  }
}

export function runBoundedCommand(
  executable,
  args,
  { timeoutMs = DEFAULT_TIMEOUT_MS, maxOutputBytes = DEFAULT_MAX_OUTPUT_BYTES, signal, spawnImpl = spawn } = {},
) {
  return new Promise((resolveResult) => {
    let child;
    try {
      child = spawnImpl(executable, args, {
        detached: process.platform !== "win32",
        env: { PATH: process.env.PATH || "", NO_COLOR: "1" },
        shell: false,
        stdio: ["ignore", "pipe", "pipe"],
      });
    } catch (error) {
      resolveResult({ ok: false, status: null, stdout: "", stderr: error.name, spawnFailed: true });
      return;
    }

    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let totalBytes = 0;
    let timedOut = false;
    let cancelled = false;
    let outputFlooded = false;
    let settled = false;
    let timer;

    const finish = (status, extra = {}) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      resolveResult({
        ok: status === 0 && !timedOut && !cancelled && !outputFlooded && !extra.spawnFailed,
        status,
        stdout: stdout.toString("utf8"),
        stderr: stderr.toString("utf8"),
        timedOut,
        cancelled,
        outputFlooded,
        ...extra,
      });
    };

    const stop = (reason) => {
      if (reason === "timeout") timedOut = true;
      if (reason === "cancel") cancelled = true;
      if (reason === "output") outputFlooded = true;
      terminateChild(child);
    };

    const append = (stream, chunk) => {
      const data = Buffer.from(chunk);
      const remaining = Math.max(0, maxOutputBytes - totalBytes);
      const kept = data.subarray(0, remaining);
      if (stream === "stdout") stdout = Buffer.concat([stdout, kept]);
      else stderr = Buffer.concat([stderr, kept]);
      totalBytes += data.length;
      if (totalBytes > maxOutputBytes) stop("output");
    };

    const onAbort = () => stop("cancel");
    child.stdout?.on("data", (chunk) => append("stdout", chunk));
    child.stderr?.on("data", (chunk) => append("stderr", chunk));
    child.once("error", (error) => finish(null, { spawnFailed: true, error: error.name }));
    child.once("close", (status) => finish(status));
    timer = setTimeout(() => stop("timeout"), timeoutMs);
    if (signal?.aborted) onAbort();
    else signal?.addEventListener("abort", onAbort, { once: true });
  });
}

function sendJson(res, status, payload) {
  const body = `${JSON.stringify(payload, null, 2)}\n`;
  res.writeHead(status, {
    "cache-control": "no-store",
    "content-length": Buffer.byteLength(body),
    "content-type": "application/json; charset=utf-8",
  });
  res.end(body);
}

function sendError(res, error) {
  const known = error instanceof WorkbenchError;
  sendJson(res, known ? error.status : 500, {
    ok: false,
    error: {
      code: known ? error.code : "INTERNAL_ERROR",
      message: known ? error.message : "unexpected workbench failure",
    },
  });
}

async function readJsonBody(req) {
  const chunks = [];
  let bytes = 0;
  for await (const chunk of req) {
    bytes += chunk.length;
    if (bytes > MAX_BODY_BYTES) throw new WorkbenchError("BODY_TOO_LARGE", "request body is too large", 413);
    chunks.push(chunk);
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
  } catch {
    throw new WorkbenchError("MALFORMED_JSON", "request body must be valid JSON");
  }
}

function validateHost(req, state) {
  if (String(req.headers.host || "").toLowerCase() !== new URL(state.origin).host.toLowerCase()) {
    throw new WorkbenchError("HOST_DENIED", "request Host does not match the configured origin", 403);
  }
}

function validateMutationBoundary(req, state) {
  if (req.method !== "POST") throw new WorkbenchError("METHOD_NOT_ALLOWED", "command execution requires POST", 405);
  if (String(req.headers.origin || "") !== state.origin) {
    throw new WorkbenchError("ORIGIN_DENIED", "command execution requires the configured Origin", 403);
  }
  if (req.headers["x-sley-workbench-csrf"] !== state.csrfToken) {
    throw new WorkbenchError("CSRF_DENIED", "invalid CSRF token", 403);
  }
  if (!String(req.headers["content-type"] || "").toLowerCase().startsWith("application/json")) {
    throw new WorkbenchError("CONTENT_TYPE_DENIED", "command execution requires application/json", 415);
  }
}

function listCommands(state) {
  return Object.entries(commandCommands).map(([name, command]) => ({
    name,
    mutates: command.mutates,
    enabled: !command.mutates || state.allowMutation,
  }));
}

function staticMime(file) {
  return new Map([
    [".css", "text/css; charset=utf-8"],
    [".html", "text/html; charset=utf-8"],
    [".js", "text/javascript; charset=utf-8"],
    [".json", "application/json; charset=utf-8"],
    [".png", "image/png"],
    [".svg", "image/svg+xml"],
  ]).get(extname(file)) || "application/octet-stream";
}

function resolveStaticFile(staticRoot, pathname) {
  let decoded;
  try {
    decoded = decodeURIComponent(pathname);
  } catch {
    throw new WorkbenchError("INVALID_PATH", "invalid static path", 400);
  }
  const relativeName = decoded === "/" ? "index.html" : decoded.replace(/^\/+/, "");
  const lexical = resolve(staticRoot, relativeName);
  if (!within(staticRoot, lexical)) throw new WorkbenchError("INVALID_PATH", "static path escapes root", 403);
  let real;
  try {
    real = realpathSync(lexical);
  } catch {
    throw new WorkbenchError("NOT_FOUND", "static asset not found", 404);
  }
  if (!within(staticRoot, real) || !lstatSync(real).isFile()) {
    throw new WorkbenchError("NOT_FOUND", "static asset not found", 404);
  }
  return real;
}

async function serveStatic(req, res, state, pathname) {
  if (!["GET", "HEAD"].includes(req.method)) {
    res.setHeader("allow", "GET, HEAD");
    throw new WorkbenchError("METHOD_NOT_ALLOWED", "static assets allow GET and HEAD", 405);
  }
  const file = resolveStaticFile(state.staticRoot, pathname);
  const size = lstatSync(file).size;
  if (req.method === "HEAD") {
    res.writeHead(200, { "content-length": size, "content-type": staticMime(file) });
    res.end();
    return;
  }
  await new Promise((resolveStream) => {
    const stream = state.createReadStream(file);
    let opened = false;
    stream.once("open", () => {
      opened = true;
      res.writeHead(200, { "content-length": size, "content-type": staticMime(file) });
      stream.pipe(res);
    });
    stream.once("error", () => {
      if (!opened && !res.headersSent) {
        sendError(res, new WorkbenchError("STREAM_ERROR", "static asset could not be read", 500));
      } else {
        res.destroy();
      }
      resolveStream();
    });
    stream.once("end", resolveStream);
  });
}

async function handleApi(req, res, state, url) {
  if (url.pathname === "/api/health" && req.method === "GET") {
    sendJson(res, 200, { ok: true, version: VERSION, mode: state.allowMutation ? "mutation-enabled" : "read-only" });
    return;
  }
  if (url.pathname === "/api/session" && req.method === "GET") {
    sendJson(res, 200, { ok: true, csrfToken: state.csrfToken, version: VERSION });
    return;
  }
  if (url.pathname === "/api/commands" && req.method === "GET") {
    sendJson(res, 200, { ok: true, commands: listCommands(state) });
    return;
  }
  if (url.pathname === "/api/file-size" && req.method === "GET") {
    const target = resolveWorkspaceTarget(state.workspaceRoot, url.searchParams.get("path") || ".");
    const metadata = lstatSync(target);
    if (!metadata.isFile()) throw new WorkbenchError("NOT_REGULAR_FILE", "path is not a regular file", 400);
    sendJson(res, 200, { ok: true, size: metadata.size });
    return;
  }
  if (url.pathname === "/api/run") {
    validateMutationBoundary(req, state);
    const body = await readJsonBody(req);
    const command = commandCommands[body.command];
    if (!command) throw new WorkbenchError("UNKNOWN_COMMAND", "unsupported command");
    if (command.mutates && !state.allowMutation) {
      throw new WorkbenchError("MUTATION_DISABLED", "mutation commands require --allow-mutation", 403);
    }
    const target = resolveWorkspaceTarget(state.workspaceRoot, body.target || ".");
    const controller = new AbortController();
    const abort = () => controller.abort();
    req.once("aborted", abort);
    res.once("close", abort);
    const result = await runBoundedCommand(
      state.sleyBin,
      [...command.args, target],
      {
        timeoutMs: state.timeoutMs,
        maxOutputBytes: state.maxOutputBytes,
        signal: controller.signal,
        spawnImpl: state.spawnImpl,
      },
    );
    req.removeListener("aborted", abort);
    res.removeListener("close", abort);
    let parsed = null;
    try {
      parsed = JSON.parse(result.stdout);
    } catch {
      parsed = result.stdout;
    }
    sendJson(res, 200, { ok: result.ok, command: body.command, ...result, parsed });
    return;
  }
  if (url.pathname.startsWith("/api/")) {
    throw new WorkbenchError("NOT_FOUND", "API route not found", 404);
  }
  await serveStatic(req, res, state, url.pathname);
}

function normalizeServerOptions(options) {
  const host = options.host || "127.0.0.1";
  const port = options.port ?? DEFAULT_PORT;
  const allowRemote = Boolean(options.allowRemote);
  if (!isLoopbackHost(host) && !allowRemote) {
    throw new WorkbenchError("REMOTE_BIND_DENIED", "non-loopback binding requires explicit authorization");
  }
  const origin = validateOrigin(
    options.origin || `http://${formatOriginHost(host, port)}`,
  );
  if (allowRemote && !options.origin) {
    throw new WorkbenchError("ORIGIN_REQUIRED", "remote binding requires an explicit origin");
  }
  return {
    allowMutation: Boolean(options.allowMutation),
    allowRemote,
    createReadStream: options.createReadStream || createReadStream,
    csrfToken: options.csrfToken || randomBytes(32).toString("base64url"),
    host,
    maxOutputBytes: options.maxOutputBytes || DEFAULT_MAX_OUTPUT_BYTES,
    origin,
    port,
    sleyBin: options.sleyBin || "sley",
    spawnImpl: options.spawnImpl || spawn,
    staticRoot: realpathSync(resolve(options.staticRoot || join(repositoryRoot, "static"))),
    timeoutMs: options.timeoutMs || DEFAULT_TIMEOUT_MS,
    workspaceRoot: realpathSync(resolve(options.target || ".")),
  };
}

export function createWorkbenchServer(options = {}) {
  const state = normalizeServerOptions(options);
  const server = createServer(async (req, res) => {
    try {
      validateHost(req, state);
      const url = new URL(req.url || "/", state.origin);
      await handleApi(req, res, state, url);
    } catch (error) {
      if (!res.headersSent) sendError(res, error);
      else res.destroy();
    }
  });
  return { server, state };
}

export async function startWorkbench(options = {}) {
  const instance = createWorkbenchServer(options);
  await new Promise((resolveListen, reject) => {
    instance.server.once("error", reject);
    instance.server.listen(instance.state.port, instance.state.host, resolveListen);
  });
  if (instance.state.port === 0 && !options.origin) {
    instance.state.port = instance.server.address().port;
    instance.state.origin = validateOrigin(
      `http://${formatOriginHost(instance.state.host, instance.state.port)}`,
    );
  }
  return instance;
}

function printUsage() {
  process.stdout.write(`sley-workbench ${VERSION}\n`);
  process.stdout.write("usage: sley-workbench [--target PATH] [--port N] [--host HOST]\n");
  process.stdout.write("       [--timeout-ms N] [--max-output-bytes N] [--sley-bin PATH]\n");
  process.stdout.write("       [--allow-mutation] [--allow-remote --origin URL]\n");
}

if (process.argv[1] === moduleFile) {
  try {
    const options = parseCliArgs(process.argv.slice(2));
    if (options.help) printUsage();
    else {
      startWorkbench(options).then(({ state }) => {
        process.stdout.write(`sley-workbench ${VERSION} listening on ${state.origin} (${state.allowMutation ? "mutation-enabled" : "read-only"})\n`);
      }).catch((error) => {
        process.stderr.write(`${error.code || "START_ERROR"}: ${error.message}\n`);
        process.exitCode = 1;
      });
    }
  } catch (error) {
    process.stderr.write(`${error.code || "ARGUMENT_ERROR"}: ${error.message}\n`);
    process.exitCode = 2;
  }
}
