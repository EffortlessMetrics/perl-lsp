//! Backward-compatible re-export of the DAP configuration microcrate.

pub use perl_dap_config::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
