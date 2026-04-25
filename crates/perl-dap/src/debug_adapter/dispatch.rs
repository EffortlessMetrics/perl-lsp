//! Request dispatching: handle_request, dispatch_request, response_succeeded_for_command.

use super::*;

impl DebugAdapter {
    const SUPPORTED_COMMANDS: [&str; 37] = [
        "initialize",
        "launch",
        "attach",
        "disconnect",
        "terminate",
        "setBreakpoints",
        "setFunctionBreakpoints",
        "setExceptionBreakpoints",
        "configurationDone",
        "threads",
        "stackTrace",
        "scopes",
        "variables",
        "setVariable",
        "continue",
        "next",
        "stepIn",
        "stepOut",
        "pause",
        "evaluate",
        "inlineValues",
        "breakpointLocations",
        "source",
        "loadedSources",
        "modules",
        "completions",
        "exceptionInfo",
        "restart",
        "setExpression",
        "dataBreakpointInfo",
        "setDataBreakpoints",
        "cancel",
        "stepInTargets",
        "gotoTargets",
        "goto",
        "restartFrame",
        "terminateThreads",
    ];

    pub fn handle_request(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        tracing::debug!(command, arguments = ?arguments, "DAP request");

        let response = self.dispatch_request(request_seq, command, arguments);

        // Preserve existing direct-call behavior for tests and in-memory usage.
        if command == "initialize" && Self::response_succeeded_for_command(&response, "initialize")
        {
            self.send_event("initialized", None);
        }

        response
    }

    /// Handle a DAP request (mock version for testing)
    pub fn handle_request_mock(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        tracing::debug!(command, arguments = ?arguments, "DAP request (mock)");

        let response = self.dispatch_request(request_seq, command, arguments);
        if command == "initialize" && Self::response_succeeded_for_command(&response, "initialize")
        {
            self.send_event("initialized", None);
        }
        response
    }

    pub(super) fn dispatch_request(
        &mut self,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> DapMessage {
        let seq = self.next_seq();

        match command {
            "initialize" => self.handle_initialize(seq, request_seq, arguments),
            "launch" => self.handle_launch(seq, request_seq, arguments),
            "attach" => self.handle_attach(seq, request_seq, arguments),
            "disconnect" => self.handle_disconnect(seq, request_seq, arguments),
            "terminate" => self.handle_terminate(seq, request_seq, arguments),
            "setBreakpoints" => self.handle_set_breakpoints(seq, request_seq, arguments),
            "setFunctionBreakpoints" => {
                self.handle_set_function_breakpoints(seq, request_seq, arguments)
            }
            "setExceptionBreakpoints" => {
                self.handle_set_exception_breakpoints(seq, request_seq, arguments)
            }
            "configurationDone" => self.handle_configuration_done(seq, request_seq),
            "threads" => self.handle_threads(seq, request_seq),
            "stackTrace" => self.handle_stack_trace(seq, request_seq, arguments),
            "scopes" => self.handle_scopes(seq, request_seq, arguments),
            "variables" => self.handle_variables(seq, request_seq, arguments),
            "setVariable" => self.handle_set_variable(seq, request_seq, arguments),
            "continue" => self.handle_continue(seq, request_seq, arguments),
            "next" => self.handle_next(seq, request_seq, arguments),
            "stepIn" => self.handle_step_in(seq, request_seq, arguments),
            "stepOut" => self.handle_step_out(seq, request_seq, arguments),
            "pause" => self.handle_pause(seq, request_seq, arguments),
            "evaluate" => self.handle_evaluate(seq, request_seq, arguments),
            "inlineValues" => self.handle_inline_values(seq, request_seq, arguments),
            "breakpointLocations" => self.handle_breakpoint_locations(seq, request_seq, arguments),
            "source" => self.handle_source(seq, request_seq, arguments),
            "loadedSources" => self.handle_loaded_sources(seq, request_seq, arguments),
            "modules" => self.handle_modules(seq, request_seq, arguments),
            "completions" => self.handle_completions(seq, request_seq, arguments),
            "exceptionInfo" => self.handle_exception_info(seq, request_seq, arguments),
            "restart" => self.handle_restart(seq, request_seq, arguments),
            "setExpression" => self.handle_set_expression(seq, request_seq, arguments),
            "dataBreakpointInfo" => self.handle_data_breakpoint_info(seq, request_seq, arguments),
            "setDataBreakpoints" => self.handle_set_data_breakpoints(seq, request_seq, arguments),
            "cancel" => self.handle_cancel(seq, request_seq, arguments),
            "stepInTargets" => self.handle_step_in_targets(seq, request_seq, arguments),
            "gotoTargets" => self.handle_goto_targets(seq, request_seq, arguments),
            "goto" => self.handle_goto(seq, request_seq, arguments),
            "restartFrame" => self.handle_restart_frame(seq, request_seq, arguments),
            "terminateThreads" => self.handle_terminate_threads(seq, request_seq, arguments),
            _ => DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: command.to_string(),
                body: None,
                message: Some(Self::unknown_command_message(command)),
            },
        }
    }

    fn unknown_command_message(command: &str) -> String {
        if let Some(suggestion) = Self::suggested_command(command) {
            format!("Unknown command: {command}. Did you mean '{suggestion}'?")
        } else {
            format!("Unknown command: {command}")
        }
    }

    fn suggested_command(command: &str) -> Option<&'static str> {
        Self::SUPPORTED_COMMANDS.iter().copied().find(|known| known.eq_ignore_ascii_case(command))
    }

    pub(super) fn response_succeeded_for_command(
        response: &DapMessage,
        expected_command: &str,
    ) -> bool {
        matches!(
            response,
            DapMessage::Response {
                success: true,
                command,
                ..
            } if command == expected_command
        )
    }
}
