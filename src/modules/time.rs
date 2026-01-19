use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use chrono::{Local, Utc, DateTime, Duration as ChronoDuration};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep as tokio_sleep, Duration};

pub struct TimeModule {
    scheduled_tasks: Arc<Mutex<HashMap<String, ScheduledTask>>>,
}

struct ScheduledTask {
    id: String,
    execute_at: DateTime<Utc>,
    callback: String,
    args: Value,
    executed: bool,
}

impl TimeModule {
    pub fn new() -> Self {
        Self {
            scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "time_now",
                "description": "Get current timestamp in various formats",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "enum": ["unix", "iso8601", "rfc3339", "rfc2822", "custom"],
                            "description": "Timestamp format (default: iso8601)"
                        },
                        "custom_format": {
                            "type": "string",
                            "description": "Custom format string (when format=custom)"
                        },
                        "timezone": {
                            "type": "string",
                            "enum": ["local", "utc"],
                            "description": "Timezone (default: local)"
                        }
                    }
                }
            }),
            json!({
                "name": "time_sleep",
                "description": "Delay execution for a specified duration",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "duration": {
                            "type": "number",
                            "description": "Duration in seconds"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["seconds", "milliseconds", "minutes", "hours"],
                            "description": "Time unit (default: seconds)"
                        }
                    },
                    "required": ["duration"]
                }
            }),
            json!({
                "name": "time_schedule",
                "description": "Schedule a task for future execution (in-memory, process lifetime)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "Unique task identifier"
                        },
                        "execute_in": {
                            "type": "number",
                            "description": "Seconds until execution"
                        },
                        "execute_at": {
                            "type": "string",
                            "description": "ISO8601 timestamp for execution"
                        },
                        "callback": {
                            "type": "string",
                            "description": "Callback identifier/name"
                        },
                        "args": {
                            "type": "object",
                            "description": "Arguments to pass to callback"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["create", "cancel", "list", "status"],
                            "description": "Action to perform (default: create)"
                        }
                    }
                }
            }),
        ]
    }

    pub async fn now(&self, args: Value) -> Result<Value> {
        let format = args["format"].as_str().unwrap_or("iso8601");
        let timezone = args["timezone"].as_str().unwrap_or("local");
        let custom_format = args["custom_format"].as_str();

        let (local_time, utc_time) = (Local::now(), Utc::now());

        let time_to_use = match timezone {
            "utc" => utc_time.with_timezone(&Utc),
            _ => local_time.with_timezone(&Local).with_timezone(&Utc),
        };

        let formatted = match format {
            "unix" => time_to_use.timestamp().to_string(),
            "iso8601" => time_to_use.to_rfc3339(),
            "rfc3339" => time_to_use.to_rfc3339(),
            "rfc2822" => time_to_use.to_rfc2822(),
            "custom" => {
                if let Some(fmt) = custom_format {
                    time_to_use.format(fmt).to_string()
                } else {
                    return Err(anyhow::anyhow!("custom_format required when format=custom"));
                }
            }
            _ => time_to_use.to_rfc3339(),
        };

        Ok(json!({
            "timestamp": formatted,
            "unix": time_to_use.timestamp(),
            "unix_millis": time_to_use.timestamp_millis(),
            "unix_nanos": time_to_use.timestamp_nanos_opt(),
            "timezone": timezone,
            "format": format,
            "local": local_time.to_rfc3339(),
            "utc": utc_time.to_rfc3339()
        }))
    }

    pub async fn sleep(&self, args: Value) -> Result<Value> {
        let duration = args["duration"].as_f64().context("Missing 'duration' parameter")?;
        let unit = args["unit"].as_str().unwrap_or("seconds");

        let sleep_duration = match unit {
            "milliseconds" => Duration::from_millis(duration as u64),
            "minutes" => Duration::from_secs((duration * 60.0) as u64),
            "hours" => Duration::from_secs((duration * 3600.0) as u64),
            _ => Duration::from_secs(duration as u64),
        };

        let start = std::time::Instant::now();
        tokio_sleep(sleep_duration).await;
        let actual_duration = start.elapsed();

        Ok(json!({
            "success": true,
            "requested_duration": duration,
            "unit": unit,
            "actual_duration_ms": actual_duration.as_millis(),
            "actual_duration_secs": actual_duration.as_secs_f64()
        }))
    }

    pub async fn schedule(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("create");

        match action {
            "create" => self.schedule_create(args).await,
            "cancel" => self.schedule_cancel(args).await,
            "list" => self.schedule_list(args).await,
            "status" => self.schedule_status(args).await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn schedule_create(&self, args: Value) -> Result<Value> {
        let task_id = args["task_id"].as_str()
            .context("Missing 'task_id' parameter")?
            .to_string();

        let callback = args["callback"].as_str()
            .context("Missing 'callback' parameter")?
            .to_string();

        let task_args = args["args"].clone();

        let execute_at = if let Some(execute_in) = args["execute_in"].as_f64() {
            Utc::now() + ChronoDuration::seconds(execute_in as i64)
        } else if let Some(timestamp_str) = args["execute_at"].as_str() {
            DateTime::parse_from_rfc3339(timestamp_str)
                .context("Invalid ISO8601 timestamp")?
                .with_timezone(&Utc)
        } else {
            return Err(anyhow::anyhow!("Must provide either 'execute_in' or 'execute_at'"));
        };

        let task = ScheduledTask {
            id: task_id.clone(),
            execute_at,
            callback: callback.clone(),
            args: task_args,
            executed: false,
        };

        let mut tasks = self.scheduled_tasks.lock().unwrap();
        tasks.insert(task_id.clone(), task);

        Ok(json!({
            "success": true,
            "task_id": task_id,
            "execute_at": execute_at.to_rfc3339(),
            "callback": callback,
            "message": "Task scheduled (in-memory, will be lost on process restart)"
        }))
    }

    async fn schedule_cancel(&self, args: Value) -> Result<Value> {
        let task_id = args["task_id"].as_str().context("Missing 'task_id' parameter")?;

        let mut tasks = self.scheduled_tasks.lock().unwrap();

        if let Some(_) = tasks.remove(task_id) {
            Ok(json!({
                "success": true,
                "task_id": task_id,
                "message": "Task cancelled"
            }))
        } else {
            Err(anyhow::anyhow!("Task not found: {}", task_id))
        }
    }

    async fn schedule_list(&self, _args: Value) -> Result<Value> {
        let tasks = self.scheduled_tasks.lock().unwrap();
        let now = Utc::now();

        let task_list: Vec<Value> = tasks.values().map(|task| {
            let time_until = task.execute_at.signed_duration_since(now);

            json!({
                "task_id": task.id,
                "callback": task.callback,
                "execute_at": task.execute_at.to_rfc3339(),
                "executed": task.executed,
                "seconds_until": time_until.num_seconds(),
                "overdue": time_until.num_seconds() < 0
            })
        }).collect();

        Ok(json!({
            "tasks": task_list,
            "count": task_list.len(),
            "current_time": now.to_rfc3339()
        }))
    }

    async fn schedule_status(&self, args: Value) -> Result<Value> {
        let task_id = args["task_id"].as_str().context("Missing 'task_id' parameter")?;

        let tasks = self.scheduled_tasks.lock().unwrap();

        if let Some(task) = tasks.get(task_id) {
            let now = Utc::now();
            let time_until = task.execute_at.signed_duration_since(now);

            Ok(json!({
                "task_id": task.id,
                "callback": task.callback,
                "args": task.args,
                "execute_at": task.execute_at.to_rfc3339(),
                "executed": task.executed,
                "seconds_until": time_until.num_seconds(),
                "overdue": time_until.num_seconds() < 0,
                "current_time": now.to_rfc3339()
            }))
        } else {
            Err(anyhow::anyhow!("Task not found: {}", task_id))
        }
    }
}
