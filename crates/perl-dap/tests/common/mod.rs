//! Shared test helpers for DAP end-to-end workflow tests.
//!
//! `DapWorkflowSession` wraps a `DebugAdapter` and an event `Receiver` with
//! higher-level helpers that chain the request → event → response cycles
//! required to drive a real `perl -d` debug session in tests.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

// ─── Public surface ───────────────────────────────────────────────────────────

/// Information extracted from a `stopped` event.
#[derive(Debug, Clone)]
pub struct StoppedInfo {
    /// Stopped reason, e.g. `"breakpoint"`, `"step"`, `"entry"`.
    pub reason: String,
    /// Thread ID (from `stopped.body.threadId`).
    pub thread_id: i64,
}

/// High-level handle for a DAP workflow test session.
///
/// Wraps `DebugAdapter` with helpers that handle protocol sequencing:
/// initialize → launch → setBreakpoints → configurationDone → wait_stopped →
/// stack_trace → scopes → variables → continue/step → wait_stopped → disconnect.
pub struct DapWorkflowSession {
    pub adapter: DebugAdapter,
    pub rx: Receiver<DapMessage>,
    pub timeout: Duration,
    seq: i64,
}

impl DapWorkflowSession {
    /// Create a new session and send `initialize`.
    ///
    /// Returns an error if initialization fails or the `initialized` event is
    /// not received within `timeout`.
    pub fn new(timeout: Duration) -> Result<Self, String> {
        let mut adapter = DebugAdapter::new();
        let (tx, rx) = channel();
        adapter.set_event_sender(tx);

        let mut session = Self {
            adapter,
            rx,
            timeout,
            seq: 0,
        };

        let resp = session.request("initialize", None);
        session.expect_success(&resp, "initialize")?;
        session.drain_until_event("initialized")?;

        Ok(session)
    }

    /// Launch a script with `stopOnEntry: false`.
    ///
    /// Callers must call `set_breakpoints` and `configuration_done` before
    /// `wait_stopped` to follow the DAP ordering requirement.
    pub fn launch(&mut self, script_path: &str) -> Result<(), String> {
        let args = json!({
            "program": script_path,
            "args": [],
            "stopOnEntry": false,
            "env": {
                "PERL_PERTURB_KEYS": "0",
                "PERL_HASH_SEED": "0",
                "LC_ALL": "C",
                "TZ": "UTC"
            }
        });
        let resp = self.request("launch", Some(args));
        self.expect_success(&resp, "launch")?;
        Ok(())
    }

    /// Attach to a running process with optional stopOnEntry.
    ///
    /// Callers must call `set_breakpoints` and `configuration_done` before
    /// `wait_stopped` to follow the DAP ordering requirement (though attach
    /// emits a stopped event immediately).
    pub fn attach(&mut self, process_id: u32, stop_on_entry: bool) -> Result<(), String> {
        let args = json!({
            "processId": process_id,
            "stopOnEntry": stop_on_entry
        });
        let resp = self.request("attach", Some(args));
        self.expect_success(&resp, "attach")?;
        Ok(())
    }

    /// Send `setBreakpoints` for the given `lines` in `source_path`.
    ///
    /// Returns the raw `setBreakpoints` response body.
    pub fn set_breakpoints(
        &mut self,
        source_path: &str,
        lines: &[u64],
    ) -> Result<Option<Value>, String> {
        let breakpoints: Vec<Value> = lines.iter().map(|&l| json!({"line": l})).collect();
        let args = json!({
            "source": { "path": source_path },
            "breakpoints": breakpoints
        });
        let resp = self.request("setBreakpoints", Some(args));
        self.expect_success(&resp, "setBreakpoints")
    }

    /// Send `configurationDone`.
    pub fn configuration_done(&mut self) -> Result<(), String> {
        let resp = self.request("configurationDone", None);
        self.expect_success(&resp, "configurationDone")?;
        Ok(())
    }

    /// Block until a `stopped` event arrives; drain and discard other events.
    ///
    /// Returns `StoppedInfo` with `reason` and `thread_id`.
    /// Line information is not present in the `stopped` event body; use
    /// `stack_trace()` to obtain the current source line after stopping.
    pub fn wait_stopped(&self) -> Result<StoppedInfo, String> {
        let msg = self.drain_until_event("stopped")?;
        let body = match &msg {
            DapMessage::Event { body, .. } => body.clone().unwrap_or(Value::Null),
            _ => return Err("expected Event message for `stopped`".to_string()),
        };

        let reason = body
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let thread_id = body.get("threadId").and_then(Value::as_i64).unwrap_or(1);

        Ok(StoppedInfo { reason, thread_id })
    }

    /// Retrieve the top stack frame for `thread_id`.
    ///
    /// Returns `(frame_id, source_path_str, frame_line)`.
    /// `frame_line` is the 1-based source line reported by the debugger, or 0
    /// if the frame does not carry line information.
    pub fn stack_trace(&mut self, thread_id: i64) -> Result<(i64, String, i64), String> {
        let args = json!({"threadId": thread_id, "startFrame": 0, "levels": 1});
        let resp = self.request("stackTrace", Some(args));
        let body = self.expect_success(&resp, "stackTrace")?;

        let body = body.ok_or("stackTrace response had no body")?;
        let frames = body
            .get("stackFrames")
            .and_then(Value::as_array)
            .ok_or("stackTrace body missing `stackFrames` array")?;

        let frame = frames.first().ok_or("stackTrace returned empty frames")?;
        let frame_id = frame
            .get("id")
            .and_then(Value::as_i64)
            .ok_or("stack frame missing `id`")?;
        let source_path = frame
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let frame_line = frame.get("line").and_then(Value::as_i64).unwrap_or(0);

        Ok((frame_id, source_path, frame_line))
    }

    /// Retrieve the `variablesReference` for the `Globals` scope in `frame_id`.
    pub fn scopes_globals_ref(&mut self, frame_id: i64) -> Result<i64, String> {
        let args = json!({"frameId": frame_id});
        let resp = self.request("scopes", Some(args));
        let body = self.expect_success(&resp, "scopes")?;

        let body = body.ok_or("scopes response had no body")?;
        let scopes = body
            .get("scopes")
            .and_then(Value::as_array)
            .ok_or("scopes body missing `scopes` array")?;

        // Find the "Globals" scope by presentation hint or name.
        for scope in scopes {
            let is_globals = scope
                .get("presentationHint")
                .and_then(Value::as_str)
                .map(|h| h == "globals")
                .unwrap_or(false)
                || scope
                    .get("name")
                    .and_then(Value::as_str)
                    .map(|n| n == "Globals")
                    .unwrap_or(false);

            if is_globals {
                let vars_ref = scope
                    .get("variablesReference")
                    .and_then(Value::as_i64)
                    .ok_or("Globals scope missing `variablesReference`")?;
                return Ok(vars_ref);
            }
        }

        Err("No Globals scope found in scopes response".to_string())
    }

    /// Retrieve the `variablesReference` for the `Locals` scope in `frame_id`.
    ///
    /// Uses the `frame_id * 10 + 1` encoding from `frames.rs`.
    pub fn scopes_locals_ref(&mut self, frame_id: i64) -> Result<i64, String> {
        let args = json!({"frameId": frame_id});
        let resp = self.request("scopes", Some(args));
        let body = self.expect_success(&resp, "scopes")?;

        let body = body.ok_or("scopes response had no body")?;
        let scopes = body
            .get("scopes")
            .and_then(Value::as_array)
            .ok_or("scopes body missing `scopes` array")?;

        // Find the "Locals" scope by presentation hint or name.
        for scope in scopes {
            let is_locals = scope
                .get("presentationHint")
                .and_then(Value::as_str)
                .map(|h| h == "locals")
                .unwrap_or(false)
                || scope
                    .get("name")
                    .and_then(Value::as_str)
                    .map(|n| n == "Locals")
                    .unwrap_or(false);

            if is_locals {
                let vars_ref = scope
                    .get("variablesReference")
                    .and_then(Value::as_i64)
                    .ok_or("Locals scope missing `variablesReference`")?;
                return Ok(vars_ref);
            }
        }

        Err("No Locals scope found in scopes response".to_string())
    }

    /// Retrieve variables for `variables_reference`.
    pub fn variables(&mut self, variables_reference: i64) -> Result<Vec<Value>, String> {
        let args = json!({"variablesReference": variables_reference});
        let resp = self.request("variables", Some(args));
        let body = self.expect_success(&resp, "variables")?;

        let body = body.ok_or("variables response had no body")?;
        let vars = body
            .get("variables")
            .and_then(Value::as_array)
            .ok_or("variables body missing `variables` array")?
            .clone();

        Ok(vars)
    }

    /// Send `evaluate` request with `expression`, `context`, and optional `frame_id`.
    ///
    /// Returns the full response body as a `Value`. The caller extracts
    /// `result`, `type`, and `variablesReference` fields as needed.
    ///
    /// # Arguments
    /// * `expression` — the Perl expression to evaluate (e.g. `"$scalar"`, `"@array"`)
    /// * `context` — evaluation context string (e.g. `"hover"`, `"watch"`, `"repl"`, `"clipboard"`)
    /// * `frame_id` — optional stack frame id; currently accepted but unused by `handle_evaluate()`
    pub fn evaluate(
        &mut self,
        expression: &str,
        context: &str,
        frame_id: Option<i64>,
    ) -> Result<Value, String> {
        let mut args = json!({
            "expression": expression,
            "context": context
        });
        if let Some(fid) = frame_id {
            args["frameId"] = serde_json::json!(fid);
        }
        let resp = self.request("evaluate", Some(args));
        let body = self.expect_success(&resp, "evaluate")?;
        body.ok_or_else(|| "evaluate response had no body".to_string())
    }

    /// Send `continue` for `thread_id`.
    pub fn continue_exec(&mut self, thread_id: i64) -> Result<(), String> {
        let args = json!({"threadId": thread_id});
        let resp = self.request("continue", Some(args));
        self.expect_success(&resp, "continue")?;
        Ok(())
    }

    /// Send `next` (step-over) for `thread_id`.
    pub fn step_over(&mut self, thread_id: i64) -> Result<(), String> {
        let args = json!({"threadId": thread_id});
        let resp = self.request("next", Some(args));
        self.expect_success(&resp, "next")?;
        Ok(())
    }

    /// Send `stepIn` for `thread_id`.
    pub fn step_into(&mut self, thread_id: i64) -> Result<(), String> {
        let args = json!({"threadId": thread_id});
        let resp = self.request("stepIn", Some(args));
        self.expect_success(&resp, "stepIn")?;
        Ok(())
    }

    /// Send `disconnect` and wait for the `terminated` event.
    pub fn disconnect(&mut self) -> Result<(), String> {
        let resp = self.request("disconnect", Some(json!({})));
        self.expect_success(&resp, "disconnect")?;
        let _ = self.drain_until_event("terminated");
        Ok(())
    }

    // ─── Private helpers ──────────────────────────────────────────────────────

    /// Issue a low-level DAP request and return the raw response.
    ///
    /// This is the primitive used by all higher-level helpers (e.g.
    /// `launch`, `set_breakpoints`, `evaluate`). Incrementing `seq` is
    /// required for DAP request/response matching.
    pub fn request(&mut self, command: &str, args: Option<Value>) -> DapMessage {
        self.seq += 1;
        self.adapter.handle_request(self.seq, command, args)
    }

    /// Validate a DAP `Response` is successful and return its optional body.
    ///
    /// Returns `Err` if the response is a failure response, if the command name
    /// does not match, or if the message is not a `Response` variant at all.
    /// This is the standard way to assert that a request succeeded before
    /// extracting data from the response body.
    pub fn expect_success(&self, msg: &DapMessage, command: &str) -> Result<Option<Value>, String> {
        match msg {
            DapMessage::Response {
                success,
                command: actual,
                body,
                message,
                ..
            } => {
                if actual != command {
                    return Err(format!("expected `{command}` response, got `{actual}`"));
                }
                if !success {
                    return Err(format!(
                        "command `{command}` failed: {}",
                        message.as_deref().unwrap_or("<no message>")
                    ));
                }
                Ok(body.clone())
            }
            other => Err(format!("expected Response for `{command}`, got: {other:?}")),
        }
    }

    /// Drain the event channel until an event with `event_name` arrives.
    ///
    /// Uses the drain-loop pattern from `dap_smoke_e2e.rs` to discard
    /// non-matching events (e.g. `output` events interleaved with `stopped`).
    pub fn drain_until_event(&self, event_name: &str) -> Result<DapMessage, String> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(format!("timeout waiting for event `{event_name}`"));
            }
            let remaining = deadline.saturating_duration_since(now);
            match self.rx.recv_timeout(remaining) {
                Ok(msg) => {
                    if let DapMessage::Event { event, .. } = &msg
                        && event == event_name
                    {
                        return Ok(msg);
                    }
                    // Non-matching event — discard and keep waiting.
                }
                Err(_) => {
                    return Err(format!("channel closed/timeout waiting for `{event_name}`"));
                }
            }
        }
    }
}

/// Returns the test timeout, inflated when running under coverage or profiling.
pub fn workflow_timeout() -> Duration {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some()
        || std::env::var_os("CARGO_LLVM_COV").is_some()
    {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(15)
    }
}

/// Returns `true` when `perl` is on `PATH`.
pub fn perl_available() -> bool {
    std::process::Command::new("perl")
        .arg("--version")
        .output()
        .is_ok()
}
