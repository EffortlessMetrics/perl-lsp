# perl-path-security

Workspace-bound path validation utilities for the Perl LSP/DAP ecosystem.

## Scope

- Validates user-supplied paths stay inside a workspace root
- Prevents parent-directory traversal (e.g. `../../`)
- Rejects null-byte/control-character path injection
- Normalizes safe relative paths for downstream operations

## API

- `validate_workspace_path(path, workspace_root)`
- `WorkspacePathError`
