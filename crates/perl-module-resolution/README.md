# perl-module-resolution

Deterministic, secure Perl module resolution helpers for workspace-aware tools.

## When to use this crate

Use `perl-module-resolution` when you need to turn Perl module names into
filesystem or URI targets in a workspace-aware tool.

It is the right crate for:

- `Foo::Bar` to path/URI resolution
- `use lib`-aware include-path discovery
- resolution that respects workspace folders, open documents, and optional system `@INC`
- a separate path-validation boundary for secure tooling

## Public surface

- Resolve module names (for example, `Foo::Bar`) to filesystem paths
- Resolve module names to `file://` URIs across open documents, workspace folders, and optional system `@INC`
- Enforce workspace path validation to prevent traversal via include paths (via
  `perl-module-resolution-path`)
- Apply timeout-aware resolution for responsive editor workflows

## API

- `resolve_module_path(root, module_name, include_paths)`
- `resolve_module_uri(module_name, open_document_uris, workspace_folders, include_paths, use_system_inc, system_inc, timeout)`
- `ModuleUriResolution`

## Workspace role

This crate is a workspace utility used by editor and language-server features.
It is useful when you need module lookup behavior without reimplementing path
rules in each consumer.
