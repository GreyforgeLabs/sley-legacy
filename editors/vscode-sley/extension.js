"use strict";

const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

async function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("sley.restartLanguageServer", async () => {
      await stopClient();
      await startClient(context);
    }),
  );
  await startClient(context);
}

function resolveServerCommand() {
  const configured = vscode.workspace
    .getConfiguration("sley")
    .get("languageServer.path", "");
  if (configured.trim()) {
    return configured.trim();
  }
  if (process.env.SLEY_LSP && process.env.SLEY_LSP.trim()) {
    return process.env.SLEY_LSP.trim();
  }
  return "sley-lsp";
}

function workspaceCwd() {
  const folders = vscode.workspace.workspaceFolders;
  if (folders && folders.length > 0) {
    return folders[0].uri.fsPath;
  }
  return process.cwd();
}

async function startClient(context) {
  const command = resolveServerCommand();
  const serverOptions = {
    command,
    args: [],
    transport: TransportKind.stdio,
    options: {
      cwd: workspaceCwd(),
    },
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "sley" }],
    outputChannelName: "Sley Language Server",
    synchronize: {
      fileEvents: [
        vscode.workspace.createFileSystemWatcher("**/*.sley"),
        vscode.workspace.createFileSystemWatcher("**/sley.toml"),
      ],
    },
  };
  client = new LanguageClient(
    "sleyLanguageServer",
    "Sley Language Server",
    serverOptions,
    clientOptions,
  );
  context.subscriptions.push(client);
  try {
    await client.start();
  } catch (error) {
    vscode.window.showErrorMessage(
      `Sley language server failed to start from ${command}: ${error.message}`,
    );
    throw error;
  }
}

async function stopClient() {
  if (!client) {
    return;
  }
  const current = client;
  client = undefined;
  await current.stop();
}

function deactivate() {
  return stopClient();
}

module.exports = {
  activate,
  deactivate,
};
