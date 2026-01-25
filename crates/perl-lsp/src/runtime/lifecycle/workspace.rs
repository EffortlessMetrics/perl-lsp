//! Workspace management
//!
//! Handles workspace folders and root URI/path management.

use super::super::*;
use url::Url;

impl LspServer {
    /// Set the root path from the root URI during initialization
    pub(crate) fn set_root_uri(&self, root_uri: &str) {
        let root_path = Url::parse(root_uri).ok().and_then(|u| u.to_file_path().ok());
        *self.root_path.lock() = root_path;
    }
}
