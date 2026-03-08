use serde_json::{json, Value};
use anyhow::{Result, Context as _};
use chrono::{Local, Utc, DateTime, Duration as ChronoDuration};
use chrono_tz::Tz;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep as tokio_sleep, Duration};

pub struct TimeModule {
    scheduled_tasks: Arc<Mutex<HashMap<String, ScheduledTask>>>,
    stopwatches: Arc<Mutex<HashMap<String, Stopwatch>>>,
    timers: Arc<Mutex<HashMap<String, TimerEntry>>>,
    alarms: Arc<Mutex<HashMap<String, Alarm>>>,
}

struct ScheduledTask {
    id: String,
    execute_at: DateTime<Utc>,
    callback: String,
    args: Value,
    executed: bool,
}

#[derive(Clone, Debug)]
struct Stopwatch {
    name: String,
    started_at: Option<DateTime<Utc>>,
    elapsed_before_stop: i64, // milliseconds accumulated from previous start/stop cycles
    laps: Vec<LapEntry>,
    running: bool,
}

#[derive(Clone, Debug)]
struct LapEntry {
    lap_number: usize,
    #[allow(dead_code)]
    timestamp: DateTime<Utc>,
    split_ms: i64,
    total_ms: i64,
}

#[derive(Clone, Debug)]
struct TimerEntry {
    name: String,
    duration_ms: i64,
    started_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
struct Alarm {
    name: String,
    alarm_time: DateTime<Utc>,
    created_at: DateTime<Utc>,
    triggered: bool,
    message: Option<String>,
}

impl Default for TimeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeModule {
    pub fn new() -> Self {
        Self {
            scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
            stopwatches: Arc::new(Mutex::new(HashMap::new())),
            timers: Arc::new(Mutex::new(HashMap::new())),
            alarms: Arc::new(Mutex::new(HashMap::new())),
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
            json!({
                "name": "time_timezone",
                "description": "Convert timestamps between IANA timezones or list available timezone names",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["convert", "list"],
                            "description": "Action to perform (default: convert)"
                        },
                        "timestamp": {
                            "type": "string",
                            "description": "ISO8601/RFC3339 timestamp to convert (default: current time)"
                        },
                        "from_tz": {
                            "type": "string",
                            "description": "Source timezone, e.g. 'America/New_York', 'UTC', 'Europe/London'"
                        },
                        "to_tz": {
                            "type": "string",
                            "description": "Target timezone"
                        },
                        "filter": {
                            "type": "string",
                            "description": "Filter timezone list by substring (for action=list)"
                        }
                    }
                }
            }),
            json!({
                "name": "time_stopwatch",
                "description": "Manage named stopwatches for timing operations. Supports start, stop, lap, reset, status, and list actions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Stopwatch name (default: 'default')"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["start", "stop", "lap", "reset", "status", "list"],
                            "description": "Action to perform (default: status)"
                        }
                    }
                }
            }),
            json!({
                "name": "time_timer",
                "description": "Set countdown timers with names. Start a timer, check remaining time, cancel, or list all timers.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Timer name (default: 'default')"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["start", "check", "cancel", "list"],
                            "description": "Action to perform (default: check)"
                        },
                        "duration": {
                            "type": "number",
                            "description": "Duration value (required for start)"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["seconds", "minutes", "hours"],
                            "description": "Time unit for duration (default: seconds)"
                        }
                    }
                }
            }),
            json!({
                "name": "time_alarm",
                "description": "Set alarms for specific times. Check, list, or cancel alarms. Alarms are in-memory only.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Alarm name"
                        },
                        "action": {
                            "type": "string",
                            "enum": ["set", "check", "cancel", "list"],
                            "description": "Action to perform (default: list)"
                        },
                        "time": {
                            "type": "string",
                            "description": "ISO8601/RFC3339 timestamp for alarm (for action=set)"
                        },
                        "in_seconds": {
                            "type": "number",
                            "description": "Alternative: set alarm N seconds from now"
                        },
                        "message": {
                            "type": "string",
                            "description": "Optional message associated with the alarm"
                        }
                    }
                }
            }),
        ]
    }

    // ── Existing tools ──────────────────────────────────────────────────

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

        if tasks.remove(task_id).is_some() {
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

    // ── Timezone ────────────────────────────────────────────────────────

    pub async fn timezone(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("convert");

        match action {
            "convert" => self.timezone_convert(args).await,
            "list" => self.timezone_list(args).await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn timezone_convert(&self, args: Value) -> Result<Value> {
        let to_tz_str = args["to_tz"].as_str().context("Missing 'to_tz' parameter")?;
        let to_tz: Tz = to_tz_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", to_tz_str))?;

        let source_dt = if let Some(ts) = args["timestamp"].as_str() {
            DateTime::parse_from_rfc3339(ts)
                .context("Invalid timestamp (expected RFC3339/ISO8601)")?
                .with_timezone(&Utc)
        } else {
            Utc::now()
        };

        let converted = if let Some(from_tz_str) = args["from_tz"].as_str() {
            let from_tz: Tz = from_tz_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", from_tz_str))?;
            let in_source = source_dt.with_timezone(&from_tz);
            in_source.with_timezone(&to_tz)
        } else {
            source_dt.with_timezone(&to_tz)
        };

        Ok(json!({
            "original": source_dt.to_rfc3339(),
            "converted": converted.to_string(),
            "to_tz": to_tz_str,
            "from_tz": args["from_tz"].as_str().unwrap_or("UTC"),
            "utc": source_dt.to_rfc3339(),
            "unix": source_dt.timestamp()
        }))
    }

    async fn timezone_list(&self, args: Value) -> Result<Value> {
        let filter = args["filter"].as_str().unwrap_or("");

        let timezones: Vec<&str> = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name())
            .filter(|name| {
                if filter.is_empty() {
                    true
                } else {
                    name.to_lowercase().contains(&filter.to_lowercase())
                }
            })
            .collect();

        Ok(json!({
            "timezones": timezones,
            "count": timezones.len(),
            "filter": if filter.is_empty() { None } else { Some(filter) }
        }))
    }

    // ── Stopwatch ───────────────────────────────────────────────────────

    pub async fn stopwatch(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("status");
        let name = args["name"].as_str().unwrap_or("default");

        match action {
            "start" => self.stopwatch_start(name).await,
            "stop" => self.stopwatch_stop(name).await,
            "lap" => self.stopwatch_lap(name).await,
            "reset" => self.stopwatch_reset(name).await,
            "status" => self.stopwatch_status(name).await,
            "list" => self.stopwatch_list().await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn stopwatch_start(&self, name: &str) -> Result<Value> {
        let mut watches = self.stopwatches.lock().unwrap();
        let now = Utc::now();

        if let Some(sw) = watches.get_mut(name) {
            if sw.running {
                return Err(anyhow::anyhow!("Stopwatch '{}' is already running", name));
            }
            sw.started_at = Some(now);
            sw.running = true;
            Ok(json!({
                "name": name,
                "action": "resumed",
                "elapsed_before_ms": sw.elapsed_before_stop,
                "started_at": now.to_rfc3339()
            }))
        } else {
            watches.insert(name.to_string(), Stopwatch {
                name: name.to_string(),
                started_at: Some(now),
                elapsed_before_stop: 0,
                laps: Vec::new(),
                running: true,
            });
            Ok(json!({
                "name": name,
                "action": "started",
                "started_at": now.to_rfc3339()
            }))
        }
    }

    async fn stopwatch_stop(&self, name: &str) -> Result<Value> {
        let mut watches = self.stopwatches.lock().unwrap();
        let sw = watches.get_mut(name)
            .with_context(|| format!("Stopwatch '{}' not found", name))?;

        if !sw.running {
            return Err(anyhow::anyhow!("Stopwatch '{}' is not running", name));
        }

        let now = Utc::now();
        let elapsed_this_run = now.signed_duration_since(sw.started_at.unwrap()).num_milliseconds();
        sw.elapsed_before_stop += elapsed_this_run;
        sw.started_at = None;
        sw.running = false;

        let total_ms = sw.elapsed_before_stop;

        Ok(json!({
            "name": name,
            "action": "stopped",
            "elapsed_ms": total_ms,
            "elapsed_secs": total_ms as f64 / 1000.0,
            "elapsed_formatted": format_duration_ms(total_ms),
            "laps": sw.laps.len()
        }))
    }

    async fn stopwatch_lap(&self, name: &str) -> Result<Value> {
        let mut watches = self.stopwatches.lock().unwrap();
        let sw = watches.get_mut(name)
            .with_context(|| format!("Stopwatch '{}' not found", name))?;

        if !sw.running {
            return Err(anyhow::anyhow!("Stopwatch '{}' is not running", name));
        }

        let now = Utc::now();
        let elapsed_this_run = now.signed_duration_since(sw.started_at.unwrap()).num_milliseconds();
        let total_ms = sw.elapsed_before_stop + elapsed_this_run;

        let last_lap_total = sw.laps.last().map(|l| l.total_ms).unwrap_or(0);
        let split_ms = total_ms - last_lap_total;
        let lap_number = sw.laps.len() + 1;

        let lap = LapEntry {
            lap_number,
            timestamp: now,
            split_ms,
            total_ms,
        };
        sw.laps.push(lap);

        Ok(json!({
            "name": name,
            "lap_number": lap_number,
            "split_ms": split_ms,
            "split_formatted": format_duration_ms(split_ms),
            "total_ms": total_ms,
            "total_formatted": format_duration_ms(total_ms),
            "timestamp": now.to_rfc3339()
        }))
    }

    async fn stopwatch_reset(&self, name: &str) -> Result<Value> {
        let mut watches = self.stopwatches.lock().unwrap();
        let removed = watches.remove(name).is_some();

        Ok(json!({
            "name": name,
            "action": "reset",
            "removed": removed
        }))
    }

    async fn stopwatch_status(&self, name: &str) -> Result<Value> {
        let watches = self.stopwatches.lock().unwrap();

        if let Some(sw) = watches.get(name) {
            let now = Utc::now();
            let total_ms = if sw.running {
                let elapsed_this_run = now.signed_duration_since(sw.started_at.unwrap()).num_milliseconds();
                sw.elapsed_before_stop + elapsed_this_run
            } else {
                sw.elapsed_before_stop
            };

            let laps: Vec<Value> = sw.laps.iter().map(|lap| {
                json!({
                    "lap": lap.lap_number,
                    "split_ms": lap.split_ms,
                    "split_formatted": format_duration_ms(lap.split_ms),
                    "total_ms": lap.total_ms,
                    "total_formatted": format_duration_ms(lap.total_ms)
                })
            }).collect();

            Ok(json!({
                "name": name,
                "running": sw.running,
                "elapsed_ms": total_ms,
                "elapsed_secs": total_ms as f64 / 1000.0,
                "elapsed_formatted": format_duration_ms(total_ms),
                "laps": laps,
                "lap_count": sw.laps.len(),
                "found": true
            }))
        } else {
            Ok(json!({
                "name": name,
                "found": false
            }))
        }
    }

    async fn stopwatch_list(&self) -> Result<Value> {
        let watches = self.stopwatches.lock().unwrap();
        let now = Utc::now();

        let list: Vec<Value> = watches.values().map(|sw| {
            let total_ms = if sw.running {
                let elapsed = now.signed_duration_since(sw.started_at.unwrap()).num_milliseconds();
                sw.elapsed_before_stop + elapsed
            } else {
                sw.elapsed_before_stop
            };

            json!({
                "name": sw.name,
                "running": sw.running,
                "elapsed_ms": total_ms,
                "elapsed_formatted": format_duration_ms(total_ms),
                "laps": sw.laps.len()
            })
        }).collect();

        Ok(json!({
            "stopwatches": list,
            "count": list.len()
        }))
    }

    // ── Timer ───────────────────────────────────────────────────────────

    pub async fn timer(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("check");
        let name = args["name"].as_str().unwrap_or("default");

        match action {
            "start" => self.timer_start(name, &args).await,
            "check" => self.timer_check(name).await,
            "cancel" => self.timer_cancel(name).await,
            "list" => self.timer_list().await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn timer_start(&self, name: &str, args: &Value) -> Result<Value> {
        let duration = args["duration"].as_f64()
            .context("Missing 'duration' parameter for start action")?;
        let unit = args["unit"].as_str().unwrap_or("seconds");

        let duration_ms = match unit {
            "minutes" => (duration * 60_000.0) as i64,
            "hours" => (duration * 3_600_000.0) as i64,
            _ => (duration * 1000.0) as i64,
        };

        let now = Utc::now();
        let entry = TimerEntry {
            name: name.to_string(),
            duration_ms,
            started_at: now,
        };

        let mut timers = self.timers.lock().unwrap();
        timers.insert(name.to_string(), entry);

        let ends_at = now + ChronoDuration::milliseconds(duration_ms);

        Ok(json!({
            "name": name,
            "action": "started",
            "duration_ms": duration_ms,
            "duration_formatted": format_duration_ms(duration_ms),
            "started_at": now.to_rfc3339(),
            "ends_at": ends_at.to_rfc3339()
        }))
    }

    async fn timer_check(&self, name: &str) -> Result<Value> {
        let timers = self.timers.lock().unwrap();
        let timer = timers.get(name)
            .with_context(|| format!("Timer '{}' not found", name))?;

        let now = Utc::now();
        let elapsed_ms = now.signed_duration_since(timer.started_at).num_milliseconds();
        let remaining_ms = (timer.duration_ms - elapsed_ms).max(0);
        let expired = elapsed_ms >= timer.duration_ms;
        let percent = ((elapsed_ms as f64 / timer.duration_ms as f64) * 100.0).min(100.0);

        let ends_at = timer.started_at + ChronoDuration::milliseconds(timer.duration_ms);

        Ok(json!({
            "name": name,
            "expired": expired,
            "remaining_ms": remaining_ms,
            "remaining_secs": remaining_ms as f64 / 1000.0,
            "remaining_formatted": format_duration_ms(remaining_ms),
            "elapsed_ms": elapsed_ms,
            "percent_complete": format!("{:.1}", percent),
            "duration_ms": timer.duration_ms,
            "started_at": timer.started_at.to_rfc3339(),
            "ends_at": ends_at.to_rfc3339()
        }))
    }

    async fn timer_cancel(&self, name: &str) -> Result<Value> {
        let mut timers = self.timers.lock().unwrap();
        let removed = timers.remove(name).is_some();

        Ok(json!({
            "name": name,
            "action": "cancelled",
            "removed": removed
        }))
    }

    async fn timer_list(&self) -> Result<Value> {
        let timers = self.timers.lock().unwrap();
        let now = Utc::now();

        let list: Vec<Value> = timers.values().map(|t| {
            let elapsed_ms = now.signed_duration_since(t.started_at).num_milliseconds();
            let remaining_ms = (t.duration_ms - elapsed_ms).max(0);
            let expired = elapsed_ms >= t.duration_ms;

            json!({
                "name": t.name,
                "expired": expired,
                "remaining_ms": remaining_ms,
                "remaining_formatted": format_duration_ms(remaining_ms),
                "duration_ms": t.duration_ms
            })
        }).collect();

        Ok(json!({
            "timers": list,
            "count": list.len()
        }))
    }

    // ── Alarm ───────────────────────────────────────────────────────────

    pub async fn alarm(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("list");

        match action {
            "set" => self.alarm_set(args).await,
            "check" => self.alarm_check(args).await,
            "cancel" => self.alarm_cancel(args).await,
            "list" => self.alarm_list().await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    async fn alarm_set(&self, args: Value) -> Result<Value> {
        let name = args["name"].as_str().context("Missing 'name' parameter")?;
        let message = args["message"].as_str().map(|s| s.to_string());
        let now = Utc::now();

        let alarm_time = if let Some(ts) = args["time"].as_str() {
            DateTime::parse_from_rfc3339(ts)
                .context("Invalid timestamp (expected RFC3339/ISO8601)")?
                .with_timezone(&Utc)
        } else if let Some(secs) = args["in_seconds"].as_f64() {
            now + ChronoDuration::milliseconds((secs * 1000.0) as i64)
        } else {
            return Err(anyhow::anyhow!("Must provide either 'time' or 'in_seconds'"));
        };

        let alarm = Alarm {
            name: name.to_string(),
            alarm_time,
            created_at: now,
            triggered: alarm_time <= now,
            message: message.clone(),
        };

        let mut alarms = self.alarms.lock().unwrap();
        alarms.insert(name.to_string(), alarm);

        let seconds_until = alarm_time.signed_duration_since(now).num_seconds();

        Ok(json!({
            "name": name,
            "action": "set",
            "alarm_time": alarm_time.to_rfc3339(),
            "seconds_until": seconds_until,
            "message": message,
            "already_triggered": alarm_time <= now
        }))
    }

    async fn alarm_check(&self, args: Value) -> Result<Value> {
        let name = args["name"].as_str().context("Missing 'name' parameter")?;

        let mut alarms = self.alarms.lock().unwrap();
        let alarm = alarms.get_mut(name)
            .with_context(|| format!("Alarm '{}' not found", name))?;

        let now = Utc::now();
        let triggered = now >= alarm.alarm_time;
        if triggered {
            alarm.triggered = true;
        }

        let seconds_until = alarm.alarm_time.signed_duration_since(now).num_seconds();

        Ok(json!({
            "name": name,
            "triggered": triggered,
            "alarm_time": alarm.alarm_time.to_rfc3339(),
            "seconds_until": seconds_until,
            "message": alarm.message,
            "created_at": alarm.created_at.to_rfc3339()
        }))
    }

    async fn alarm_cancel(&self, args: Value) -> Result<Value> {
        let name = args["name"].as_str().context("Missing 'name' parameter")?;

        let mut alarms = self.alarms.lock().unwrap();
        let removed = alarms.remove(name).is_some();

        Ok(json!({
            "name": name,
            "action": "cancelled",
            "removed": removed
        }))
    }

    async fn alarm_list(&self) -> Result<Value> {
        let mut alarms = self.alarms.lock().unwrap();
        let now = Utc::now();

        let list: Vec<Value> = alarms.values_mut().map(|a| {
            if now >= a.alarm_time {
                a.triggered = true;
            }
            let seconds_until = a.alarm_time.signed_duration_since(now).num_seconds();

            json!({
                "name": a.name,
                "triggered": a.triggered,
                "alarm_time": a.alarm_time.to_rfc3339(),
                "seconds_until": seconds_until,
                "message": a.message,
                "created_at": a.created_at.to_rfc3339()
            })
        }).collect();

        Ok(json!({
            "alarms": list,
            "count": list.len(),
            "current_time": now.to_rfc3339()
        }))
    }
}

fn format_duration_ms(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", ms)
    }
}
