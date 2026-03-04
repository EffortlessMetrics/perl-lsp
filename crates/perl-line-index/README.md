# perl-line-index

Line indexing primitives used throughout the perl-lsp workspace.

This crate provides:

- `LineStartsCache`: non-owning line starts cache for `&str` and `ropey::Rope`
- `LineIndex`: owning line index with UTF-16 aware position conversion
