import { mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  LspMessageFramer,
  frameMessage,
  handleJsonRpc,
  initializeResponse,
  normalizeUri,
  positionFromUtf8Offset,
  runSley,
} from "../src/server.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const serverPath = join(repoRoot, "src/server.mjs");

const response = initializeResponse(1);
if (!response.result.capabilities.documentFormattingProvider) {
  throw new Error("missing formatting capability");
}
if (response.result.capabilities.codeActionProvider) {
  throw new Error("server advertises an unsupported code-action method");
}

const unicodeMessages = [
  { jsonrpc: "2.0", id: 1, method: "example/emoji", params: { text: "A😀中e\u0301" } },
  { jsonrpc: "2.0", id: 2, method: "example/astral", params: { text: "𐐷" } },
];
const combined = Buffer.concat(unicodeMessages.map(frameMessage));
const bytewise = [];
const bytewiseErrors = [];
const bytewiseFramer = new LspMessageFramer({
  onMessage: (message) => bytewise.push(message),
  onError: (error) => bytewiseErrors.push(error),
});
for (const byte of combined) bytewiseFramer.push(Buffer.from([byte]));
if (bytewiseErrors.length || JSON.stringify(bytewise) !== JSON.stringify(unicodeMessages)) {
  throw new Error("bytewise Unicode framing failed");
}

const chunked = [];
const chunkedFramer = new LspMessageFramer({
  onMessage: (message) => chunked.push(message),
  onError: (error) => {
    throw new Error(error.error.message);
  },
});
let offset = 0;
let chunkSize = 1;
while (offset < combined.length) {
  chunkedFramer.push(combined.subarray(offset, offset + chunkSize));
  offset += chunkSize;
  chunkSize = (chunkSize * 7) % 23 + 1;
}
if (JSON.stringify(chunked) !== JSON.stringify(unicodeMessages)) {
  throw new Error("randomized chunk framing failed");
}

for (const invalidHeader of [
  "X-Test: 1\r\n\r\n",
  "Content-Length: -1\r\n\r\n",
  "Content-Length: nope\r\n\r\n",
  "Content-Length: 1\r\nContent-Length: 1\r\n\r\n{}",
]) {
  const errors = [];
  const framer = new LspMessageFramer({
    onMessage: () => {},
    onError: (error) => errors.push(error),
    maxBodyBytes: 32,
  });
  framer.push(Buffer.from(invalidHeader, "utf8"));
  if (!framer.fatal || errors.length !== 1) throw new Error("invalid Content-Length did not fail closed");
}

const oversizeErrors = [];
const oversize = new LspMessageFramer({
  onMessage: () => {},
  onError: (error) => oversizeErrors.push(error),
  maxBodyBytes: 4,
});
oversize.push(Buffer.from("Content-Length: 5\r\n\r\n", "ascii"));
if (!oversize.fatal || oversizeErrors.length !== 1) throw new Error("oversized frame passed");

const recovered = [];
const recoverErrors = [];
const recover = new LspMessageFramer({
  onMessage: (message) => recovered.push(message),
  onError: (error) => recoverErrors.push(error),
});
recover.push(Buffer.concat([
  Buffer.from("Content-Length: 1\r\n\r\n{", "ascii"),
  frameMessage(unicodeMessages[0]),
]));
if (recoverErrors.length !== 1 || recovered.length !== 1 || recovered[0].id !== 1) {
  throw new Error("known-length malformed JSON did not recover deterministically");
}

const text = "a😀中\nb";
const firstLineBytes = Buffer.byteLength("a😀中", "utf8");
const position = positionFromUtf8Offset(text, firstLineBytes);
if (position.line !== 0 || position.character !== 4) {
  throw new Error("UTF-16 position mismatch: " + JSON.stringify(position));
}

const workspace = mkdtempSync(join(tmpdir(), "sley-lsp-workspace-"));
const outside = mkdtempSync(join(tmpdir(), "sley-lsp-outside-"));
const insideFile = join(workspace, "inside.sley");
const outsideFile = join(outside, "outside.sley");
const escapedLink = join(workspace, "escaped.sley");
writeFileSync(insideFile, "export task Inside {}\n");
writeFileSync(outsideFile, "export task Outside {}\n");
symlinkSync(outsideFile, escapedLink);
const rootUri = pathToFileURL(workspace).href;
const insideUri = pathToFileURL(insideFile).href;
const outsideUri = pathToFileURL(outsideFile).href;
handleJsonRpc({ jsonrpc: "2.0", id: 10, method: "initialize", params: { rootUri } });
const symbols = handleJsonRpc({
  jsonrpc: "2.0",
  id: 11,
  method: "textDocument/documentSymbol",
  params: { textDocument: { uri: insideUri } },
});
if (symbols.result?.[0]?.name !== "Inside") throw new Error("workspace document read failed");
const denied = handleJsonRpc({
  jsonrpc: "2.0",
  id: 12,
  method: "textDocument/documentSymbol",
  params: { textDocument: { uri: outsideUri } },
});
if (denied.error?.code !== -32602) throw new Error("out-of-workspace file read was not denied");
const symlinkDenied = handleJsonRpc({
  jsonrpc: "2.0",
  id: 14,
  method: "textDocument/documentSymbol",
  params: { textDocument: { uri: pathToFileURL(escapedLink).href } },
});
if (symlinkDenied.error?.code !== -32602) throw new Error("workspace symlink escape was not denied");

const unknown = handleJsonRpc({ jsonrpc: "2.0", id: 13, method: "unknown/method" });
if (unknown.error?.code !== -32601) throw new Error("unknown request did not return Method not found");

const closed = handleJsonRpc({
  jsonrpc: "2.0",
  method: "textDocument/didClose",
  params: { textDocument: { uri: insideUri } },
});
if (closed.params?.uri !== insideUri || closed.params?.diagnostics?.length !== 0) {
  throw new Error("didClose did not clear diagnostics");
}

const windowsUri = normalizeUri("file:///C:/Users/Test%20Space/example.sley");
if (!windowsUri.replaceAll("\\", "/").includes("C:/Users/Test Space/example.sley")) {
  throw new Error("Windows-style file URI was decoded incorrectly: " + windowsUri);
}

const timedOut = runSley([], {
  executable: process.execPath,
  prefixArgs: ["-e", "setTimeout(() => {}, 10000)"],
  timeout: 50,
});
if (timedOut.ok || !timedOut.timedOut) throw new Error("hung compiler did not time out");

const flooded = runSley([], {
  executable: process.execPath,
  prefixArgs: ["-e", "process.stdout.write('x'.repeat(100000))"],
  maxBuffer: 1024,
});
if (flooded.ok || !flooded.outputLimited) throw new Error("compiler output flood was not bounded");

const help = spawnSync(process.execPath, [serverPath, "--help"], {
  cwd: repoRoot,
  encoding: "utf8",
  timeout: 1000,
});
if (help.status !== 0 || help.stdout.trim() !== "usage: sley-lsp --stdio") {
  throw new Error("help did not exit cleanly: " + help.status + " " + help.stdout + " " + help.stderr);
}

console.log("sley-lsp smoke ok");
