//! Unified public API surface for `perl-workspace`.
//!
//! Explicit re-exports of the enumeration satellite public APIs.
//! No wildcards — required because type name conflicts exist between
//! `monitoring` and `state_machine` modules.

// Discovery public API
pub use crate::discovery::{
    DiscoveryMethod, DiscoveryResult, discover_perl_files, is_perl_discovery_path,
};

// Folder public API
pub use crate::folder::{
    WorkspaceFolderChange, extract_workspace_folder_change, extract_workspace_folder_uris,
    root_path_to_file_uri, workspace_folder_to_path,
};

// Ignore public API
pub use crate::ignore::{is_skipped_dir_name, path_contains_skipped_component};
