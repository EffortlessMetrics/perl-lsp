//! Workspace request handlers
//!
//! Wraps workspace/* LSP requests.

use super::super::*;

impl LspServer {
    pub(super) fn handle_workspace_symbols_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        #[cfg(feature = "workspace")]
        let result = self.handle_workspace_symbols_v2(params);
        #[cfg(not(feature = "workspace"))]
        let result = self.handle_workspace_symbols(params);
        result
    }

    pub(super) fn handle_workspace_symbol_resolve_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_workspace_symbol_resolve(params)
    }

    pub(super) fn handle_configuration_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_configuration(params)
    }

    pub(super) fn handle_did_change_watched_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_did_change_watched_files(params)
    }

    pub(super) fn handle_did_change_workspace_folders_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        match self.handle_did_change_workspace_folders(params) {
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub(super) fn handle_did_change_configuration_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_did_change_configuration(params);
        Ok(None) // Notification, no response
    }

    pub(super) fn handle_progress_cancel_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_progress_cancel(params);
        Ok(None) // Notification, no response
    }

    pub(super) fn handle_will_rename_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_will_rename_files(params)
    }

    pub(super) fn handle_did_rename_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_did_rename_files(params)
    }

    pub(super) fn handle_will_delete_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_will_delete_files(params)
    }

    pub(super) fn handle_did_delete_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_did_delete_files(params)
    }

    pub(super) fn handle_will_create_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_will_create_files(params)
    }

    pub(super) fn handle_did_create_files_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_did_create_files(params)
    }

    pub(super) fn handle_apply_edit_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_apply_edit(params)
    }

    pub(super) fn handle_text_document_content_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_text_document_content(params)
    }
}
