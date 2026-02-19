//! Stdio transport for MCP JSON-RPC communication

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};

pub struct StdioTransport;

impl StdioTransport {
    pub async fn run<F, Fut>(handler: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(JsonRpcRequest) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = JsonRpcResponse> + Send,
    {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = TokioBufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Parse JSON-RPC request
                    let request: JsonRpcRequest = match serde_json::from_str(line) {
                        Ok(req) => req,
                        Err(e) => {
                            let error_response = JsonRpcResponse::error(
                                serde_json::Value::Null,
                                super::protocol::PARSE_ERROR,
                                format!("Parse error: {}", e),
                                None,
                            );
                            let response_json = serde_json::to_string(&error_response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                            continue;
                        }
                    };

                    // Notifications have no id (or null id) — don't send response
                    let is_notification = request.id.is_null();

                    // Handle request
                    let response = handler(request).await;

                    // Only send response for requests (not notifications)
                    if !is_notification {
                        let response_json = serde_json::to_string(&response)?;
                        stdout.write_all(response_json.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}