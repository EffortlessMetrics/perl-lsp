use crate::feature_catalog::has_feature as catalog_has_feature;
use crate::inline_values::collect_inline_values;
use crate::protocol::{
    Breakpoint, Capabilities, ExceptionBreakpointFilter, InitializeRequestArguments,
    InlineValuesArguments, InlineValuesResponseBody, Request, SetBreakpointsArguments,
    SetBreakpointsResponseBody,
};
use anyhow::{Context, Result};
use serde_json::Value;

use super::state::DispatcherState;

pub(crate) fn handle_initialize(request: &Request) -> Result<Value> {
    let _args: InitializeRequestArguments = request
        .arguments
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .context("Failed to parse initialize arguments")?
        .unwrap_or(InitializeRequestArguments {
            client_id: None,
            client_name: None,
            adapter_id: "perl-rs".to_string(),
            locale: None,
            lines_start_at1: Some(true),
            columns_start_at1: Some(true),
            path_format: None,
        });

    let supports_breakpoints = catalog_has_feature("dap.breakpoints.basic");
    let supports_hit_conditions = catalog_has_feature("dap.breakpoints.hit_condition");
    let supports_log_points = catalog_has_feature("dap.breakpoints.logpoints");
    let supports_exceptions = catalog_has_feature("dap.exceptions.die");
    let supports_inline_values = catalog_has_feature("dap.inline_values");

    let capabilities = Capabilities {
        supports_configuration_done_request: Some(true),
        supports_evaluate_for_hovers: Some(true),
        supports_conditional_breakpoints: Some(supports_breakpoints),
        supports_hit_conditional_breakpoints: Some(supports_hit_conditions),
        supports_log_points: Some(supports_log_points),
        supports_exception_options: Some(supports_exceptions),
        supports_exception_filter_options: Some(supports_exceptions),
        supports_terminate_request: Some(true),
        supports_inline_values: Some(supports_inline_values),
        supports_function_breakpoints: Some(supports_breakpoints),
        supports_set_variable: Some(true),
        supports_value_formatting_options: Some(false),
        support_terminate_debuggee: Some(true),
        supports_step_back: Some(false),
        supports_data_breakpoints: Some(false),
        exception_breakpoint_filters: if supports_exceptions {
            Some(vec![ExceptionBreakpointFilter {
                filter: "die".to_string(),
                label: "Break on die/croak".to_string(),
                default: Some(false),
            }])
        } else {
            None
        },
    };

    serde_json::to_value(&capabilities).context("Failed to serialize capabilities")
}

pub(crate) fn handle_configuration_done(state: &DispatcherState) -> Result<Value> {
    let initialized = state.initialized.lock().unwrap_or_else(|e| e.into_inner());
    if !*initialized {
        anyhow::bail!("configurationDone received before initialized");
    }
    Ok(serde_json::Value::Null)
}

pub(crate) fn handle_set_breakpoints(state: &DispatcherState, request: &Request) -> Result<Value> {
    let args: SetBreakpointsArguments = request
        .arguments
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing arguments for setBreakpoints"))
        .and_then(|v| serde_json::from_value(v.clone()).context("Invalid arguments"))?;

    let breakpoints: Vec<Breakpoint> = state.breakpoint_store.set_breakpoints(&args);
    let body = SetBreakpointsResponseBody { breakpoints };

    serde_json::to_value(&body).context("Failed to serialize setBreakpoints response")
}

pub(crate) fn handle_inline_values(request: &Request) -> Result<Value> {
    let args: InlineValuesArguments = request
        .arguments
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing arguments for inlineValues"))
        .and_then(|v| serde_json::from_value(v.clone()).context("Invalid arguments"))?;

    let source_path =
        args.source.path.ok_or_else(|| anyhow::anyhow!("inlineValues requires source.path"))?;

    if args.start_line <= 0 || args.end_line <= 0 {
        anyhow::bail!("inlineValues requires positive startLine/endLine");
    }

    let start_line = args.start_line.min(args.end_line);
    let end_line = args.end_line.max(args.start_line);
    let content = std::fs::read_to_string(&source_path)
        .with_context(|| format!("Failed to read source file: {}", source_path))?;

    let inline_values = collect_inline_values(&content, start_line, end_line);
    let body = InlineValuesResponseBody { inline_values };

    serde_json::to_value(&body).context("Failed to serialize inlineValues response")
}
