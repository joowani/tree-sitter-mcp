# Tree-sitter MCP Server

An [MCP](https://modelcontextprotocol.io) server that shows coding agents the
[Tree-sitter](https://tree-sitter.github.io) syntax tree of a code snippet, so they can check node kinds, field names,
and positions before writing Tree-sitter queries or code that walks a syntax tree.

## Installation

Claude Code:

```shell
claude mcp add tree-sitter -- npx -y tree-sitter-mcp-server
```

Codex:

```shell
codex mcp add tree-sitter -- npx -y tree-sitter-mcp-server
```

Other clients:

```json
{
  "mcpServers": {
    "tree-sitter": {
      "command": "npx",
      "args": ["-y", "tree-sitter-mcp-server"]
    }
  }
}
```

`npx` needs Node.js. Without it, install the binary with `cargo install --locked tree-sitter-mcp-server` or download
it from the [releases](https://github.com/joowani/tree-sitter-mcp-server/releases), and use `tree-sitter-mcp-server` as
the command.

Clients that browse the [MCP Registry](https://registry.modelcontextprotocol.io), such as VS Code, can also install it
from there.

mcp-name: io.github.joowani/tree-sitter-mcp-server

## Supported Languages

`c`, `cpp`, `csharp`, `go`, `graphql`, `hcl`, `java`, `javascript`, `json`, `jsx`, `kotlin`, `php`, `prisma`,
`protobuf`, `python`, `ruby`, `rust`, `sql`, `swift`, `terraform`, `thrift`, `toml`, `tsx`, `typescript`, `yaml`

Names are case-insensitive, and common aliases such as `c++`, `c#`, `py`, and `ts` also work. `javascript` and `jsx`
use the TypeScript and TSX grammars, and `terraform` uses the HCL grammar. PHP code must start with `<?php`.

## Tool

`get_tree_sitter_ast` returns the syntax tree of `code` in `language`, one named node per line, with field names and
zero-based `(row-column)` ranges. `ERROR` and `MISSING` nodes mark syntax errors.

```json
{
  "language": "python",
  "code": "x = 1"
}
```

```text
module (0-0) - (0-5)
  expression_statement (0-0) - (0-5)
    assignment (0-0) - (0-5)
      left: identifier (0-0) - (0-1)
      right: integer (0-4) - (0-5)
```
