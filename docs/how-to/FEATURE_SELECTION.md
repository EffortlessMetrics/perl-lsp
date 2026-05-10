# Feature Selection Guide

This guide explains how to choose and configure the feature profile for perl-lsp, and how the feature governance system controls which LSP capabilities are active at runtime.

## Table of Contents

- [Overview](#overview)
- [Available Profiles](#available-profiles)
  - [ga-lock](#ga-lock-profile)
  - [production](#production-profile)
  - [all](#all-profile)
- [Selecting a Profile at Runtime](#selecting-a-profile-at-runtime)
- [Runtime Feature Gating: Native Formatting](#runtime-feature-gating-native-formatting)
- [Verifying the Active Profile](#verifying-the-active-profile)
- [Feature Catalog and Compliance](#feature-catalog-and-compliance)
- [Per-Feature Reference](#per-feature-reference)
- [Architecture Overview](#architecture-overview)

---

## Overview

perl-lsp uses a **feature governance** system to control which LSP capabilities are enabled. Instead of an all-or-nothing approach, three ordered profiles let you balance compatibility, functionality, and external tooling availability.

The canonical source of truth for all features is `features.toml` at the root of the repository. Every feature entry has:

- `id` - Stable identifier (e.g., `lsp.completion`)
- `area` - Grouping (`text_document`, `workspace`, `window`, `debug`, `protocol`)
- `maturity` - Readiness level (currently all `ga` = generally available)
- `advertised` - Whether the server announces the capability during `initialize`

Profile selection happens at **startup**. The server announces its capabilities to the editor during the LSP `initialize` handshake, so the profile cannot be changed while the server is running.

---

## Available Profiles

### ga-lock Profile

**CLI token:** `ga-lock`, `ga`, or `ga_lock`

The conservative profile. It enables a stable subset of features that excludes `lsp.inline_value` (DAP-time inline variable display). Every other feature available in `production` is also present here, including formatting (no perltidy requirement in this profile).

**When to use:**
- Environments where you need strict backward compatibility
- Situations where inline value display causes issues with your editor client

**Notably enabled vs. production:**
- `lsp.formatting` and `lsp.range_formatting` are on by default (no perltidy check)

**Notably disabled vs. production:**
- `lsp.inline_value` is gated out

---

### production Profile

**CLI token:** `production` or `prod`

The default profile for normal runtime operation. This is what the server uses unless you specify otherwise, or unless the binary was compiled with the `lsp-ga-lock` Cargo feature.

**When to use:**
- Day-to-day Perl development with any supported editor
- Standard deployment without special constraints

**What changes vs. ga-lock:**
- `lsp.inline_value` is enabled (DAP debugging shows inline variable values)
- `lsp.formatting` and `lsp.range_formatting` use the native formatter by default; explicit external Perltidy compatibility mode is opt-in

---

### all Profile

**CLI token:** `all`

Every in-tree capability is enabled. This profile is primarily intended for test matrices, BDD reporting, and snapshot verification. It enables `lsp.formatting` and `lsp.range_formatting` unconditionally regardless of whether Perltidy is installed.

**When to use:**
- Automated test environments where you want full capability coverage
- Generating the complete feature grid JSON for documentation tooling
- Verifying LSP compliance against the full feature catalog

**Caution:** Because every in-tree capability is enabled, this profile can expose experimental surfaces intended for test matrices. Native formatting does not require Perltidy, but explicit external compatibility mode still requires the external tool.

---

## Selecting a Profile at Runtime

Pass `--feature-profile <token>` when starting the server:

```bash
# Start with the production profile (default)
perllsp --stdio

# Explicitly select production
perllsp --stdio --feature-profile production

# Use the conservative ga-lock profile
perllsp --stdio --feature-profile ga-lock

# Enable all features (test/snapshot mode)
perllsp --stdio --feature-profile all
```

Token normalization rules:
- Tokens are **case-insensitive**: `GA-LOCK`, `Prod`, `ALL` all work
- **Whitespace is stripped**: `" production "` resolves correctly
- **Underscores and hyphens are interchangeable**: `ga_lock` == `ga-lock`
- `auto` resolves to the compiled default (either `ga-lock` or `production`)

If you pass an unrecognized token, the server falls back to the compiled default profile and logs a warning.

---

## Editor Configuration

### VS Code

Add to your workspace `.vscode/settings.json` or user settings:

```json
{
  "perl-lsp.featureProfile": "production"
}
```

Or pass it as a server argument in your editor's LSP configuration. For VS Code with a custom server path:

```json
{
  "perl-lsp.serverPath": "/usr/local/bin/perllsp",
  "perl-lsp.serverArgs": ["--feature-profile", "ga-lock"]
}
```

### Neovim (via nvim-lspconfig)

```lua
require('lspconfig').perl_lsp.setup({
  cmd = { 'perllsp', '--stdio', '--feature-profile', 'production' },
})
```

### Helix

In `languages.toml`:

```toml
[[language]]
name = "perl"
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perllsp"
args = ["--stdio", "--feature-profile", "production"]
```

### Emacs (via eglot)

```elisp
(add-to-list 'eglot-server-programs
  '((perl-mode cperl-mode) . ("perllsp" "--stdio" "--feature-profile" "production")))
```

---

## Runtime Feature Gating: Native Formatting

Formatting capabilities (`lsp.formatting`, `lsp.range_formatting`,
`lsp.on_type_formatting`) are backed by the native formatter by default. The
server no longer removes document/range formatting just because Perltidy is not
available on `PATH`; Perltidy only matters when explicit external compatibility
mode is selected.

| Profile | No Perltidy | Perltidy present |
|---------|-------------|------------------|
| `ga-lock` | Formatting enabled (static) | Formatting enabled |
| `production` | Formatting enabled | Formatting enabled |
| `all` | Formatting enabled (static) | Formatting enabled |

In `production` mode without Perltidy the server still starts and advertises
native formatting. If explicit external Perltidy compatibility mode is selected,
formatting errors include guidance for installing the external tool.

To install Perltidy for explicit compatibility mode:

```bash
cpanm Perl::Tidy
# or
cpan Perl::Tidy
```

Verify it is on PATH:

```bash
perltidy --version
```

---

## Verifying the Active Profile

Use the `--info` flag to inspect the server's compiled configuration and feature profile:

```bash
perllsp --info
```

This prints the version, active profile, and the number of advertised features.

To see the full feature catalog as JSON for a given profile:

```bash
perllsp --features-json --feature-profile production
```

The JSON output includes:
- `version` - Catalog version (matches `features.toml` metadata)
- `profile` - Active profile name
- `feature_count` - Total features in catalog
- `advertised_count` - Features advertised for the active profile
- `features` - Array of feature rows with `id`, `area`, `maturity`, `advertised`

---

## Feature Catalog and Compliance

The compliance percentage reported by the server reflects how many features the active profile advertises as a fraction of all `advertised = true` entries in the catalog.

Profile compliance is monotonic: `all >= production >= ga-lock`.

To check compliance from the command line:

```bash
# View all profiles
perllsp --features-json --feature-profile all

# Quick health check (prints version and ok/error)
perllsp --health
```

The `features.toml` file is the canonical definition. The Rust code in `perl-lsp-feature-contracts` generates type-safe bindings from it at compile time.

---

## Per-Feature Reference

The table below summarizes which features each profile enables. "Dynamic" means the feature depends on runtime configuration or an availability check; native formatting is enabled by default and does not require Perltidy.

| Feature ID | ga-lock | production | all |
|---|---|---|---|
| `lsp.completion` | yes | yes | yes |
| `lsp.hover` | yes | yes | yes |
| `lsp.signature_help` | yes | yes | yes |
| `lsp.definition` | yes | yes | yes |
| `lsp.declaration` | yes | yes | yes |
| `lsp.type_definition` | yes | yes | yes |
| `lsp.implementation` | yes | yes | yes |
| `lsp.references` | yes | yes | yes |
| `lsp.document_symbol` | yes | yes | yes |
| `lsp.document_highlight` | yes | yes | yes |
| `lsp.code_action` | yes | yes | yes |
| `lsp.code_lens` | yes | yes | yes |
| `lsp.formatting` | yes | dynamic | yes |
| `lsp.range_formatting` | yes | dynamic | yes |
| `lsp.on_type_formatting` | yes | yes | yes |
| `lsp.rename` | yes | yes | yes |
| `lsp.document_link` | yes | yes | yes |
| `lsp.folding_range` | yes | yes | yes |
| `lsp.selection_range` | yes | yes | yes |
| `lsp.semantic_tokens` | yes | yes | yes |
| `lsp.inlay_hint` | yes | yes | yes |
| `lsp.call_hierarchy` | yes | yes | yes |
| `lsp.type_hierarchy` | yes | yes | yes |
| `lsp.inline_value` | **no** | yes | yes |
| `lsp.inline_completion` | yes | yes | yes |
| `lsp.pull_diagnostics` | yes | yes | yes |
| `lsp.workspace_symbol` | yes | yes | yes |
| `lsp.workspace_symbol_resolve` | yes | yes | yes |
| `lsp.linked_editing_range` | yes | yes | yes |
| `lsp.document_color` | yes | yes | yes |
| `lsp.moniker` | yes | yes | yes |
| `lsp.notebook_document_sync` | yes | yes | yes |

DAP features (`dap.*`) are not gated by profile selection and are always present when the DAP server binary is used.

---

## Architecture Overview

The feature governance system is split across several microcrates to keep concerns separate:

| Crate | Responsibility |
|---|---|
| `perl-lsp-feature-ids` | Raw `&'static str` constants for every feature ID |
| `perl-lsp-feature-contracts` | `FeatureProfileKind` enum, `FeatureProfileSpec` metadata, alias table |
| `perl-lsp-feature-flags` | `BuildFlags` and `AdvertisedFeatures` structs; static profile shapes |
| `perl-lsp-feature-profile` | Token normalization (trimming, case, underscore/hyphen) |
| `perl-lsp-feature-policy` | `FeatureProfile` runtime enum; bridges profile to flags and advertised IDs |
| `perl-lsp-feature-profile-cli` | CLI argument parsing; structured error with supported-token list |
| `perl-lsp-feature-grid` | JSON grid rendering for BDD and tooling output |
| `perl-lsp-feature-governance` | Facade re-exporting all of the above under one stable API |

The data flow at startup:

```
CLI --feature-profile <token>
    -> parse_feature_profile_arg()          [perl-lsp-feature-profile-cli]
    -> FeatureProfile::from_kind()          [perl-lsp-feature-policy]
    -> FeatureProfile::runtime_flags()      [perl-lsp-feature-policy]
    -> BuildFlags                           [perl-lsp-feature-flags]
    -> AdvertisedFeatures                   [perl-lsp-feature-flags]
    -> ServerCapabilities in initialize     [perl-lsp runtime]
```

The `features.toml` file feeds into `perl-lsp-feature-contracts` at build time via a build script that generates Rust types, keeping the catalog as the single source of truth for both runtime behavior and documentation tooling.

---

## See Also

- [features.toml](../../features.toml) - Complete feature catalog
- [PERFORMANCE_TUNING.md](PERFORMANCE_TUNING.md) - Workspace and hardware tuning
- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Editor-specific setup
- [INSTALLATION.md](INSTALLATION.md) - Installation and setup
- [CURRENT_STATUS.md](../project/CURRENT_STATUS.md) - Current feature coverage metrics
