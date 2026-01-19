use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use notify_rust::Notification;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use cli_clipboard::{ClipboardContext, ClipboardProvider};

pub struct InputModule;

impl InputModule {
    pub fn new() -> Self {
        Self
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "input_notify",
                "description": "Send notifications (terminal and desktop)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Notification title"
                        },
                        "message": {
                            "type": "string",
                            "description": "Notification message"
                        },
                        "type": {
                            "type": "string",
                            "enum": ["terminal", "desktop", "both"],
                            "description": "Notification type (default: both)"
                        },
                        "urgency": {
                            "type": "string",
                            "enum": ["low", "normal", "critical"],
                            "description": "Desktop notification urgency (default: normal)"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Notification timeout in milliseconds (desktop only)"
                        }
                    },
                    "required": ["message"]
                }
            }),
            json!({
                "name": "input_prompt",
                "description": "Interactive user prompts (supports MCP and terminal)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "Prompt message"
                        },
                        "default": {
                            "type": "string",
                            "description": "Default value"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["terminal", "mcp"],
                            "description": "Input mode (default: terminal)"
                        }
                    },
                    "required": ["prompt"]
                }
            }),
            json!({
                "name": "input_select",
                "description": "Selection menus (supports MCP and terminal)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "Prompt message"
                        },
                        "options": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "List of options to choose from"
                        },
                        "default": {
                            "type": "number",
                            "description": "Default option index"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["terminal", "mcp"],
                            "description": "Input mode (default: terminal)"
                        }
                    },
                    "required": ["prompt", "options"]
                }
            }),
            json!({
                "name": "input_progress",
                "description": "Display progress indicators",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["start", "update", "finish"],
                            "description": "Progress action"
                        },
                        "id": {
                            "type": "string",
                            "description": "Progress bar identifier"
                        },
                        "total": {
                            "type": "number",
                            "description": "Total steps (for start)"
                        },
                        "current": {
                            "type": "number",
                            "description": "Current step (for update)"
                        },
                        "message": {
                            "type": "string",
                            "description": "Progress message"
                        }
                    },
                    "required": ["action"]
                }
            }),
            json!({
                "name": "input_clipboard_read",
                "description": "Read from clipboard",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            json!({
                "name": "input_clipboard_write",
                "description": "Write to clipboard",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to write to clipboard"
                        }
                    },
                    "required": ["content"]
                }
            }),
        ]
    }

    pub async fn notify(&self, args: Value) -> Result<Value> {
        let title = args["title"].as_str().unwrap_or("Poly MCP");
        let message = args["message"].as_str().context("Missing 'message' parameter")?;
        let notification_type = args["type"].as_str().unwrap_or("both");
        let urgency = args["urgency"].as_str().unwrap_or("normal");
        let timeout = args["timeout"].as_u64().map(|t| t as i32);

        let mut results = json!({
            "title": title,
            "message": message
        });

        // Terminal notification
        if notification_type == "terminal" || notification_type == "both" {
            println!("\n┌─ {} ─┐", "─".repeat(title.len().max(message.len())));
            println!("│ {} │", title);
            println!("├─{}─┤", "─".repeat(title.len().max(message.len())));
            println!("│ {} │", message);
            println!("└─{}─┘\n", "─".repeat(title.len().max(message.len())));

            results["terminal"] = json!(true);
        }

        // Desktop notification
        if notification_type == "desktop" || notification_type == "both" {
            let mut notification = Notification::new();
            notification.summary(title);
            notification.body(message);

            match urgency {
                "low" => { notification.urgency(notify_rust::Urgency::Low); }
                "critical" => { notification.urgency(notify_rust::Urgency::Critical); }
                _ => { notification.urgency(notify_rust::Urgency::Normal); }
            }

            if let Some(t) = timeout {
                notification.timeout(t);
            }

            match notification.show() {
                Ok(_) => {
                    results["desktop"] = json!(true);
                }
                Err(e) => {
                    results["desktop"] = json!(false);
                    results["desktop_error"] = json!(e.to_string());
                }
            }
        }

        Ok(results)
    }

    pub async fn prompt_user(&self, args: Value) -> Result<Value> {
        let prompt = args["prompt"].as_str().context("Missing 'prompt' parameter")?;
        let default_value = args["default"].as_str();
        let mode = args["mode"].as_str().unwrap_or("terminal");

        match mode {
            "terminal" => {
                let input = Input::<String>::new()
                    .with_prompt(prompt);

                let input = if let Some(default) = default_value {
                    input.default(default.to_string())
                } else {
                    input
                };

                let result = input.interact_text()?;

                Ok(json!({
                    "prompt": prompt,
                    "response": result,
                    "mode": "terminal"
                }))
            }
            "mcp" => {
                // For MCP mode, we would need to use MCP sampling
                // For now, return a placeholder indicating MCP prompting is needed
                Ok(json!({
                    "prompt": prompt,
                    "mode": "mcp",
                    "message": "MCP sampling would be triggered here",
                    "default": default_value
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown mode: {}", mode)),
        }
    }

    pub async fn select(&self, args: Value) -> Result<Value> {
        let prompt = args["prompt"].as_str().context("Missing 'prompt' parameter")?;
        let options = args["options"].as_array().context("Missing 'options' parameter")?;
        let default_idx = args["default"].as_u64().map(|i| i as usize);
        let mode = args["mode"].as_str().unwrap_or("terminal");

        let option_strs: Vec<String> = options
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if option_strs.is_empty() {
            return Err(anyhow::anyhow!("No valid options provided"));
        }

        match mode {
            "terminal" => {
                let select = Select::new()
                    .with_prompt(prompt)
                    .items(&option_strs);

                let select = if let Some(idx) = default_idx {
                    select.default(idx)
                } else {
                    select
                };

                let selection_idx = select.interact()?;
                let selected = &option_strs[selection_idx];

                Ok(json!({
                    "prompt": prompt,
                    "selected": selected,
                    "index": selection_idx,
                    "mode": "terminal"
                }))
            }
            "mcp" => {
                // For MCP mode, we would need to use MCP sampling
                Ok(json!({
                    "prompt": prompt,
                    "options": option_strs,
                    "mode": "mcp",
                    "message": "MCP sampling would be triggered here",
                    "default": default_idx
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown mode: {}", mode)),
        }
    }

    pub async fn progress(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().context("Missing 'action' parameter")?;
        let id = args["id"].as_str().unwrap_or("default");

        match action {
            "start" => {
                let total = args["total"].as_u64().context("Missing 'total' parameter for start action")?;
                let message = args["message"].as_str().unwrap_or("Processing...");

                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"),
                );
                pb.set_message(message.to_string());

                // Note: In a real implementation, we'd store this progress bar
                // for later updates. For now, we'll just create and finish it.
                pb.finish_with_message("Started");

                Ok(json!({
                    "action": "start",
                    "id": id,
                    "total": total,
                    "message": message
                }))
            }
            "update" => {
                let current = args["current"].as_u64().context("Missing 'current' parameter for update action")?;
                let message = args["message"].as_str();

                // In a real implementation, we'd retrieve and update the stored progress bar
                Ok(json!({
                    "action": "update",
                    "id": id,
                    "current": current,
                    "message": message
                }))
            }
            "finish" => {
                let message = args["message"].as_str().unwrap_or("Done!");

                // In a real implementation, we'd finish the stored progress bar
                Ok(json!({
                    "action": "finish",
                    "id": id,
                    "message": message
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    pub async fn clipboard_read(&self, _args: Value) -> Result<Value> {
        let mut ctx = ClipboardContext::new()
            .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;

        let content = ctx.get_contents()
            .map_err(|e| anyhow::anyhow!("Failed to read clipboard: {}", e))?;

        Ok(json!({
            "content": content,
            "length": content.len()
        }))
    }

    pub async fn clipboard_write(&self, args: Value) -> Result<Value> {
        let content = args["content"].as_str().context("Missing 'content' parameter")?;

        let mut ctx = ClipboardContext::new()
            .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;

        ctx.set_contents(content.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to write to clipboard: {}", e))?;

        Ok(json!({
            "success": true,
            "content_length": content.len()
        }))
    }
}
