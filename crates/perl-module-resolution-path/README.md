# perl-module-resolution-path

Single-purpose workspace-aware resolver for Perl module names to filesystem paths.

## Scope

- Convert Perl module names to relative module paths
- Check each include path under a workspace root with traversal protection
- Return a deterministic fallback path under `root/lib` when no include path matches

This crate intentionally owns only filesystem path resolution; URI conversion is
handled by `perl-module-resolution-uri`.

