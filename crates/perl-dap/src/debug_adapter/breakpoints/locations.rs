use super::*;

impl DebugAdapter {
    pub(in crate::debug_adapter) fn handle_breakpoint_locations(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: BreakpointLocationsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "breakpointLocations".to_string(),
                        body: None,
                        message: Some("Missing or invalid arguments".to_string()),
                    };
                }
            };

        let source_path = match args.source.path {
            Some(ref p) => p.clone(),
            None => {
                let body = BreakpointLocationsResponseBody { breakpoints: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "breakpointLocations".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                };
            }
        };

        // Validate path against workspace root to prevent path traversal
        let validated_path = match self.validate_source_path(&source_path) {
            Ok(p) => p,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "breakpointLocations".to_string(),
                    body: None,
                    message: Some(e),
                };
            }
        };

        let content = match std::fs::read_to_string(&validated_path) {
            Ok(c) => c,
            Err(_) => {
                let body = BreakpointLocationsResponseBody { breakpoints: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "breakpointLocations".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                };
            }
        };

        let mut breakpoints = Vec::new();
        let end_line = args.end_line.unwrap_or(args.line);

        if let Ok(validator) = AstBreakpointValidator::new(&content) {
            for line in args.line..=end_line {
                if self.cancel_requested.load(Ordering::Acquire) {
                    self.cancel_requested.store(false, Ordering::Release);
                    break;
                }
                if validator.is_executable_line(line) {
                    breakpoints.push(BreakpointLocation {
                        line,
                        column: None,
                        end_line: None,
                        end_column: None,
                    });
                }
            }
        }

        let body = BreakpointLocationsResponseBody { breakpoints };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "breakpointLocations".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }
}
