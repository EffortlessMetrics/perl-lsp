use serde_json::Value;

/// DAP event from TCP attach session
#[derive(Debug, Clone)]
pub enum DapEvent {
    /// Output event from debugger
    Output { category: String, output: String },
    /// Stopped event (breakpoint hit, step, etc.)
    Stopped { reason: String, thread_id: i32 },
    /// Continued event (execution resumed)
    Continued { thread_id: i32 },
    /// Terminated event (debugger exited)
    Terminated { reason: String },
    /// Error event
    Error { message: String },
}

pub(crate) fn dap_event_from_value(value: &Value) -> Option<DapEvent> {
    let event_type = value.get("type").and_then(|t| t.as_str())?;
    if event_type != "event" {
        return None;
    }

    let event_name = value.get("event").and_then(|e| e.as_str()).unwrap_or("unknown");
    match event_name {
        "output" => output_event(value),
        "stopped" => stopped_event(value),
        "continued" => continued_event(value),
        "terminated" => terminated_event(value),
        _ => {
            tracing::debug!(event = %event_name, "Unhandled DAP event");
            None
        }
    }
}

fn output_event(value: &Value) -> Option<DapEvent> {
    let body = value.get("body");
    let category = body
        .and_then(|b| b.get("category"))
        .and_then(|c| c.as_str())
        .unwrap_or("stdout")
        .to_string();
    let output =
        body.and_then(|b| b.get("output")).and_then(|o| o.as_str()).unwrap_or("").to_string();

    Some(DapEvent::Output { category, output })
}

fn stopped_event(value: &Value) -> Option<DapEvent> {
    let body = value.get("body");
    let reason = body
        .and_then(|b| b.get("reason"))
        .and_then(|r| r.as_str())
        .unwrap_or("unknown")
        .to_string();
    let thread_id =
        body.and_then(|b| b.get("threadId")).and_then(|t| t.as_i64()).unwrap_or(1) as i32;

    Some(DapEvent::Stopped { reason, thread_id })
}

fn continued_event(value: &Value) -> Option<DapEvent> {
    let body = value.get("body");
    let thread_id =
        body.and_then(|b| b.get("threadId")).and_then(|t| t.as_i64()).unwrap_or(1) as i32;

    Some(DapEvent::Continued { thread_id })
}

fn terminated_event(value: &Value) -> Option<DapEvent> {
    let reason = value
        .get("body")
        .and_then(|b| b.get("reason"))
        .and_then(|r| r.as_str())
        .unwrap_or("unknown")
        .to_string();

    Some(DapEvent::Terminated { reason })
}
