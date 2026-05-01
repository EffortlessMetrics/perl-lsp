use regex::Regex;
use std::sync::OnceLock;

/// Compiled regex patterns for debugger output parsing
pub(super) static CONTEXT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static PROMPT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static STACK_FRAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
#[allow(dead_code)] // Reserved for future variable parsing enhancements
pub(super) static VARIABLE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static ERROR_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static EXCEPTION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static DANGEROUS_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static REGEX_MUTATION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static ASSIGNMENT_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static DEREF_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static GLOB_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static ANSI_ESCAPE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static SET_VARIABLE_NAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static FUNCTION_BREAKPOINT_NAME_RE: OnceLock<Result<Regex, regex::Error>> =
    OnceLock::new();
pub(super) static WARNING_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
pub(super) static INC_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
