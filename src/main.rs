//! MCP server, over stdio, that parses code snippets into Tree-sitter ASTs.

use anyhow::Context;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::transport::stdio;
use rmcp::{ErrorData, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::Deserialize;
use tree_sitter_mcp_server::{SUPPORTED_LANGUAGES, format_ast, parse};

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct GetTreeSitterAstArgs {
    /// Language of the code, such as `python`, `tsx`, or an alias like `py`, `ts`, or `c#`; case-insensitive.
    language: String,
    /// Code to parse. It can be a fragment rather than a whole file.
    code: String,
}

fn tool_description() -> String {
    format!(
        "Parse code with Tree-sitter and return its syntax tree, one named node per line, indented two spaces per \
         level: `field: kind (startRow-startColumn) - (endRow-endColumn)`. Rows and columns are zero-based, and \
         columns count bytes. `ERROR` marks code that could not be parsed, and `MISSING kind` marks a token the \
         parser inserted to recover. Supported languages: {}.",
        SUPPORTED_LANGUAGES.join(", ")
    )
}

#[derive(Clone)]
struct TreeSitterServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl TreeSitterServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        title = "Get Tree-sitter AST",
        description = tool_description(),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_tree_sitter_ast(
        &self,
        Parameters(args): Parameters<GetTreeSitterAstArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let parsed = tokio::task::spawn_blocking(move || {
            parse(&args.language, &args.code).and_then(|tree| format_ast(tree.root_node()))
        })
        .await
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        Ok(match parsed {
            Ok(ast) => CallToolResult::success(vec![ContentBlock::text(ast)]),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.to_string())]),
        })
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TreeSitterServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
                    .with_title("Tree-sitter MCP Server")
                    .with_description(env!("CARGO_PKG_DESCRIPTION"))
                    .with_website_url(env!("CARGO_PKG_REPOSITORY")),
            )
            .with_instructions(
                "Use get_tree_sitter_ast to see the Tree-sitter node kinds, field names, and positions of a code \
                 snippet before writing Tree-sitter queries or code that walks a syntax tree.",
            )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let service = TreeSitterServer::new()
        .serve(stdio())
        .await
        .context("could not start the MCP session")?;
    service.waiting().await?;
    Ok(())
}
