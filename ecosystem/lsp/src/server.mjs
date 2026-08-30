#!/usr/bin/env node
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { TextDecoder } from "node:util";
import { fileURLToPath } from "node:url";

const VERSION = "0.0.1-private";
const DEFAULT_COMPILER_TIMEOUT_MS = 10_000;
const DEFAULT_COMPILER_MAX_BUFFER = 4 * 1024 * 1024;
const DEFAULT_HEADER_LIMIT = 8 * 1024;
const DEFAULT_BODY_LIMIT = 4 * 1024 * 1024;

const documents = new Map();
let workspaceRoots = [];

class RequestError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

function positiveInteger(value, fallback) {
  if (value === undefined || value === "") return fallback;
  if (!/^[1-9][0-9]*$/.test(String(value))) return fallback;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : fallback;
}

export function runSley(args, options = {}) {
  const executable = options.executable ?? process.env.SLEY_BIN ?? "sley";
  const prefixArgs = options.prefixArgs ?? [];
  const timeout = positiveInteger(
    options.timeout ?? process.env.SLEY_LSP_COMPILER_TIMEOUT_MS,
    DEFAULT_COMPILER_TIMEOUT_MS,
  );
  const maxBuffer = positiveInteger(
    options.maxBuffer ?? process.env.SLEY_LSP_COMPILER_MAX_BUFFER,
    DEFAULT_COMPILER_MAX_BUFFER,
  );
  const result = spawnSync(executable, [...prefixArgs, ...args], {
    encoding: "utf8",
    timeout,
    maxBuffer,
    killSignal: "SIGKILL",
    windowsHide: true,
    input: options.input,
  });
  const errorCode = result.error?.code ?? null;
  return {
    ok: result.status === 0 && !result.error,
    status: Number.isInteger(result.status) ? result.status : null,
    signal: result.signal ?? null,
    timedOut: errorCode === "ETIMEDOUT",
    outputLimited: errorCode === "ENOBUFS",
    errorCode,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

export function normalizeUri(uri) {
  if (typeof uri !== "string" || !uri.startsWith("file:")) {
    throw new RequestError(-32602, "text document URI must use the file scheme");
  }
  try {
    return resolve(fileURLToPath(uri));
  } catch {
    throw new RequestError(-32602, "invalid file URI");
  }
}

function pathInside(candidate, root) {
  const rel = relative(root, candidate);
  return rel === "" || (!rel.startsWith("..") && !isAbsolute(rel));
}

function canonicalPath(candidate) {
  if (existsSync(candidate)) return realpathSync(candidate);
  return resolve(realpathSync(dirname(candidate)), basename(candidate));
}

function requireWorkspacePath(uri) {
  const filePath = canonicalPath(normalizeUri(uri));
  if (workspaceRoots.length === 0 || !workspaceRoots.some((root) => pathInside(filePath, root))) {
    throw new RequestError(-32602, "text document is outside initialized workspace roots");
  }
  return filePath;
}

function resolveWorkspaceTarget(target) {
  const candidate = canonicalPath(resolve(target));
  if (workspaceRoots.length === 0 || !workspaceRoots.some((root) => pathInside(candidate, root))) {
    throw new RequestError(-32602, "target is outside initialized workspace roots");
  }
  return candidate;
}

function configureWorkspaceRoots(params = {}) {
  const roots = [];
  for (const folder of params.workspaceFolders ?? []) {
    if (folder?.uri) roots.push(canonicalPath(normalizeUri(folder.uri)));
  }
  if (roots.length === 0 && params.rootUri) roots.push(canonicalPath(normalizeUri(params.rootUri)));
  if (roots.length === 0 && typeof params.rootPath === "string") roots.push(canonicalPath(resolve(params.rootPath)));
  workspaceRoots = [...new Set(roots)];
  documents.clear();
}

function readText(uri) {
  if (documents.has(uri)) return documents.get(uri).text;
  return readFileSync(requireWorkspacePath(uri), "utf8");
}

function positionFromCodeUnitOffset(text, offset) {
  const safeOffset = Math.max(0, Math.min(Number.isFinite(offset) ? offset : 0, text.length));
  const lines = text.slice(0, safeOffset).split(/\r?\n/);
  return { line: lines.length - 1, character: lines.at(-1).length };
}

export function positionFromUtf8Offset(text, byteOffset) {
  const bytes = Buffer.from(text, "utf8");
  const safeOffset = Math.max(0, Math.min(Number.isFinite(byteOffset) ? byteOffset : 0, bytes.length));
  const prefix = bytes.subarray(0, safeOffset).toString("utf8");
  return positionFromCodeUnitOffset(prefix, prefix.length);
}

function diagnosticsFromCheck(uri) {
  const text = readText(uri);
  const scratch = mkdtempSync(join(tmpdir(), "sley-lsp-"));
  try {
    const filePath = join(scratch, "document.sley");
    writeFileSync(filePath, text, "utf8");
    const result = runSley(["check", "--json", filePath]);
    const diagnostics = [];
    if (!result.ok) {
      try {
        const report = JSON.parse(result.stdout);
        for (const finding of report.findings ?? []) {
          const start = Number.parseInt(finding.start?.offset ?? 0, 10);
          const end = Number.parseInt(finding.end?.offset ?? start, 10);
          diagnostics.push({
            range: {
              start: positionFromUtf8Offset(text, Number.isFinite(start) ? start : 0),
              end: positionFromUtf8Offset(text, Number.isFinite(end) ? end : start),
            },
            severity: 1,
            message: finding.message ?? "diagnostic",
            source: "sley",
          });
        }
      } catch {
        const reason = result.timedOut
          ? "Sley check timed out"
          : result.outputLimited
            ? "Sley check exceeded its output limit"
            : "Sley check failed";
        diagnostics.push({
          range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
          severity: 1,
          message: reason,
          source: "sley",
        });
      }
    }
    return { ok: result.ok, diagnostics, raw: result };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

function regexSymbols(text) {
  const patterns = [
    { kind: 5, pattern: /\bexport\s+task\s+([A-Za-z_][A-Za-z0-9_]*)/g, group: 1 },
    { kind: 5, pattern: /\bexport\s+type\s+([A-Za-z_][A-Za-z0-9_]*)/g, group: 1 },
    { kind: 5, pattern: /\bexport\s+effect\s+([A-Za-z_][A-Za-z0-9_]*)/g, group: 1 },
    { kind: 7, pattern: /\b(type|effect|task)\s+([A-Za-z_][A-Za-z0-9_]*)/g, group: 2 },
  ];
  const symbols = [];
  for (const spec of patterns) {
    for (const match of text.matchAll(spec.pattern)) {
      const name = match[spec.group];
      const nameOffset = match.index + match[0].lastIndexOf(name);
      const start = positionFromCodeUnitOffset(text, nameOffset);
      symbols.push({
        name,
        kind: spec.kind,
        range: {
          start,
          end: { line: start.line, character: start.character + name.length },
        },
      });
    }
  }
  return symbols;
}

function publishDiagnostics(uri, diagnostics) {
  return {
    jsonrpc: "2.0",
    method: "textDocument/publishDiagnostics",
    params: { uri, diagnostics },
  };
}

export function initializeResponse(id) {
  return {
    jsonrpc: "2.0",
    id,
    result: {
      capabilities: {
        textDocumentSync: 1,
        documentFormattingProvider: true,
        documentSymbolProvider: true,
        hoverProvider: true,
      },
      serverInfo: { name: "sley-lsp", version: VERSION },
    },
  };
}

function hasRequestId(message) {
  return Object.prototype.hasOwnProperty.call(message ?? {}, "id");
}

function requestError(message, code, text) {
  return { jsonrpc: "2.0", id: message?.id ?? null, error: { code, message: text } };
}

function dispatchJsonRpc(message) {
  if (!message || message.jsonrpc !== "2.0" || typeof message.method !== "string") {
    if (hasRequestId(message)) return requestError(message, -32600, "Invalid Request");
    return null;
  }

  if (message.method === "initialize") {
    configureWorkspaceRoots(message.params);
    return initializeResponse(message.id);
  }
  if (message.method === "shutdown") return { jsonrpc: "2.0", id: message.id, result: null };
  if (message.method === "exit" || message.method === "initialized") return null;

  if (message.method === "textDocument/didOpen") {
    const uri = message.params?.textDocument?.uri;
    const text = message.params?.textDocument?.text;
    if (typeof uri !== "string" || typeof text !== "string") return null;
    requireWorkspacePath(uri);
    documents.set(uri, { text });
    return publishDiagnostics(uri, diagnosticsFromCheck(uri).diagnostics);
  }

  if (message.method === "textDocument/didChange") {
    const uri = message.params?.textDocument?.uri;
    const changes = message.params?.contentChanges;
    if (typeof uri !== "string" || !Array.isArray(changes) || changes.length === 0) return null;
    requireWorkspacePath(uri);
    const replacement = changes.at(-1)?.text;
    if (typeof replacement !== "string") return null;
    documents.set(uri, { text: replacement });
    return publishDiagnostics(uri, diagnosticsFromCheck(uri).diagnostics);
  }

  if (message.method === "textDocument/didClose") {
    const uri = message.params?.textDocument?.uri;
    if (typeof uri !== "string") return null;
    documents.delete(uri);
    return publishDiagnostics(uri, []);
  }

  if (message.method === "textDocument/documentSymbol") {
    const uri = message.params?.textDocument?.uri;
    if (!uri) throw new RequestError(-32602, "missing textDocument.uri");
    const symbols = regexSymbols(readText(uri)).map((symbol) => ({
      name: symbol.name,
      kind: symbol.kind,
      location: { uri, range: symbol.range },
    }));
    return { jsonrpc: "2.0", id: message.id, result: symbols };
  }

  if (message.method === "textDocument/hover") {
    const uri = message.params?.textDocument?.uri;
    const pos = message.params?.position;
    if (!uri || !pos) throw new RequestError(-32602, "missing hover document or position");
    requireWorkspacePath(uri);
    return {
      jsonrpc: "2.0",
      id: message.id,
      result: {
        contents: [{ language: "sley", value: "Line " + pos.line + ", character " + pos.character }],
      },
    };
  }

  if (message.method === "sley/checkDocument") {
    const uri = message.params?.textDocument?.uri;
    if (!uri) throw new RequestError(-32602, "missing textDocument.uri");
    return { jsonrpc: "2.0", id: message.id, result: diagnosticsFromCheck(uri) };
  }

  if (message.method === "sley/checkTarget" || message.method === "sley/plan") {
    const target = resolveWorkspaceTarget(message.params?.target ?? ".");
    const args = message.method === "sley/checkTarget"
      ? ["check", "--json", target]
      : ["plan", "--json", "--graft-templates", target];
    return { jsonrpc: "2.0", id: message.id, result: runSley(args) };
  }

  if (message.method === "textDocument/formatting") {
    const uri = message.params?.textDocument?.uri;
    if (!uri) throw new RequestError(-32602, "missing textDocument.uri");
    const text = readText(uri);
    const result = runSley(["format", "--stdin"], { input: text });
    if (!result.ok) throw new RequestError(-32603, "Sley formatting failed");
    if (result.stdout === text) return { jsonrpc: "2.0", id: message.id, result: [] };
    return {
      jsonrpc: "2.0",
      id: message.id,
      result: [{
        range: {
          start: { line: 0, character: 0 },
          end: positionFromCodeUnitOffset(text, text.length),
        },
        newText: result.stdout,
      }],
    };
  }

  if (hasRequestId(message)) return requestError(message, -32601, "Method not found");
  return null;
}

export function handleJsonRpc(message) {
  try {
    return dispatchJsonRpc(message);
  } catch (error) {
    if (!hasRequestId(message)) return null;
    if (error instanceof RequestError) return requestError(message, error.code, error.message);
    return requestError(message, -32603, error instanceof Error ? error.message : "Internal error");
  }
}

function findHeaderDelimiter(buffer) {
  const crlf = buffer.indexOf("\r\n\r\n");
  const lf = buffer.indexOf("\n\n");
  if (crlf >= 0 && (lf < 0 || crlf <= lf)) return { index: crlf, length: 4 };
  if (lf >= 0) return { index: lf, length: 2 };
  return null;
}

export class LspMessageFramer {
  constructor({
    onMessage,
    onError,
    maxHeaderBytes = DEFAULT_HEADER_LIMIT,
    maxBodyBytes = DEFAULT_BODY_LIMIT,
  }) {
    this.onMessage = onMessage;
    this.onError = onError;
    this.maxHeaderBytes = maxHeaderBytes;
    this.maxBodyBytes = maxBodyBytes;
    this.buffer = Buffer.alloc(0);
    this.fatal = false;
  }

  fail(message, fatal = true) {
    this.onError({ jsonrpc: "2.0", id: null, error: { code: -32700, message } });
    if (fatal) {
      this.fatal = true;
      this.buffer = Buffer.alloc(0);
    }
  }

  push(chunk) {
    if (this.fatal) return;
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    if (this.buffer.length + bytes.length > this.maxHeaderBytes + this.maxBodyBytes) {
      this.fail("LSP frame exceeds configured buffer limit");
      return;
    }
    this.buffer = Buffer.concat([this.buffer, bytes]);
    this.parse();
  }

  parse() {
    while (!this.fatal) {
      const delimiter = findHeaderDelimiter(this.buffer);
      if (!delimiter) {
        if (this.buffer.length > this.maxHeaderBytes) this.fail("LSP header exceeds configured limit");
        return;
      }
      if (delimiter.index > this.maxHeaderBytes) {
        this.fail("LSP header exceeds configured limit");
        return;
      }
      const headerBytes = this.buffer.subarray(0, delimiter.index);
      if ([...headerBytes].some((byte) => byte > 0x7f)) {
        this.fail("LSP headers must be ASCII");
        return;
      }
      const headerText = headerBytes.toString("ascii");
      const contentLengths = [];
      for (const line of headerText.split(/\r?\n/)) {
        const separator = line.indexOf(":");
        if (separator <= 0) {
          this.fail("Malformed LSP header line");
          return;
        }
        const name = line.slice(0, separator).trim().toLowerCase();
        const value = line.slice(separator + 1).trim();
        if (name === "content-length") contentLengths.push(value);
      }
      if (contentLengths.length !== 1 || !/^(0|[1-9][0-9]*)$/.test(contentLengths[0])) {
        this.fail("LSP frame requires exactly one valid Content-Length header");
        return;
      }
      const bodyLength = Number(contentLengths[0]);
      if (!Number.isSafeInteger(bodyLength) || bodyLength > this.maxBodyBytes) {
        this.fail("LSP body exceeds configured limit");
        return;
      }
      const bodyStart = delimiter.index + delimiter.length;
      const frameEnd = bodyStart + bodyLength;
      if (this.buffer.length < frameEnd) return;
      const body = this.buffer.subarray(bodyStart, frameEnd);
      this.buffer = this.buffer.subarray(frameEnd);
      let text;
      try {
        text = new TextDecoder("utf-8", { fatal: true }).decode(body);
      } catch {
        this.fail("LSP body is not valid UTF-8", false);
        continue;
      }
      try {
        this.onMessage(JSON.parse(text));
      } catch {
        this.fail("LSP body is not valid JSON", false);
      }
    }
  }
}

export function frameMessage(message) {
  const body = Buffer.from(JSON.stringify(message), "utf8");
  return Buffer.concat([
    Buffer.from("Content-Length: " + body.length + "\r\n\r\n", "ascii"),
    body,
  ]);
}

function send(message) {
  if (!message) return;
  process.stdout.write(frameMessage(message));
}

function startServer() {
  const framer = new LspMessageFramer({
    onMessage(message) {
      const response = handleJsonRpc(message);
      if (response) send(response);
      if (message?.method === "exit") process.exit(0);
    },
    onError(error) {
      send(error);
      process.stdin.pause();
      process.exitCode = 1;
    },
  });
  process.stdout.on("error", () => process.exit(0));
  process.stdin.on("data", (chunk) => framer.push(chunk));
}

function isMainModule() {
  return process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}

if (isMainModule()) {
  if (process.argv.includes("--help") || process.argv.includes("-h")) {
    console.log("usage: sley-lsp --stdio");
  } else if (process.argv.length > 2 && !process.argv.includes("--stdio")) {
    console.error("usage: sley-lsp --stdio");
    process.exitCode = 2;
  } else {
    startServer();
  }
}
