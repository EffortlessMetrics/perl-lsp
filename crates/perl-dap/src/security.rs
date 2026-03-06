//! Security validation helpers re-exported from `perl-dap-security`.

pub use perl_dap_security::{
    DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression,
    validate_path, validate_timeout,
};
