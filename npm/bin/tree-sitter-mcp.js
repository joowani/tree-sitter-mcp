#!/usr/bin/env node
// Runs the tree-sitter-mcp binary from the platform package that npm installed as an optional dependency.

"use strict";

const { spawn } = require("node:child_process");
const path = require("node:path");
const { optionalDependencies } = require("../package.json");

const platform = `${process.platform}-${process.arch}`;
const packageName = `@joowani/tree-sitter-mcp-${platform}`;
if (!(packageName in optionalDependencies)) {
  fail(`no prebuilt binary for ${platform}; install it with \`cargo install --locked tree-sitter-mcp\` instead`);
}

let binary;
try {
  const packageDir = path.dirname(require.resolve(`${packageName}/package.json`));
  binary = path.join(packageDir, "bin", process.platform === "win32" ? "tree-sitter-mcp.exe" : "tree-sitter-mcp");
} catch {
  fail(`${packageName} is not installed; reinstall tree-sitter-mcp without --no-optional or --omit=optional`);
}

// The server speaks MCP over stdio, so the child shares this process's stdin, stdout, and stderr.
const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
const forwardedSignals = ["SIGINT", "SIGTERM", "SIGHUP"];
for (const signal of forwardedSignals) {
  process.on(signal, () => child.kill(signal));
}
child.on("error", (error) => fail(`could not start ${binary}: ${error.message}`));
child.on("exit", (code, signal) => {
  if (signal) {
    // Exit by the same signal, so the caller sees how the server stopped.
    for (const forwarded of forwardedSignals) {
      process.removeAllListeners(forwarded);
    }
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 1);
  }
});

function fail(message) {
  // Write to stderr only, since stdout carries the MCP session.
  console.error(`tree-sitter-mcp: ${message}`);
  process.exit(1);
}
