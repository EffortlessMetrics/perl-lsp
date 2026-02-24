# perl-module-resolution

Deterministic, secure Perl module resolution helpers for workspace-aware tools.

## Scope

- Resolve module names (for example, `Foo::Bar`) to filesystem paths
- Resolve module names to `file://` URIs across open documents, workspace folders, and optional system `@INC`
- Enforce workspace path validation to prevent traversal via include paths
- Apply timeout-aware resolution for responsive editor workflows

## API

- `resolve_module_path(root, module_name, include_paths)`
- `resolve_module_uri(module_name, open_document_uris, workspace_folders, include_paths, use_system_inc, system_inc, timeout)`
- `ModuleUriResolution`
