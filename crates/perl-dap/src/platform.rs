//! Compatibility module re-exporting cross-platform DAP helpers.
//!
//! This module now delegates implementation to dedicated microcrates to keep
//! `perl-dap` focused on protocol and adapter orchestration concerns.

pub use perl_dap_command_args::format_command_args;
pub use perl_dap_platform::{
    detect_perlbrew_perl, detect_plenv_perl, normalize_path, resolve_perl_path,
    resolve_perl_path_with_toolchain, setup_environment,
};
