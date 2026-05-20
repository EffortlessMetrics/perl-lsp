use super::*;

impl DebugAdapter {
    /// Handle setExceptionBreakpoints request.
    ///
    /// Supports `die`/uncaught exception breaks via output classification in the
    /// debugger reader thread.
    pub(in crate::debug_adapter) fn handle_set_exception_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let mut break_on_die = false;
        let mut break_on_warn = false;
        let supports_die = catalog_has_feature("dap.exceptions.die");
        let supports_warn = catalog_has_feature("dap.exceptions.warn");

        if let Some(args) = arguments
            .and_then(|v| serde_json::from_value::<SetExceptionBreakpointsArguments>(v).ok())
        {
            let matches_filter = |id: &str| -> (bool, bool) {
                let all = id.eq_ignore_ascii_case("all");
                let die = supports_die && (id.eq_ignore_ascii_case("die") || all);
                let warn = supports_warn && (id.eq_ignore_ascii_case("warn") || all);
                (die, warn)
            };

            for filter in &args.filters {
                let (die, warn) = matches_filter(filter);
                break_on_die |= die;
                break_on_warn |= warn;
            }

            if let Some(filter_options) = args.filter_options {
                for entry in &filter_options {
                    let (die, warn) = matches_filter(&entry.filter_id);
                    break_on_die |= die;
                    break_on_warn |= warn;
                }
            }
        }

        if let Ok(mut guard) = self.exception_break_on_die.lock() {
            *guard = break_on_die;
        }
        if let Ok(mut guard) = self.exception_break_on_warn.lock() {
            *guard = break_on_warn;
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setExceptionBreakpoints".to_string(),
            body: Some(json!({ "breakpoints": [] })),
            message: None,
        }
    }
}
