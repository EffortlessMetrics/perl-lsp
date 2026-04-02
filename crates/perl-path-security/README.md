# perl-path-security

Workspace path validation for editor-facing file operations.

Use this crate when user input needs to become a filesystem path but must stay
inside the active workspace.

## Where it fits

This is a guardrail crate at the protocol edge. It sits between LSP/DAP request
handling and any filesystem access, and it is also used by path-based completion
helpers.

## Key entry points

- `validate_workspace_path(path, workspace_root)` - normalize and bound-check a path
- `WorkspacePathError` - traversal, outside-workspace, and invalid-character errors
- `sanitize_completion_path_input(path)` - reject traversal before path completion
- `split_completion_path_components(path)` - split a safe completion path
- `resolve_completion_base_directory(dir_part)` - locate the directory side of a completion path
- `is_safe_completion_filename(filename)` - reject reserved, malformed, or unsafe names

## Typical use

Use `perl-path-security` before any file open, rename, or completion operation
that starts from user input. If the caller already has a trusted absolute path,
this crate is not the right layer for filesystem discovery.
