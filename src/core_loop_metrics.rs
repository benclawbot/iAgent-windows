use chrono::Utc;
use serde::Serialize;
use serde_json::{Value, json};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct CoreLoopMetricEvent {
    schema_version: u32,
    event: String,
    timestamp: String,
    payload: Value,
}

fn metrics_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?
        .join("telemetry")
        .join("core_loop_metrics.jsonl"))
}

fn append_event(event: &str, payload: Value) {
    let Ok(path) = metrics_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = crate::storage::ensure_dir(parent);
    }
    let row = CoreLoopMetricEvent {
        schema_version: SCHEMA_VERSION,
        event: event.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        payload,
    };
    if let Ok(line) = serde_json::to_string(&row) {
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{}", line);
        }
    }
}

pub fn record_suggestions_generated(count: usize) {
    append_event(
        "suggestion.generated",
        json!({
            "count": count
        }),
    );
}

pub fn record_approval_decision(
    approved: bool,
    latency_ms: Option<i64>,
    risk_level: Option<&str>,
    action: Option<&str>,
) {
    append_event(
        "approval.decision",
        json!({
            "approved": approved,
            "latency_ms": latency_ms,
            "risk_level": risk_level,
            "action": action
        }),
    );
}

pub fn record_action_execution(
    action_type: &str,
    success: bool,
    risk_level: Option<&str>,
    undo_available: bool,
) {
    append_event(
        "action.execution",
        json!({
            "action_type": action_type,
            "success": success,
            "risk_level": risk_level,
            "undo_available": undo_available
        }),
    );
}

pub fn record_undo_usage(action_type: &str) {
    append_event(
        "action.undo",
        json!({
            "action_type": action_type,
            "undo_used": true
        }),
    );
}
