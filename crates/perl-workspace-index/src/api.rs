//! Unified public API surface for `perl-workspace-index`.
//!
//! This module defines the crate's stable "import once" entry-point for callers
//! that only need discovery, folder parsing, and ignore-policy helpers.
//! Re-exports are intentionally explicit (no glob exports) so generated docs stay
//! navigable and type conflicts are avoided.
//!
//! # Why a dedicated API module?
//!
//! `monitoring` and `state_machine` both expose similarly named types
//! (`IndexStateKind`, `DegradationReason`, etc.). Re-exporting everything from
//! crate root would create ambiguous imports for downstream crates. By keeping a
//! curated public surface in `api`, consumers can opt into just the common
//! workspace-bootstrap utilities without pulling in overlapping lifecycle types.
//!
//! # Typical usage
//!
//! ```rust
//! use perl_workspace_index::api::{
//!     discover_perl_files, extract_workspace_folder_uris, is_skipped_dir_name,
//! };
//!
//! let _folders = extract_workspace_folder_uris(&[]);
//! let _is_noise = is_skipped_dir_name("target");
//! let _result = discover_perl_files(std::path::Path::new("."));
//! ```

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
