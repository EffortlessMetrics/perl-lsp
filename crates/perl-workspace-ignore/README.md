# perl-workspace-ignore

Shared workspace noise-directory rules used by workspace discovery and runtime filtering.

This crate owns one responsibility: deciding whether a path component is part of the
canonical workspace ignore set (`.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`).
