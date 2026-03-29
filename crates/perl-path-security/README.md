# perl-path-security

Workspace-bound path validation utilities for the Perl LSP/DAP ecosystem.

## When to use this crate

Use `perl-path-security` when you need to validate a user-supplied path before
touching the filesystem.

It focuses on the common security boundary problems:

- rejecting parent-directory traversal
- keeping paths inside a workspace root
- rejecting null-byte and control-character injection
- normalizing safe completion inputs for downstream completion or file-finding code

## Public surface

- Validates user-supplied paths stay inside a workspace root
- Prevents parent-directory traversal (e.g. `../../`)
- Rejects null-byte/control-character path injection
- Normalizes safe relative paths for downstream operations

## API

- `validate_workspace_path(path, workspace_root)`
- `WorkspacePathError`

## Workspace role

This is a small security boundary crate used by `perl-lsp`, `perl-dap`, and
other workspace tools that accept paths from editors or test harnesses.
