# tree-sitter-perl-c Roadmap

> **Note:** This is the component-specific roadmap for `tree-sitter-perl-c`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose

Conventional tree-sitter Perl grammar binding (C FFI), maintained for compatibility
and comparison against the native v3 parser (`perl-parser`).

## Current Status (v0.12.2)

- **Status:** Published — stable maintenance
- **Role:** Compatibility baseline and benchmarking reference. Active parser
  development happens in `perl-parser` (native v3 Rust).
- **Integration:** Part of the `perl-lsp` workspace, on the publish allowlist.

## Stability

This crate tracks the upstream [tree-sitter-perl] C grammar. The public API
(`language()`, `try_create_parser()`, `parse_perl_code()`, etc.) is stable.
Breaking changes will follow semver.

**Known limitations vs. upstream grammar:**

- The vendored `c-src/` is a periodic snapshot; it may lag behind upstream by
  one or two grammar releases. File an issue to request a snapshot update.
- The crate does not expose tree-sitter query helpers — use the
  `tree-sitter` crate directly with the `language()` return value.

## Planned Work

### Maintenance (ongoing)

- Periodic snapshot updates when upstream tree-sitter-perl releases new grammar versions.
- Keep `tree-sitter` runtime dependency in sync with the workspace.

### Not planned

- New high-level parse APIs (out of scope — use `perl-parser` for that).
- Grammar extensions or bug fixes (those belong upstream in tree-sitter-perl).

## Internal Dependencies

- Aligns with project-wide capability goals defined in `features.toml`.
- Benchmarked against `perl-parser` via `just benchmarks`.

[tree-sitter-perl]: https://github.com/tree-sitter-perl/tree-sitter-perl

<!-- Last Updated: 2026-04-07 -->
