use std::io::{self, BufRead, Write};
use std::process::Command;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
                    "version": "0.1.0"
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
                        "description": "Compress noisy logs, compiler errors, or JSON to save 60-90% LLM tokens",
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
                        "description": "Execute a shell command and return compressed output saving tokens",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string", "description": "Shell command to run" }
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
                    let output = if cfg!(target_os = "windows") {
                        Command::new("cmd").args(["/C", command]).output()
                    } else {
                        Command::new("sh").args(["-c", command]).output()
                    };

                    let (stdout_str, stderr_str) = match output {
                        Ok(out) => (
                            String::from_utf8_lossy(&out.stdout).to_string(),
                            String::from_utf8_lossy(&out.stderr).to_string(),
                        ),
                        Err(e) => (String::new(), format!("Execution error: {}", e)),
                    };

                    let combined = if stderr_str.is_empty() {
                        stdout_str
                    } else {
                        format!("{}\n{}", stdout_str, stderr_str)
                    };

                    let compressed = compressors::compress(&combined, ContentType::Auto);
                    let stats = TokenStats::compute(&combined, &compressed);

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
