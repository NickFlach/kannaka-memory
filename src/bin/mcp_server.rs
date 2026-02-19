//! Kannaka Memory MCP Server
//! 
//! Model Context Protocol server for Kannaka Memory system.
//! Provides tools for memory storage, retrieval, and introspection.

use std::env;
use std::path::PathBuf;
use std::process;

use serde_json::json;
use kannaka_memory::openclaw::KannakaMemorySystem;
use kannaka_memory::mcp::{
    StdioTransport,
    protocol::{JsonRpcRequest, JsonRpcResponse, InitializeParams, InitializeResult, 
               ServerInfo, ServerCapabilities, ToolsCapability, ToolsListResult,
               ToolCallParams, INVALID_REQUEST, METHOD_NOT_FOUND, 
               INVALID_PARAMS, TOOL_NOT_FOUND},
    tools::McpToolSet,
};

struct McpServer {
    tools: McpToolSet,
    initialized: bool,
}

impl McpServer {
    fn new(tools: McpToolSet) -> Self {
        Self {
            tools,
            initialized: false,
        }
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "initialized" => self.handle_initialized(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            _ => JsonRpcResponse::error(
                request.id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
                None,
            ),
        }
    }

    async fn handle_initialize(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let _params: InitializeParams = match request.params {
            Some(params) => match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => return JsonRpcResponse::error(
                    request.id,
                    INVALID_PARAMS,
                    format!("Invalid initialize params: {}", e),
                    None,
                ),
            },
            None => return JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Missing initialize params".to_string(),
                None,
            ),
        };

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
            },
            server_info: ServerInfo {
                name: "kannaka-memory".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
    }

    async fn handle_initialized(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse::success(request.id, json!(null))
    }

    async fn handle_tools_list(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        if !self.initialized {
            return JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Server not initialized".to_string(),
                None,
            );
        }

        let result = ToolsListResult {
            tools: McpToolSet::get_tool_definitions(),
        };

        JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
    }

    async fn handle_tools_call(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        if !self.initialized {
            return JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Server not initialized".to_string(),
                None,
            );
        }

        let params: ToolCallParams = match request.params {
            Some(params) => match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => return JsonRpcResponse::error(
                    request.id,
                    INVALID_PARAMS,
                    format!("Invalid tool call params: {}", e),
                    None,
                ),
            },
            None => return JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Missing tool call params".to_string(),
                None,
            ),
        };

        // Check if tool exists
        let tool_names: Vec<String> = McpToolSet::get_tool_definitions()
            .into_iter()
            .map(|t| t.name)
            .collect();

        if !tool_names.contains(&params.name) {
            return JsonRpcResponse::error(
                request.id,
                TOOL_NOT_FOUND,
                format!("Tool not found: {}", params.name),
                None,
            );
        }

        // Execute tool
        let result = self.tools.handle_tool_call(params);
        JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap())
    }
}

fn get_config() -> (PathBuf, String, String) {
    let data_dir = env::var("KANNAKA_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_or_default()
        });

    let ollama_url = env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    let ollama_model = env::var("OLLAMA_MODEL")
        .unwrap_or_else(|_| "all-minilm".to_string());

    (data_dir, ollama_url, ollama_model)
}

fn dirs_or_default() -> PathBuf {
    // Use current directory / .kannaka as fallback
    PathBuf::from(".kannaka")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse configuration from environment
    let (data_dir, ollama_url, ollama_model) = get_config();

    eprintln!("Starting Kannaka Memory MCP Server");
    eprintln!("Data directory: {:?}", data_dir);
    eprintln!("Ollama URL: {}", ollama_url);
    eprintln!("Ollama Model: {}", ollama_model);

    // Initialize memory system
    let memory_system = match KannakaMemorySystem::init(data_dir) {
        Ok(sys) => sys,
        Err(e) => {
            eprintln!("Failed to initialize memory system: {}", e);
            process::exit(1);
        }
    };

    // Create MCP tools
    let tools = McpToolSet::new(memory_system, ollama_url, ollama_model);
    let server = McpServer::new(tools);

    eprintln!("Server initialized, listening on stdio...");

    // Run the server
    let server = std::sync::Arc::new(tokio::sync::Mutex::new(server));
    
    StdioTransport::run(move |request| {
        let server = server.clone();
        async move {
            let mut server_guard = server.lock().await;
            server_guard.handle_request(request).await
        }
    }).await?;

    eprintln!("Server shutdown");
    Ok(())
}