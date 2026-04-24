# perl-pragma Roadmap

> **Note:** This is the component-specific roadmap for `perl-pragma`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Perl pragma extraction and lexical-scope analysis primitives.

## Shipped Surface (as of 2026-04-24)

- Tracks `use`/`no` state for:
  - `strict` (full + category selective)
  - `warnings` (global plus category-level disables)
  - `utf8`
  - `encoding`
  - `locale`
  - `feature` (named features, `:all`, and version bundles like `:5.36`)
  - `builtin` lexical import lists
- Parses Perl version pragmas and applies implied semantics:
  - strict from `use v5.12+`
  - warnings from `use v5.35+`
  - version-implied feature bundles via `features_enabled_by_version`
- Applies lexical restoration across scoped forms, including standard blocks,
  phase blocks, `eval { ... }`, and braced package blocks.
- Includes crate-local tests in `tests/` covering behavior scenarios and broad
  API edge cases.

## Hardening / Remaining Work

- Expand conformance coverage for nuanced Perl edge cases (especially mixed
  conditional pragmas and uncommon argument forms).
- Add targeted regression fixtures for parser-shape variations that affect
  pragma argument normalization.
- Continue tightening API documentation and examples for downstream consumers.
- Gather adopter feedback before declaring a stabilized public contract.

## Stability Target

### v0.15.0 Stability Contract

- Lock down public API for semantic versioning.
- Guarantee stability across supported platforms.

## Internal Dependencies

- Aligns with project-wide capability goals defined in `features.toml`.

<!-- Last Updated: 2026-04-24 -->
