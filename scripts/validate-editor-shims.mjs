import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const shimRoot = process.argv[2] ?? "editors/vscode-sley";
const extensionRoot = path.join(repoRoot, shimRoot);

function readJson(relativePath) {
  const fullPath = path.join(repoRoot, relativePath);
  return JSON.parse(fs.readFileSync(fullPath, "utf8"));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertFile(relativePath) {
  const fullPath = path.join(repoRoot, relativePath);
  assert(fs.existsSync(fullPath), `${relativePath} is missing`);
  return fullPath;
}

const manifest = readJson(path.join(shimRoot, "package.json"));
assert(manifest.name === "sley-vscode", "VS Code package name should be sley-vscode");
assert(manifest.private === true, "VS Code shim must stay private");
assert(manifest.main === "./extension.js", "VS Code shim main should be ./extension.js");
assert(
  manifest.dependencies?.["vscode-languageclient"],
  "VS Code shim should depend on vscode-languageclient",
);
assert(
  manifest.activationEvents.includes("onLanguage:sley"),
  "VS Code shim should activate on Sley documents",
);
assert(
  manifest.activationEvents.includes("workspaceContains:sley.toml"),
  "VS Code shim should activate in Sley project workspaces",
);

const language = manifest.contributes.languages.find((entry) => entry.id === "sley");
assert(language, "VS Code shim should contribute the sley language");
assert(language.extensions.includes(".sley"), "VS Code shim should claim .sley files");
assertFile(path.join(shimRoot, language.configuration));

const grammar = manifest.contributes.grammars.find((entry) => entry.language === "sley");
assert(grammar, "VS Code shim should contribute a Sley grammar");
assert(grammar.scopeName === "source.sley", "Sley grammar scope should be source.sley");
assertFile(path.join(shimRoot, grammar.path));

const configuration = readJson(path.join(shimRoot, "language-configuration.json"));
assert(configuration.comments.lineComment === "//", "Sley line comment should be //");
assert(
  configuration.brackets.some(([open, close]) => open === "{" && close === "}"),
  "Sley language configuration should include braces",
);

const tmLanguage = readJson(path.join(shimRoot, "syntaxes/sley.tmLanguage.json"));
assert(tmLanguage.scopeName === "source.sley", "TextMate grammar scope should be source.sley");
assert(tmLanguage.repository.keywords, "TextMate grammar should include keywords");
assert(tmLanguage.repository.hostCalls, "TextMate grammar should include host calls");

const extensionSource = fs.readFileSync(path.join(extensionRoot, "extension.js"), "utf8");
assert(
  extensionSource.includes('require("vscode-languageclient/node")'),
  "Extension should use vscode-languageclient/node",
);
assert(extensionSource.includes("sley-lsp"), "Extension should default to sley-lsp");
assert(
  extensionSource.includes('getConfiguration("sley")') &&
    extensionSource.includes('get("languageServer.path"'),
  "Extension should honor the sley.languageServer.path setting",
);
assert(extensionSource.includes("SLEY_LSP"), "Extension should honor SLEY_LSP");
assert(
  extensionSource.includes("sley.restartLanguageServer"),
  "Extension should expose restart command",
);

console.log("editor shims validated");
