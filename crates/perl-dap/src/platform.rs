//! Compatibility module re-exporting cross-platform DAP helpers.
//!
//! This module now delegates implementation to the dedicated
//! `perl-dap-platform` microcrate to keep `perl-dap` focused on protocol and
//! adapter orchestration concerns.

pub use perl_dap_platform::{
    format_command_args, normalize_path, resolve_perl_path, setup_environment,
};
