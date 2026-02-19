//! MCP (Model Context Protocol) server implementation for Kannaka Memory

#[cfg(feature = "mcp")]
pub mod bm25;
#[cfg(feature = "mcp")]
pub mod embeddings;
#[cfg(feature = "mcp")]
pub mod protocol;
#[cfg(feature = "mcp")]
pub mod retrieval;
#[cfg(feature = "mcp")]
pub mod tools;
#[cfg(feature = "mcp")]
pub mod transport;

#[cfg(feature = "mcp")]
pub use transport::StdioTransport;
#[cfg(feature = "mcp")]
pub use protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcError, ServerCapabilities, ToolDefinition};
#[cfg(feature = "mcp")]
pub use tools::McpToolSet;