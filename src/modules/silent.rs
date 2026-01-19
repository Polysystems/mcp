use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use std::process::Command;
use sysinfo::System;

pub struct SilentModule {
    system: System,
}

impl SilentModule {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "silent_script",
                "description": "Execute bash scripts (silent scripting language)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "string",
                            "description": "Bash script content to execute"
                        },
                        "args": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Arguments to pass to the script"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory for script execution"
                        },
                        "env": {
                            "type": "object",
                            "description": "Environment variables to set"
                        },
                        "timeout": {
                            "type": "number",
                            "description": "Timeout in seconds (default: 300)"
                        }
                    },
                    "required": ["script"]
                }
            }),
            json!({
                "name": "silent_resources",
                "description": "Monitor system resources (GPU/RAM/CPU usage)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "detailed": {
                            "type": "boolean",
                            "description": "Include detailed per-process information"
                        },
                        "process_filter": {
                            "type": "string",
                            "description": "Filter processes by name"
                        }
                    }
                }
            }),
        ]
    }

    pub async fn script(&self, args: Value) -> Result<Value> {
        let script = args["script"].as_str().context("Missing 'script' parameter")?;
        let script_args = args["args"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        }).unwrap_or_default();

        let cwd = args["cwd"].as_str();
        let timeout = args["timeout"].as_u64().unwrap_or(300);

        // Create a temporary script file
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("silent_script_{}.sh", std::process::id()));

        std::fs::write(&script_path, script)
            .context("Failed to write script to temp file")?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms)?;
        }

        // Execute script
        let mut cmd = Command::new("bash");
        cmd.arg(&script_path);

        for arg in script_args {
            cmd.arg(arg);
        }

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        if let Some(env_obj) = args["env"].as_object() {
            for (key, value) in env_obj {
                if let Some(val_str) = value.as_str() {
                    cmd.env(key, val_str);
                }
            }
        }

        let start = std::time::Instant::now();
        let output = cmd.output().context("Failed to execute script")?;
        let duration = start.elapsed();

        // Clean up temp file
        let _ = std::fs::remove_file(&script_path);

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
            "duration_ms": duration.as_millis(),
            "timed_out": duration.as_secs() >= timeout
        }))
    }

    pub async fn resources(&mut self, args: Value) -> Result<Value> {
        let detailed = args["detailed"].as_bool().unwrap_or(false);
        let process_filter = args["process_filter"].as_str();

        // Refresh system information
        self.system.refresh_all();

        // CPU information
        let mut cpu_usage = Vec::new();
        for cpu in self.system.cpus() {
            cpu_usage.push(json!({
                "name": cpu.name(),
                "usage": cpu.cpu_usage(),
                "frequency": cpu.frequency()
            }));
        }

        let global_cpu_usage = self.system.global_cpu_info().cpu_usage();

        // Memory information
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let available_memory = self.system.available_memory();
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        // Swap information
        let total_swap = self.system.total_swap();
        let used_swap = self.system.used_swap();
        let swap_usage_percent = if total_swap > 0 {
            (used_swap as f64 / total_swap as f64) * 100.0
        } else {
            0.0
        };

        // GPU information (attempt to get from nvidia-smi)
        let gpu_info = self.get_gpu_info();

        let mut result = json!({
            "cpu": {
                "global_usage": global_cpu_usage,
                "cores": cpu_usage,
                "core_count": self.system.cpus().len()
            },
            "memory": {
                "total_bytes": total_memory,
                "used_bytes": used_memory,
                "available_bytes": available_memory,
                "usage_percent": memory_usage_percent,
                "total_gb": total_memory as f64 / 1024.0 / 1024.0 / 1024.0,
                "used_gb": used_memory as f64 / 1024.0 / 1024.0 / 1024.0
            },
            "swap": {
                "total_bytes": total_swap,
                "used_bytes": used_swap,
                "usage_percent": swap_usage_percent
            },
            "gpu": gpu_info
        });

        // Add detailed process information if requested
        if detailed {
            let mut processes = Vec::new();

            for (pid, process) in self.system.processes() {
                let name = process.name();

                // Filter by process name if specified
                if let Some(filter) = process_filter {
                    if !name.contains(filter) {
                        continue;
                    }
                }

                processes.push(json!({
                    "pid": pid.as_u32(),
                    "name": name,
                    "cpu_usage": process.cpu_usage(),
                    "memory_bytes": process.memory(),
                    "memory_mb": process.memory() as f64 / 1024.0 / 1024.0,
                    "disk_usage": {
                        "read_bytes": process.disk_usage().read_bytes,
                        "written_bytes": process.disk_usage().written_bytes
                    }
                }));
            }

            // Sort by CPU usage
            processes.sort_by(|a, b| {
                b["cpu_usage"].as_f64().unwrap_or(0.0)
                    .partial_cmp(&a["cpu_usage"].as_f64().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            result["processes"] = json!(processes);
            result["process_count"] = json!(processes.len());
        }

        Ok(result)
    }

    fn get_gpu_info(&self) -> Value {
        // Try to get GPU info from nvidia-smi
        if let Ok(output) = Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=index,name,temperature.gpu,utilization.gpu,utilization.memory,memory.total,memory.used,memory.free",
                "--format=csv,noheader,nounits"
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut gpus = Vec::new();

                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 8 {
                        gpus.push(json!({
                            "index": parts[0].parse::<u32>().ok(),
                            "name": parts[1],
                            "temperature": parts[2].parse::<f64>().ok(),
                            "utilization_gpu": parts[3].parse::<f64>().ok(),
                            "utilization_memory": parts[4].parse::<f64>().ok(),
                            "memory_total_mb": parts[5].parse::<u64>().ok(),
                            "memory_used_mb": parts[6].parse::<u64>().ok(),
                            "memory_free_mb": parts[7].parse::<u64>().ok()
                        }));
                    }
                }

                if !gpus.is_empty() {
                    return json!({
                        "available": true,
                        "count": gpus.len(),
                        "devices": gpus
                    });
                }
            }
        }

        // No GPU info available
        json!({
            "available": false,
            "message": "No GPU information available (nvidia-smi not found or failed)"
        })
    }
}
