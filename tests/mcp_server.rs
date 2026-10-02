//! Drives the `tree-sitter-mcp-server` binary with the official MCP client over both protocol lifecycles.

use rmcp::RoleClient;
use rmcp::model::{CallToolRequestParams, CallToolResult, ProtocolVersion};
use rmcp::service::{ClientLifecycleMode, ClientServiceExt, RunningService};
use rmcp::transport::TokioChildProcess;
use serde_json::json;
use tokio::process::Command;
use tree_sitter_mcp_server::{MAX_CODE_BYTES, SUPPORTED_LANGUAGES};

async fn connect(lifecycle: ClientLifecycleMode) -> RunningService<RoleClient, ()> {
    let server = Command::new(env!("CARGO_BIN_EXE_tree-sitter-mcp-server"));
    let transport = TokioChildProcess::new(server).expect("server should start");
    ().serve_with_lifecycle(transport, lifecycle)
        .await
        .expect("client should connect")
}

async fn get_ast(
    client: &RunningService<RoleClient, ()>,
    language: &str,
    code: &str,
) -> CallToolResult {
    let arguments = json!({ "language": language, "code": code });
    let params = CallToolRequestParams::new("get_tree_sitter_ast").with_arguments(
        arguments
            .as_object()
            .expect("arguments are an object")
            .clone(),
    );
    client
        .call_tool(params)
        .await
        .expect("tool call should succeed")
}

fn text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|content| content.as_text())
        .map(|content| content.text.as_str())
        .collect()
}

async fn assert_serves_asts(client: &RunningService<RoleClient, ()>) {
    let result = get_ast(client, "Python", "x = 1\n").await;
    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        text(&result),
        "module (0-0) - (1-0)\n  expression_statement (0-0) - (0-5)\n    assignment (0-0) - (0-5)\n      \
         left: identifier (0-0) - (0-1)\n      right: integer (0-4) - (0-5)"
    );

    let result = get_ast(client, "c#", "class A {}").await;
    assert!(
        text(&result).starts_with("compilation_unit (0-0)"),
        "{}",
        text(&result)
    );

    let result = get_ast(client, "cobol", "DISPLAY 'HI'.").await;
    assert_eq!(result.is_error, Some(true));
    assert!(
        text(&result).contains("unsupported language `cobol`"),
        "{}",
        text(&result)
    );
}

#[tokio::test]
async fn test_legacy_initialize_lifecycle_serves_asts() {
    let client = connect(ClientLifecycleMode::Initialize).await;
    let server = client.peer_info().expect("server info after initialize");
    assert_eq!(
        server.protocol_version,
        ProtocolVersion::LATEST_WITH_INITIALIZE
    );
    let implementation = server.server_info.as_ref().expect("server implementation");
    assert_eq!(implementation.name, "tree-sitter-mcp-server");
    assert_eq!(implementation.version, env!("CARGO_PKG_VERSION"));

    assert_serves_asts(&client).await;
    client.cancel().await.expect("client should shut down");
}

#[tokio::test]
async fn test_reports_asts_larger_than_the_limit_as_tool_errors() {
    let client = connect(ClientLifecycleMode::Initialize).await;
    // Each nested array is indented one level deeper, so the AST would be about 100 MB.
    let code = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
    let result = get_ast(&client, "json", &code).await;
    assert_eq!(result.is_error, Some(true));
    assert!(
        text(&result).contains("the AST is larger than"),
        "{}",
        text(&result)
    );
    client.cancel().await.expect("client should shut down");
}

#[tokio::test]
async fn test_reports_code_larger_than_the_limit_as_tool_errors() {
    let client = connect(ClientLifecycleMode::Initialize).await;
    let code = "x".repeat(MAX_CODE_BYTES + 1);
    let result = get_ast(&client, "python", &code).await;
    assert_eq!(result.is_error, Some(true));
    assert!(
        text(&result).contains("the code is larger than 4 MiB"),
        "{}",
        text(&result)
    );
    // The server keeps serving after rejecting the code.
    let result = get_ast(&client, "python", "x = 1\n").await;
    assert_eq!(result.is_error, Some(false));
    client.cancel().await.expect("client should shut down");
}

#[tokio::test]
async fn test_stateless_2026_07_28_lifecycle_serves_asts() {
    let lifecycle = ClientLifecycleMode::Discover {
        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
    };
    let client = connect(lifecycle).await;
    let server = client.peer_info().expect("server info after discovery");
    assert_eq!(server.protocol_version, ProtocolVersion::V_2026_07_28);
    let implementation = server.server_info.as_ref().expect("server implementation");
    assert_eq!(implementation.name, "tree-sitter-mcp-server");

    let tools = client.list_all_tools().await.expect("tools should list");
    assert_eq!(tools.len(), 1);
    let tool = &tools[0];
    assert_eq!(tool.name, "get_tree_sitter_ast");
    let annotations = tool.annotations.as_ref().expect("tool annotations");
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.open_world_hint, Some(false));
    let description = tool.description.as_deref().unwrap_or_default();
    for language in SUPPORTED_LANGUAGES {
        assert!(
            description.contains(language),
            "description is missing {language}"
        );
    }

    assert_serves_asts(&client).await;
    client.cancel().await.expect("client should shut down");
}
