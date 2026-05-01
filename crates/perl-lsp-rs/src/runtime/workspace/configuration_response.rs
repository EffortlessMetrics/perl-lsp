use crate::runtime::workspace_folder::WorkspaceFolderState;
use serde_json::Value;

pub(super) fn apply_workspace_configuration_results(
    folders: &mut [WorkspaceFolderState],
    folder_uris: &[String],
    includes_global_item: bool,
    results: &[Value],
    request_id: i64,
) {
    let global_settings = if includes_global_item { results.first() } else { None };
    let folder_results_start = usize::from(includes_global_item);

    for (idx, folder_uri) in folder_uris.iter().enumerate() {
        let Some(folder) = folders.iter_mut().find(|folder| &folder.uri == folder_uri) else {
            continue;
        };

        let mut effective_config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        if let Some(project_config) = &folder.project_config {
            project_config.apply_to_workspace_config(&mut effective_config);
        }

        if let Some(global_settings) = global_settings {
            effective_config.update_from_value(global_settings);
        }

        if let Some(perl_settings) = results.get(folder_results_start + idx) {
            effective_config.update_from_value(perl_settings);
        } else {
            tracing::warn!(
                request_id,
                folder_uri = %folder_uri,
                "workspace/configuration response missing folder item; using TOML/default config for folder"
            );
        }

        folder.effective_workspace_config = effective_config;
    }
}
