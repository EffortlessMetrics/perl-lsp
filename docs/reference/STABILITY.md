# API Stability and Version Policy

**MSRV:** 1.92 • **Edition:** 2024 • **Status:** Public alpha

This document describes the stability posture for the current public-alpha line. The current release line is `v0.12.x`; stronger compatibility guarantees are still targeted for the `v0.15.0` stability-contract milestone.

## Current Alpha Stance

What public alpha means here:

- APIs and behaviors are usable today but can still change between minor releases
- Advertised protocol behavior is tracked carefully, but the formal compatibility contract is not locked yet
- Packaging and distribution surfaces can still change while the alpha line is being hardened
- Documentation is being aligned with the actual shipped posture rather than treated as frozen

## What We Ship Today

| Distribution | Format | Support level |
| --- | --- | --- |
| GitHub Releases | Tagged source and binary artifacts | Alpha |
| crates.io | Published crates | Alpha |
| VS Code extension | Marketplace / Open VSX distribution | Alpha |
| Source builds | Git checkout + Cargo | Alpha |

Availability can vary by release. Check the release notes and repo documentation for the exact surface shipped in a given version.

## Public Crate Line

These crates define the user-facing alpha line:

| Crate | Current line | Purpose | Stability posture |
| --- | --- | --- | --- |
| `perl-parser` | `0.11.x` | Parser and AST-facing library | Evolving |
| `perl-lexer` | `0.11.x` | Tokenizer | Evolving |
| `perl-lsp` | `0.11.x` | LSP server binary | Evolving |
| `perl-corpus` | `0.11.x` | Corpus and fixtures | Evolving |
| `perl-dap` | `0.11.x` | Debug adapter | Preview / evolving |
| `perl-parser-pest` | `0.11.x` | Legacy parser path | Maintenance only |

## Versioning Policy

### Minor releases (`0.Y.0`)

Breaking changes are still allowed in minor public-alpha releases. We aim to document those changes clearly, but full multi-release deprecation cycles are not promised before `v0.15.0`.

### Patch releases (`0.Y.Z`)

Patch releases are intended for fixes, hardening, and documentation updates that do not deliberately reshape the public surface.

## Support Expectations

During the alpha line, the project aims for:

1. Stronger parser and workspace correctness on real-world Perl
2. A stable enough editor experience for early adopters
3. Clearer receipts and project-health documentation
4. Continued hardening of security and validation boundaries

## Toward the Stability Contract (`v0.15.0`)

The `v0.15.0` milestone is where the project intends to tighten the contract around:

1. Public API compatibility expectations
2. Advertised protocol behavior
3. Deprecation policy and migration guidance
4. Platform support commitments

## Verification

```bash
nix develop -c just ci-gate
```

For current receipts and project posture, see:

- [../project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md)
- [../project/ROADMAP.md](../project/ROADMAP.md)
