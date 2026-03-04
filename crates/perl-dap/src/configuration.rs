//! Compatibility module re-exporting DAP launch/attach configuration helpers.
//!
//! This module delegates implementation to the dedicated
//! `perl-dap-configuration` microcrate to keep `perl-dap` focused on protocol
//! and adapter orchestration concerns.

pub use perl_dap_configuration::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
