//! Explicit public API re-exports for Wave H collapsed modules.
//!
//! This module provides named re-exports from all 11 collapsed satellite modules.
//! No wildcard re-exports — all items are explicitly named.

// Re-exports from breakpoint module
pub use crate::breakpoint::{
    AstBreakpointValidator, BreakpointError, BreakpointValidation, BreakpointValidator,
    SearchDirection, ValidationReason, find_nearest_valid_line,
};

// Re-exports from eval module
pub use crate::eval::{DANGEROUS_OPERATIONS, SafeEvaluator, ValidationError, ValidationResult};

// Re-exports from config module
pub use crate::config::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};

// Re-exports from command_args module
pub use crate::command_args::format_command_args;

// Re-exports from platform module
pub use crate::platform::{
    PerlInterpreterResult, detect_perlbrew_perl, detect_plenv_perl, find_perl_interpreter,
    normalize_path, resolve_perl_path, resolve_perl_path_with_toolchain, setup_environment,
};

// Re-exports from stack module
pub use crate::stack::{
    FrameCategory, FrameClassifier, PerlFrameClassifier, PerlStackParser, StackParseError,
    filter_user_visible_frames, is_internal_frame, is_internal_frame_name_and_path,
};

// Re-exports from types module
pub use crate::types::{
    Source as TypesSource, StackFrame as TypesStackFrame, Variable as TypesVariable,
};

// Re-exports from value module
pub use crate::value::PerlValue;

// Re-exports from variables module
pub use crate::variables::{
    PerlVariableRenderer, RenderedVariable, VariableParseError, VariableParser,
    VariablePresentationHint, VariableRenderer,
};

// Re-exports from security module
pub use crate::security::{
    DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression,
    validate_path, validate_timeout,
};
