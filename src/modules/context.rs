use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiktoken_rs::{cl100k_base, o200k_base};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write as _;

pub struct ContextModule {
    memory_store: Arc<Mutex<HashMap<String, Value>>>,
    context_usage: Arc<Mutex<ContextUsage>>,
}

#[derive(Default)]
struct ContextUsage {
    total_tokens: usize,
    used_tokens: usize,
}

impl ContextModule {
    pub fn new() -> Self {
        Self {
            memory_store: Arc::new(Mutex::new(HashMap::new())),
            context_usage: Arc::new(Mutex::new(ContextUsage::default())),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "ctx_context",
                "description": "Get token usage statistics (total, left, used)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "set_total": {
                            "type": "number",
                            "description": "Set the total context window size"
                        },
                        "add_used": {
                            "type": "number",
                            "description": "Add to used token count"
                        }
                    }
                }
            }),
            json!({
                "name": "ctx_compact",
                "description": "Compress text using algorithms to reduce size",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to compress"
                        },
                        "algorithm": {
                            "type": "string",
                            "enum": ["zlib", "gzip"],
                            "description": "Compression algorithm (default: zlib)"
                        }
                    },
                    "required": ["text"]
                }
            }),
            json!({
                "name": "ctx_remove",
                "description": "Clear context and reset usage",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "reset_memory": {
                            "type": "boolean",
                            "description": "Also clear memory store (default: false)"
                        }
                    }
                }
            }),
            json!({
                "name": "ctx_token_count",
                "description": "Count tokens in text for various LLM providers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to count tokens for"
                        },
                        "model": {
                            "type": "string",
                            "enum": ["gpt-4", "gpt-3.5-turbo", "claude-3", "claude-2", "o200k"],
                            "description": "Model to use for tokenization (default: gpt-4)"
                        }
                    },
                    "required": ["text"]
                }
            }),
            json!({
                "name": "ctx_memory_store",
                "description": "Store data in memory (process lifetime)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Key to store data under"
                        },
                        "value": {
                            "description": "Value to store (any JSON type)"
                        }
                    },
                    "required": ["key", "value"]
                }
            }),
            json!({
                "name": "ctx_memory_recall",
                "description": "Retrieve stored data from memory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Key to retrieve (omit to list all keys)"
                        }
                    }
                }
            }),
            json!({
                "name": "ctx_estimate_cost",
                "description": "Estimate API costs for LLM providers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provider": {
                            "type": "string",
                            "enum": ["anthropic", "openai", "ollama", "glm"],
                            "description": "LLM provider"
                        },
                        "model": {
                            "type": "string",
                            "description": "Model name"
                        },
                        "input_tokens": {
                            "type": "number",
                            "description": "Number of input tokens"
                        },
                        "output_tokens": {
                            "type": "number",
                            "description": "Number of output tokens"
                        }
                    },
                    "required": ["provider", "model", "input_tokens", "output_tokens"]
                }
            }),
        ]
    }

    pub async fn context(&self, args: Value) -> Result<Value> {
        let mut usage = self.context_usage.lock().unwrap();

        if let Some(total) = args["set_total"].as_u64() {
            usage.total_tokens = total as usize;
        }

        if let Some(add_used) = args["add_used"].as_u64() {
            usage.used_tokens += add_used as usize;
        }

        let left = usage.total_tokens.saturating_sub(usage.used_tokens);
        let usage_percent = if usage.total_tokens > 0 {
            (usage.used_tokens as f64 / usage.total_tokens as f64) * 100.0
        } else {
            0.0
        };

        Ok(json!({
            "total": usage.total_tokens,
            "used": usage.used_tokens,
            "left": left,
            "usage_percent": usage_percent
        }))
    }

    pub async fn compact_context(&self, args: Value) -> Result<Value> {
        let text = args["text"].as_str().context("Missing 'text' parameter")?;
        let algorithm = args["algorithm"].as_str().unwrap_or("zlib");

        let original_size = text.len();

        let compressed = match algorithm {
            "zlib" | "gzip" => {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
                encoder.write_all(text.as_bytes())?;
                encoder.finish()?
            }
            _ => return Err(anyhow::anyhow!("Unknown compression algorithm: {}", algorithm)),
        };

        let compressed_size = compressed.len();
        let compression_ratio = (compressed_size as f64 / original_size as f64) * 100.0;

        // Encode compressed data as base64 for safe transport
        use base64::{Engine, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(&compressed);

        Ok(json!({
            "original_size": original_size,
            "compressed_size": compressed_size,
            "compression_ratio_percent": compression_ratio,
            "algorithm": algorithm,
            "compressed_data": encoded,
            "savings_bytes": original_size - compressed_size
        }))
    }

    pub async fn remove_context(&self, args: Value) -> Result<Value> {
        let reset_memory = args["reset_memory"].as_bool().unwrap_or(false);

        let mut usage = self.context_usage.lock().unwrap();
        usage.used_tokens = 0;

        let memory_cleared = if reset_memory {
            let mut store = self.memory_store.lock().unwrap();
            let count = store.len();
            store.clear();
            count
        } else {
            0
        };

        Ok(json!({
            "success": true,
            "context_reset": true,
            "memory_cleared": reset_memory,
            "memory_items_cleared": memory_cleared
        }))
    }

    pub async fn token_count(&self, args: Value) -> Result<Value> {
        let text = args["text"].as_str().context("Missing 'text' parameter")?;
        let model = args["model"].as_str().unwrap_or("gpt-4");

        let token_count = match model {
            "gpt-4" | "gpt-3.5-turbo" | "claude-3" | "claude-2" => {
                let bpe = cl100k_base()?;
                bpe.encode_with_special_tokens(text).len()
            }
            "o200k" => {
                let bpe = o200k_base()?;
                bpe.encode_with_special_tokens(text).len()
            }
            _ => {
                // Fallback: simple word-based estimation
                let words = text.split_whitespace().count();
                (words as f64 * 1.3) as usize // Rough approximation
            }
        };

        let char_count = text.chars().count();
        let byte_count = text.len();
        let word_count = text.split_whitespace().count();

        Ok(json!({
            "token_count": token_count,
            "char_count": char_count,
            "byte_count": byte_count,
            "word_count": word_count,
            "model": model,
            "tokens_per_word": if word_count > 0 { token_count as f64 / word_count as f64 } else { 0.0 }
        }))
    }

    pub async fn memory_store(&self, args: Value) -> Result<Value> {
        let key = args["key"].as_str().context("Missing 'key' parameter")?;
        let value = args.get("value").context("Missing 'value' parameter")?;

        let mut store = self.memory_store.lock().unwrap();
        store.insert(key.to_string(), value.clone());

        Ok(json!({
            "success": true,
            "key": key,
            "stored": true,
            "total_keys": store.len()
        }))
    }

    pub async fn memory_recall(&self, args: Value) -> Result<Value> {
        let store = self.memory_store.lock().unwrap();

        if let Some(key) = args["key"].as_str() {
            if let Some(value) = store.get(key) {
                Ok(json!({
                    "key": key,
                    "value": value,
                    "found": true
                }))
            } else {
                Ok(json!({
                    "key": key,
                    "found": false,
                    "error": "Key not found"
                }))
            }
        } else {
            // List all keys
            let keys: Vec<String> = store.keys().cloned().collect();

            Ok(json!({
                "keys": keys,
                "count": keys.len()
            }))
        }
    }

    pub async fn estimate_cost(&self, args: Value) -> Result<Value> {
        let provider = args["provider"].as_str().context("Missing 'provider' parameter")?;
        let model = args["model"].as_str().context("Missing 'model' parameter")?;
        let input_tokens = args["input_tokens"].as_u64().context("Missing 'input_tokens' parameter")? as usize;
        let output_tokens = args["output_tokens"].as_u64().context("Missing 'output_tokens' parameter")? as usize;

        let (input_price_per_1m, output_price_per_1m) = match (provider, model) {
            // Anthropic Claude pricing (per 1M tokens)
            ("anthropic", "claude-3-opus") => (15.0, 75.0),
            ("anthropic", "claude-3-sonnet") => (3.0, 15.0),
            ("anthropic", "claude-3-haiku") => (0.25, 1.25),
            ("anthropic", "claude-2") => (8.0, 24.0),

            // OpenAI pricing (per 1M tokens)
            ("openai", "gpt-4") => (30.0, 60.0),
            ("openai", "gpt-4-turbo") => (10.0, 30.0),
            ("openai", "gpt-3.5-turbo") => (0.5, 1.5),

            // Ollama (free/local)
            ("ollama", _) => (0.0, 0.0),

            // GLM (example pricing - adjust as needed)
            ("glm", "glm-4") => (1.0, 3.0),

            _ => return Err(anyhow::anyhow!("Unknown provider/model combination: {}/{}", provider, model)),
        };

        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price_per_1m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price_per_1m;
        let total_cost = input_cost + output_cost;

        Ok(json!({
            "provider": provider,
            "model": model,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens,
            "input_cost_usd": input_cost,
            "output_cost_usd": output_cost,
            "total_cost_usd": total_cost,
            "pricing": {
                "input_per_1m_tokens": input_price_per_1m,
                "output_per_1m_tokens": output_price_per_1m
            }
        }))
    }
}
