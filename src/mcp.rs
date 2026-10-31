use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::compressors::{self, ContentType};
use crate::tokens::TokenStats;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

pub fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) else {
            continue;
        };

        let response = handle_request(&req);
        if let Some(resp) = response {
            let resp_str = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", resp_str)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

fn handle_request(req: &JsonRpcRequest) -> Option<Value> {
    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "token-reducer",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }
        })),

        "notifications/initialized" => None,

        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "reduce_text",
                        "description": "Summarize logs, diffs, or JSON and report tokenizer counts",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": { "type": "string", "description": "Raw text or logs to compress" },
                                "type": { "type": "string", "enum": ["auto", "log", "diff", "json", "ansi"], "default": "auto" }
                            },
                            "required": ["text"]
                        }
                    },
                    {
                        "name": "reduce_exec",
                        "description": "Execute a shell command safely and return compressed output saving tokens",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string", "description": "Shell command to run" },
                                "timeout_seconds": { "type": "integer", "description": "Maximum execution time in seconds (default: 30)" },
                                "max_output_bytes": { "type": "integer", "description": "Maximum output bytes to capture (default: 5242880 = 5MiB)" }
                            },
                            "required": ["command"]
                        }
                    }
                ]
            }
        })),

        "tools/call" => {
            let params = req.params.as_ref()?;
            let tool_name = params.get("name").and_then(|v| v.as_str())?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            match tool_name {
                "reduce_text" => {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let type_str = args.get("type").and_then(|v| v.as_str()).unwrap_or("auto");
                    let content_type = match type_str {
                        "log" => ContentType::Log,
                        "diff" => ContentType::Diff,
                        "json" => ContentType::Json,
                        "ansi" => ContentType::AnsiOnly,
                        _ => ContentType::Auto,
                    };

                    let compressed = compressors::compress(text, content_type);
                    let stats = TokenStats::compute(text, &compressed);

                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("{}\n\n<!-- token-reducer: {} -->", compressed, stats.format_summary())
                                }
                            ]
                        }
                    }))
                }

                "reduce_exec" => {
                    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout = args
                        .get("timeout_seconds")
                        .and_then(|v| v.as_u64())
                        .map(std::time::Duration::from_secs)
                        .unwrap_or(crate::executor::DEFAULT_TIMEOUT);
                    let max_output_bytes = args
                        .get("max_output_bytes")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize)
                        .unwrap_or(crate::executor::DEFAULT_MAX_OUTPUT_BYTES);

                    let exec_config = crate::executor::ExecConfig {
                        timeout,
                        max_output_bytes,
                    };

                    let exec_result = crate::executor::execute_bounded(command, &exec_config);
                    let compressed = compressors::compress(&exec_result.combined, ContentType::Auto);
                    let stats = TokenStats::compute(&exec_result.combined, &compressed);

                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "isError": exec_result.failed,
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("{}\n\n<!-- token-reducer: {} -->", compressed, stats.format_summary())
                                }
                            ]
                        }
                    }))
                }

                _ => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Tool '{}' not found", tool_name)
                    }
                })),
            }
        }

        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method '{}' not found", req.method)
            }
        })),
    }
}
