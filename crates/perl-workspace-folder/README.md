# perl-workspace-folder

Single responsibility crate for converting workspace folder inputs into local
filesystem paths.

## Scope

- Parse plain path strings as `PathBuf`s.
- Resolve `file://` URIs to local paths when possible.
- Fall back to stripping `file://` when URI conversion is unavailable.

## API

- `workspace_folder_to_path(workspace_folder: &str) -> PathBuf`
