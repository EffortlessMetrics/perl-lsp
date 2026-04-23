//! Backward-compatible re-export of the DAP configuration module.

pub use crate::config::{
    AttachConfiguration, LaunchConfiguration, create_attach_json_snippet,
    create_launch_json_snippet,
};
