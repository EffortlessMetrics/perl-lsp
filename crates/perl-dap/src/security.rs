//! Compatibility module re-exporting DAP security validation helpers.
//!
//! This module delegates implementation to the dedicated
//! `perl-dap-security` microcrate to keep `perl-dap` focused on protocol and
//! adapter orchestration concerns.

pub use perl_dap_security::{
    DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression,
    validate_path, validate_timeout,
};
