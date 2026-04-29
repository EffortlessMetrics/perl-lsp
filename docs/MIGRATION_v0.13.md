# Migration Guide — v0.13.0

This guide is for downstream users of perl-lsp crates who are upgrading to v0.13.0.

> **Note (2026-04-16):** This file was previously named `MIGRATION_v0.14.md`. The target release
> for the microcrate collapse clean break was corrected to v0.13.0. See ADR-0041 Amendment 2.

## What's changing in v0.13.0

v0.13.0 is a **clean break** release. The published crate count drops from 132 to **30**.
Approximately 100 product-internal microcrates stop being published. Their code moves
into subfolder modules inside the owning published crate. There are no bridge crates
or re-export shims — old crate names will no longer appear on crates.io after this release.

This is a deliberate one-time cost to eliminate a permanent operational burden. See
[ADR-0041](adr/0041-microcrate-collapse.md) for the full rationale.

## If you depend on perl-lsp-rs, perl-parser, perl-dap, or perllsp

**No change.** These four product crates survive the collapse unchanged and continue to be
published under the same names. Their public APIs are not affected by the internal module
reorganization.

The same applies to the other 26 crates in the published set:

- `tree-sitter-perl-c`, `tree-sitter-perl-rs`
- `perl-parser-pest`
- `perl-lexer`, `perl-token`, `perl-line-index`, `perl-uri`, `perl-pod`
- `perl-diagnostics` (renamed from `perl-diagnostics-codes`, see Wave E below)
- `perl-lsp-protocol`, `perl-content-length-framing` (kept published — shared by LSP+DAP)
- `perl-semantic-analyzer`, `perl-module`, `perl-workspace`
- `perl-symbol`
- `perl-lsp-rs-core` (new in v0.13.0 — implementation sibling of `perl-lsp-rs`)
- `perl-lsp-perltidy`
- `perl-subprocess-runtime`, `perl-lsp-text-utils`
- `perl-corpus`, `perl-tdd-support`, `perl-test-must`, `perl-test-generators`
- `perl-parser-core`

## If you depend on a retired crate

If your `Cargo.toml` lists a dependency on any crate not in the list above, that crate
has been retired. Its code now lives as a module inside one of the 30 published crates.

**Steps to migrate:**

1. Find the retired crate name in the migration table below.
2. Replace the `Cargo.toml` dependency line with the new owning crate.
3. Update import paths using the old→new column.

---

## Migration table by wave

### Wave 1 — `perl-module-*` → `perl-module`

13 microcrates absorbed into `perl-module` as subfolder modules. The new crate name is
`perl-module` (not `perl-module-resolution` — see ADR-0041). Merged as PR #4422.

All sub-modules are re-exported at the crate root via `pub use api::*`.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-module-name` | `perl-module` | `use perl_module_name::` | `use perl_module::name::` |
| `perl-module-path` | `perl-module` | `use perl_module_path::` | `use perl_module::path::` |
| `perl-module-token` | `perl-module` | `use perl_module_token::` | `use perl_module::token::` |
| `perl-module-token-core` | `perl-module` | `use perl_module_token_core::` | `use perl_module::token_core::` |
| `perl-module-token-parser` | `perl-module` | `use perl_module_token_parser::` | `use perl_module::token_parser::` |
| `perl-module-boundary` | `perl-module` | `use perl_module_boundary::` | `use perl_module::boundary::` |
| `perl-module-import` | `perl-module` | `use perl_module_import::` | `use perl_module::import::` |
| `perl-module-import-match` | `perl-module` | `use perl_module_import_match::` | `use perl_module::import_match::` |
| `perl-module-reference` | `perl-module` | `use perl_module_reference::` | `use perl_module::reference::` |
| `perl-module-rename` | `perl-module` | `use perl_module_rename::` | `use perl_module::rename::` |
| `perl-module-resolution` | `perl-module` | `use perl_module_resolution::` | `use perl_module::resolution::` |
| `perl-module-resolution-path` | `perl-module` | `use perl_module_resolution_path::` | `use perl_module::resolution::` |
| `perl-module-resolution-uri` | `perl-module` | `use perl_module_resolution_uri::` | `use perl_module::resolution::` |

**Cargo.toml change:**

```toml
# Before (any of the 13 crates)
perl-module-name = "0.12.4"

# After
perl-module = "0.13.0"
```

---

### Wave 2 (Wave A) — `perl-workspace-*` → `perl-workspace`

6 workspace satellite crates absorbed into `perl-workspace`. The `perl-workspace-index`
crate is **renamed** to `perl-workspace` as part of this wave (the name better reflects
the broader scope). Merged as PRs #4434, #4438.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-workspace-index` | `perl-workspace` | `use perl_workspace_index::` | `use perl_workspace::` |
| `perl-workspace-folder` | `perl-workspace` | `use perl_workspace_folder::` | `use perl_workspace::folder::` |
| `perl-workspace-ignore` | `perl-workspace` | `use perl_workspace_ignore::` | `use perl_workspace::ignore::` |
| `perl-workspace-discovery` | `perl-workspace` | `use perl_workspace_discovery::` | `use perl_workspace::discovery::` |
| `perl-workspace-index-monitoring` | `perl-workspace` | `use perl_workspace_index_monitoring::` | `use perl_workspace::monitoring::` |
| `perl-workspace-index-state-machine` | `perl-workspace` | `use perl_workspace_index_state_machine::` | `use perl_workspace::state_machine::` |
| `perl-workspace-index-slo` | `perl-workspace` | `use perl_workspace_index_slo::` | `use perl_workspace::slo::` |

**Cargo.toml change:**

```toml
# Before
perl-workspace-index = "0.12.4"

# After — note: workspace alias preserved for smooth upgrade
perl-workspace = "0.13.0"
```

> The root `Cargo.toml` aliases `perl-workspace = { path = "crates/perl-workspace-index", version = "0.12.4" }`
> so the package is published under the new name. Your `Cargo.toml` dependency key changes
> from `perl-workspace-index` to `perl-workspace`.

---

### Wave 3 (Wave C) — Lexer satellites → `perl-lexer`

4 lexer satellite crates absorbed into `perl-lexer`. `perl-token` is **NOT** absorbed —
it remains a separate published foundation primitive. Merged as PRs #4433, #4486.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-tokenizer` | `perl-lexer` | `use perl_tokenizer::` | `use perl_lexer::tokenizer::` |
| `perl-keywords` | `perl-lexer` | `use perl_keywords::` | `use perl_lexer::keywords::` |
| `perl-builtins` | `perl-lexer` | `use perl_builtins::` | `use perl_lexer::builtins::` |
| `perl-builtins-phf` | `perl-lexer` | `use perl_builtins_phf::` | `use perl_lexer::builtins::` |

**Cargo.toml change:**

```toml
# Before
perl-tokenizer = "0.12.4"
perl-keywords = "0.12.4"

# After
perl-lexer = "0.13.0"
```

---

### Wave 4 (Wave D) — Parser/AST satellites → `perl-parser`

~20 parser and AST satellite crates absorbed into `perl-parser`. `perl-line-index`,
`perl-uri`, and `perl-pod` are **NOT** absorbed — they remain foundation primitives.
`perl-uri-classify` folds into `perl-uri` (not `perl-parser`). Merged as PRs #4493,
#4506, #4510.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-ast` | `perl-parser` | `use perl_ast::` | `use perl_parser::` (re-exported) |
| `perl-ast-v2` | `perl-parser` | `use perl_ast_v2::` | `use perl_parser::` (re-exported) |
| `perl-ast-utils` | `perl-parser` | `use perl_ast_utils::` | `use perl_parser::` |
| `perl-quote` | `perl-parser` | `use perl_quote::` | `use perl_parser::` |
| `perl-heredoc` | `perl-parser` | `use perl_heredoc::` | `use perl_parser::` |
| `perl-heredoc-anti-patterns` | `perl-parser` | `use perl_heredoc_anti_patterns::` | `use perl_parser::` |
| `perl-error` | `perl-parser` | `use perl_error::` | `use perl_parser::` |
| `perl-incremental-parsing` | `perl-parser` | `use perl_incremental_parsing::` | `use perl_parser::incremental::` |
| `perl-refactoring` | `perl-parser` | `use perl_refactoring::` | `use perl_parser::refactor::` |
| `perl-dead-code` | `perl-parser` | `use perl_dead_code::` | `use perl_parser::dead_code::` |
| `perl-feature-catalog` | `perl-parser` | `use perl_feature_catalog::` | `use perl_parser::` (build-dep) |
| `perl-position-tracking` | `perl-parser` | `use perl_position_tracking::` | `use perl_parser::` |
| `perl-qualified-name` | `perl-parser` | `use perl_qualified_name::` | `use perl_parser::` |
| `perl-source-file` | `perl-parser` | `use perl_source_file::` | `use perl_parser::` |
| `perl-percentile` | `perl-parser` | `use perl_percentile::` | `use perl_parser::` |
| `perl-text-line` | `perl-parser` | `use perl_text_line::` | `use perl_parser::` |
| `perl-edit` | `perl-parser` | `use perl_edit::` | `use perl_parser::` |
| `perl-path-normalize` | `perl-parser` | `use perl_path_normalize::` | `use perl_parser::` |
| `perl-path-security` | `perl-parser` | `use perl_path_security::` | `use perl_parser::` |
| `perl-uri-classify` | `perl-uri` | `use perl_uri_classify::` | `use perl_uri::` |

**Cargo.toml change:**

```toml
# Before
perl-ast = "0.12.4"
perl-incremental-parsing = "0.12.4"

# After
perl-parser = "0.13.0"
```

**Feature flag change — `incremental` feature:**

The `perl-incremental-parsing` crate was feature-gated in its consumer. In v0.13.0, the
incremental parsing module lives in `perl-parser` and is controlled by the `incremental`
feature flag on `perl-parser`:

```toml
# Before
perl-incremental-parsing = { version = "0.12.4", optional = true }

# After
perl-parser = { version = "0.13.0", features = ["incremental"] }
```

**Feature flag change — `workspace_refactor` feature:**

The refactoring module is accessible at `perl_parser::refactor`. Enable the
`workspace_refactor` feature to gate it at build time:

```toml
perl-parser = { version = "0.13.0", features = ["workspace_refactor"] }
```

---

### Wave B — `perl-symbol-*` → `perl-symbol`

4 symbol satellite crates absorbed into a new `perl-symbol` published crate. This is a
**new crate** (not a pre-existing one) that keeps symbols separate from `perl-semantic-analyzer`
to preserve the correct dependency layering. Merged as PRs #4459, #4435.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-symbol-types` | `perl-symbol` | `use perl_symbol_types::` | `use perl_symbol::types::` |
| `perl-symbol-cursor` | `perl-symbol` | `use perl_symbol_cursor::` | `use perl_symbol::cursor::` |
| `perl-symbol-index` | `perl-symbol` | `use perl_symbol_index::` | `use perl_symbol::index::` |
| `perl-symbol-surface` | `perl-symbol` | `use perl_symbol_surface::` | `use perl_symbol::surface::` |

**Cargo.toml change:**

```toml
# Before
perl-symbol-types = "0.12.4"

# After
perl-symbol = "0.13.0"
```

---

### Wave E — Diagnostic catalog

3 diagnostic crates merged into a single `perl-diagnostics` published crate. The crate
`perl-diagnostics-codes` is **renamed** to `perl-diagnostics`. Merged as PR #4521.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-diagnostics-codes` | `perl-diagnostics` | `use perl_diagnostics_codes::` | `use perl_diagnostics::codes::` |
| `perl-lsp-diagnostic-types` | `perl-diagnostics` | `use perl_lsp_diagnostic_types::` | `use perl_diagnostics::types::` |
| `perl-lsp-diagnostic-catalog` | `perl-diagnostics` | `use perl_lsp_diagnostic_catalog::` | `use perl_diagnostics::catalog::` |

**Cargo.toml change:**

```toml
# Before
perl-diagnostics-codes = "0.12.4"
perl-lsp-diagnostic-types = "0.12.4"

# After
perl-diagnostics = "0.13.0"
```

**Type unification note:** `DiagnosticSeverity` and `DiagnosticTag` are defined in
`perl_diagnostics::codes` and re-exported from `perl_diagnostics::types`. The legacy
`use perl_lsp_diagnostic_types::DiagnosticSeverity` path still resolves to the same
underlying type via re-exports, but the canonical path is now `perl_diagnostics::DiagnosticSeverity`.

---

### Wave F — `perl-lsp-feature-*` → `perl-lsp-rs-core::features`

8 LSP feature/capability crates absorbed into the new `perl-lsp-rs-core` implementation
crate as the `features` module. This wave also creates `perl-lsp-rs-core` as a new
published crate (mirroring `perl-parser`/`perl-parser-core` split). Merged as PR #4539.

**New crate: `perl-lsp-rs-core`** — the implementation sibling of `perl-lsp-rs`. The
`perl-lsp-rs` facade re-exports from `perl-lsp-rs-core`.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-lsp-feature-ids` | `perl-lsp-rs-core` | `use perl_lsp_feature_ids::` | `use perl_lsp_rs_core::features::ids::` |
| `perl-lsp-feature-contracts` | `perl-lsp-rs-core` | `use perl_lsp_feature_contracts::` | `use perl_lsp_rs_core::features::contracts::` |
| `perl-lsp-feature-flags` | `perl-lsp-rs-core` | `use perl_lsp_feature_flags::` | `use perl_lsp_rs_core::features::flags::` |
| `perl-lsp-feature-profile` | `perl-lsp-rs-core` | `use perl_lsp_feature_profile::` | `use perl_lsp_rs_core::features::profile::` |
| `perl-lsp-feature-profile-cli` | `perl-lsp-rs-core` | `use perl_lsp_feature_profile_cli::` | `use perl_lsp_rs_core::features::profile_cli::` |
| `perl-lsp-feature-policy` | `perl-lsp-rs-core` | `use perl_lsp_feature_policy::` | `use perl_lsp_rs_core::features::policy::` |
| `perl-lsp-feature-grid` | `perl-lsp-rs-core` | `use perl_lsp_feature_grid::` | `use perl_lsp_rs_core::features::grid::` |
| `perl-lsp-capability-map` | `perl-lsp-rs-core` | `use perl_lsp_capability_map::` | `use perl_lsp_rs_core::capability_map::` |

**Feature flag change — `lsp-ga-lock`:**

This feature previously existed on the individual feature crates. In v0.13.0, it is
consolidated on `perl-lsp-rs-core`:

```toml
# Before (any of the 5 feature crates that had lsp-ga-lock)
perl-lsp-feature-flags = { version = "0.12.4", features = ["lsp-ga-lock"] }

# After
perl-lsp-rs-core = { version = "0.13.0", features = ["lsp-ga-lock"] }
```

**Feature flag change — `lsp-compat`:**

The `lsp-compat` feature (LSP type compatibility shim) is now on `perl-lsp-rs-core`
and forwarded through `perl-parser` and `perl-workspace`:

```toml
# After — enable lsp-compat for LSP type shim across the stack
perl-lsp-rs-core = { version = "0.13.0", features = ["lsp-compat"] }
```

**Cargo.toml change:**

```toml
# Before
perl-lsp-feature-flags = "0.12.4"
perl-lsp-feature-contracts = "0.12.4"

# After
perl-lsp-rs-core = "0.13.0"
```

---

### Wave G1 — LSP providers → `perl-lsp-rs-core::providers`

25 LSP provider crates absorbed into `perl-lsp-rs-core::providers`. This is the largest
wave by crate count. Merged as PR #4543.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-lsp-providers` | `perl-lsp-rs-core` | `use perl_lsp_providers::` | `use perl_lsp_rs_core::providers::lsp_compat::` |
| `perl-lsp-navigation` | `perl-lsp-rs-core` | `use perl_lsp_navigation::` | `use perl_lsp_rs_core::providers::navigation::` |
| `perl-lsp-completion` | `perl-lsp-rs-core` | `use perl_lsp_completion::` | `use perl_lsp_rs_core::providers::completion::` |
| `perl-lsp-completion-item` | `perl-lsp-rs-core` | `use perl_lsp_completion_item::` | `use perl_lsp_rs_core::providers::completion_item::` |
| `perl-lsp-file-completion` | `perl-lsp-rs-core` | `use perl_lsp_file_completion::` | `use perl_lsp_rs_core::providers::file_completion::` |
| `perl-lsp-inline-completion` | `perl-lsp-rs-core` | `use perl_lsp_inline_completion::` | `use perl_lsp_rs_core::providers::inline_completion::` |
| `perl-lsp-ai-provider` | `perl-lsp-rs-core` | `use perl_lsp_ai_provider::` | `use perl_lsp_rs_core::providers::ai::` |
| `perl-lsp-code-actions` | `perl-lsp-rs-core` | `use perl_lsp_code_actions::` | `use perl_lsp_rs_core::providers::code_actions::` |
| `perl-lsp-code-lens` | `perl-lsp-rs-core` | `use perl_lsp_code_lens::` | `use perl_lsp_rs_core::providers::code_lens::` |
| `perl-lsp-document-highlight` | `perl-lsp-rs-core` | `use perl_lsp_document_highlight::` | `use perl_lsp_rs_core::providers::document_highlight::` |
| `perl-lsp-document-links` | `perl-lsp-rs-core` | `use perl_lsp_document_links::` | `use perl_lsp_rs_core::providers::document_links::` |
| `perl-lsp-folding` | `perl-lsp-rs-core` | `use perl_lsp_folding::` | `use perl_lsp_rs_core::providers::folding::` |
| `perl-lsp-selection-range` | `perl-lsp-rs-core` | `use perl_lsp_selection_range::` | `use perl_lsp_rs_core::providers::selection_range::` |
| `perl-lsp-semantic-tokens` | `perl-lsp-rs-core` | `use perl_lsp_semantic_tokens::` | `use perl_lsp_rs_core::providers::semantic_tokens::` |
| `perl-lsp-inlay-hints` | `perl-lsp-rs-core` | `use perl_lsp_inlay_hints::` | `use perl_lsp_rs_core::providers::inlay_hints::` |
| `perl-lsp-rename` | `perl-lsp-rs-core` | `use perl_lsp_rename::` | `use perl_lsp_rs_core::providers::rename::` |
| `perl-lsp-type-hierarchy` | `perl-lsp-rs-core` | `use perl_lsp_type_hierarchy::` | `use perl_lsp_rs_core::providers::type_hierarchy::` |
| `perl-lsp-workspace-symbols` | `perl-lsp-rs-core` | `use perl_lsp_workspace_symbols::` | `use perl_lsp_rs_core::providers::workspace_symbols::` |
| `perl-lsp-symbol-query` | `perl-lsp-rs-core` | `use perl_lsp_symbol_query::` | `use perl_lsp_rs_core::providers::symbol_query::` |
| `perl-lsp-formatting` | `perl-lsp-rs-core` | `use perl_lsp_formatting::` | `use perl_lsp_rs_core::providers::formatting::` |
| `perl-lsp-formatting-types` | `perl-lsp-rs-core` | `use perl_lsp_formatting_types::` | `use perl_lsp_rs_core::providers::formatting_types::` |
| `perl-lsp-on-type-formatting` | `perl-lsp-rs-core` | `use perl_lsp_on_type_formatting::` | `use perl_lsp_rs_core::providers::on_type_formatting::` |
| `perl-lsp-color-provider` | `perl-lsp-rs-core` | `use perl_lsp_color_provider::` | `use perl_lsp_rs_core::providers::color::` |
| `perl-lsp-diagnostics` | `perl-lsp-rs-core` | `use perl_lsp_diagnostics::` | `use perl_lsp_rs_core::providers::diagnostics::` |
| `perl-lsp-import-management` | `perl-lsp-rs-core` | `use perl_lsp_import_management::` | `use perl_lsp_rs_core::providers::import_management::` |

**Cargo.toml change:**

```toml
# Before (any provider crate)
perl-lsp-navigation = "0.12.4"
perl-lsp-completion = "0.12.4"

# After
perl-lsp-rs-core = "0.13.0"
```

---

### Wave G2 — LSP runtime → `perl-lsp-rs-core::runtime`

5 LSP runtime infrastructure crates absorbed into `perl-lsp-rs-core::runtime`. Merged as PR #4543.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-lsp-cancellation` | `perl-lsp-rs-core` | `use perl_lsp_cancellation::` | `use perl_lsp_rs_core::runtime::cancellation::` |
| `perl-lsp-limits` | `perl-lsp-rs-core` | `use perl_lsp_limits::` | `use perl_lsp_rs_core::runtime::limits::` |
| `perl-lsp-launcher` | `perl-lsp-rs-core` | `use perl_lsp_launcher::` | `use perl_lsp_rs_core::runtime::launcher::` |
| `perl-lsp-input-validation` | `perl-lsp-rs-core` | `use perl_lsp_input_validation::` | `use perl_lsp_rs_core::runtime::input_validation::` |
| `perl-lsp-text-utils` | `perl-lsp-rs-core` | `use perl_lsp_text_utils::` | `use perl_lsp_rs_core::runtime::text_utils::` |

> **Note:** `perl-lsp-text-utils` remains published at v0.13.0 for external consumers who use it
> independently. The absorption into `perl-lsp-rs-core::runtime::text_utils` provides an
> additional access path; the standalone crate is still published.

**Cargo.toml change:**

```toml
# Before
perl-lsp-cancellation = "0.12.4"

# After
perl-lsp-rs-core = "0.13.0"
```

---

### Wave G3 — LSP governance/tooling/infra → `perl-lsp-rs-core`

7 LSP governance, tooling, and infrastructure crates absorbed into `perl-lsp-rs-core`.
Merged as PR #4543.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-lsp-feature-governance` | `perl-lsp-rs-core` | `use perl_lsp_feature_governance::` | `use perl_lsp_rs_core::governance::` |
| `perl-lsp-tooling` | `perl-lsp-rs-core` | `use perl_lsp_tooling::` | `use perl_lsp_rs_core::tooling::` |
| `perl-lsp-performance` | `perl-lsp-rs-core` | `use perl_lsp_performance::` | `use perl_lsp_rs_core::performance::` |
| `perl-lsp-critic-parser` | `perl-lsp-rs-core` | `use perl_lsp_critic_parser::` | `use perl_lsp_rs_core::critic_parser::` |
| `perl-lsp-transport` | `perl-lsp-rs-core` | `use perl_lsp_transport::` | `use perl_lsp_rs_core::transport::` |
| `perl-lsp-uri` | `perl-lsp-rs-core` | `use perl_lsp_uri::` | `use perl_lsp_rs_core::uri::` |
| `perl-lsp-config` | `perl-lsp-rs-core` | `use perl_lsp_config::` | `use perl_lsp_rs_core::config::` |

**Cargo.toml change:**

```toml
# Before
perl-lsp-feature-governance = "0.12.4"

# After
perl-lsp-rs-core = "0.13.0"
```

---

### Wave H — `perl-dap-*` → `perl-dap`

11 DAP satellite crates absorbed into `perl-dap`. Merged as PR #4544.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-dap-breakpoint` | `perl-dap` | `use perl_dap_breakpoint::` | `use perl_dap::breakpoint::` |
| `perl-dap-eval` | `perl-dap` | `use perl_dap_eval::` | `use perl_dap::eval::` |
| `perl-dap-config` | `perl-dap` | `use perl_dap_config::` | `use perl_dap::config::` |
| `perl-dap-platform` | `perl-dap` | `use perl_dap_platform::` | `use perl_dap::platform::` |
| `perl-dap-command-args` | `perl-dap` | `use perl_dap_command_args::` | `use perl_dap::command_args::` |
| `perl-dap-shell` | `perl-dap` | `use perl_dap_shell::` | `use perl_dap::shell::` |
| `perl-dap-stack` | `perl-dap` | `use perl_dap_stack::` | `use perl_dap::stack::` |
| `perl-dap-types` | `perl-dap` | `use perl_dap_types::` | `use perl_dap::types::` |
| `perl-dap-value` | `perl-dap` | `use perl_dap_value::` | `use perl_dap::value::` |
| `perl-dap-security` | `perl-dap` | `use perl_dap_security::` | `use perl_dap::security::` |
| `perl-dap-variables` | `perl-dap` | `use perl_dap_variables::` | `use perl_dap::variables::` |

**Cargo.toml change:**

```toml
# Before
perl-dap-breakpoint = "0.12.4"
perl-dap-types = "0.12.4"

# After
perl-dap = "0.13.0"
```

---

### Wave Final — Remaining deferrals → `perl-lsp-rs-core`

3 crates that were deferred from earlier waves are absorbed in the final PR. Merged as PR #4544.

| Retired crate | New owning crate | Old import path | New import path |
|---|---|---|---|
| `perl-feature-catalog` | `perl-lsp-rs-core` | `use perl_feature_catalog::` | `use perl_lsp_rs_core::feature_catalog::` |
| `perl-lsp-config` | `perl-lsp-rs-core` | `use perl_lsp_config::` | `use perl_lsp_rs_core::config::` |
| `perl-content-length-framing` | `perl-lsp-rs-core` | `use perl_content_length_framing::` | `use perl_lsp_rs_core::transport::framing::` |

> **Note on `perl-content-length-framing`:** This crate is small (~150 LOC) and shared by
> both LSP and DAP layers. It is absorbed into `perl-lsp-rs-core::transport::framing` in the
> final PR. If you use it in a DAP context, access it via `perl_lsp_rs_core::transport::framing`.

---

## Breaking changes per wave

### Wave 1 (perl-module-*)

- **No breaking changes** in the public surface. All types re-exported from `perl-module::api`.
- `perl-module-resolution` was an internal name; the new public module is `perl_module::resolution`.

### Wave 2 (perl-workspace-*)

- **Crate rename:** `perl-workspace-index` → `perl-workspace`. Update your `Cargo.toml`
  dependency key and import prefix.
- The `perl-workspace` feature flag (`workspace = []`) controls workspace indexing; ensure
  your Cargo.toml enables it if you use workspace-wide features.

### Wave 3 (lexer satellites)

- **No breaking changes** in the public surface. All lexer types remain accessible through
  `perl-lexer`.

### Wave 4 (parser/AST satellites)

- **`perl-incremental-parsing` is now feature-gated** inside `perl-parser`. You must enable
  the `incremental` feature:
  ```toml
  perl-parser = { version = "0.13.0", features = ["incremental"] }
  ```
- **`perl-path-security` removed from public surface** — absorbed into `perl-parser` internals.
  If you depended on its security primitives directly, access them via `perl_parser`.

### Wave B (perl-symbol-*)

- **New crate `perl-symbol`** — this was not published before. Previously you would have depended
  on `perl-symbol-types`, `perl-symbol-cursor`, etc. directly.

### Wave E (diagnostics)

- **`perl-diagnostics-codes` renamed to `perl-diagnostics`**. Update Cargo dependency.
- `DiagnosticSeverity` and `DiagnosticTag` are now canonical in `perl_diagnostics::codes`.
  The `perl_diagnostics::types::DiagnosticSeverity` re-export is available but the canonical
  path has changed.

### Wave F (perl-lsp-feature-*)

- **New crate `perl-lsp-rs-core`** — all feature flag APIs migrate here.
- **`lsp-ga-lock` feature** consolidates to `perl-lsp-rs-core`. See feature flag section.

### Wave G1/G2/G3 (LSP providers, runtime, governance)

- **No breaking changes to `perl-lsp-rs` API** — it remains the facade and re-exports
  what consumers need. Changes are internal.
- Direct dependencies on provider crates (`perl-lsp-navigation`, etc.) are removed from
  crates.io; replace with `perl-lsp-rs-core`.

### Wave H (perl-dap-*)

- **`perl-dap-platform` had `cfg(unix)`/`cfg(windows)` preserved** — platform-specific
  code is available at `perl_dap::platform` with the same conditional compilation guards.

### Wave Final

- **`perl-content-length-framing`** is no longer published. Use
  `perl_lsp_rs_core::transport::framing` instead.

---

## Feature flag reference (v0.13.0)

All feature flags that were previously spread across microcrates are consolidated on
their owning published crate in v0.13.0.

| Feature | Owning crate | Old location | Purpose |
|---|---|---|---|
| `lsp-ga-lock` | `perl-lsp-rs-core` | `perl-lsp-feature-{flags,contracts,ids,...}` | Restrict capabilities to GA-only set for emergency point releases |
| `lsp-compat` | `perl-lsp-rs-core`, `perl-parser`, `perl-workspace` | `perl-lsp-feature-flags` | LSP type compatibility shim for migration period |
| `incremental` | `perl-parser` | `perl-incremental-parsing` | Incremental parsing with segment-based token cache |
| `workspace` | `perl-workspace` | `perl-workspace-index` | Workspace-wide indexing for cross-file features |
| `workspace_refactor` | `perl-parser` | `perl-refactoring` | Workspace-wide rename and refactoring operations |
| `lsp-advanced` | `perl-parser` | various | Experimental LSP features (profiling, git integration) |
| `experimental-features` | `perl-parser` | various | Experimental features for testing |

**Enabling feature flags in v0.13.0:**

```toml
# Enable incremental parsing and workspace-wide indexing
perl-parser = { version = "0.13.0", features = ["incremental", "workspace"] }

# Enable the lsp-compat shim (needed if you consume lsp-types alongside perl-lsp types)
perl-lsp-rs-core = { version = "0.13.0", features = ["lsp-compat"] }

# Lock capabilities to GA set only
perl-lsp-rs-core = { version = "0.13.0", features = ["lsp-ga-lock"] }
```

---

## Compat aliases and removal schedule

The retired crates are **not re-published as shims**. There is no grace period at the
crate level — old crate names disappear from crates.io with this release.

Within the published crates, type aliases are provided where renamed types would break
downstream `impl` blocks:

- `perl_diagnostics::types::DiagnosticSeverity` → alias for `perl_diagnostics::codes::DiagnosticSeverity`
  (kept through 0.14.0)
- `perl_workspace::WorkspaceIndex` → re-export of the internal index type
  (kept through 0.14.0)

**Removal schedule:**

| Alias | Kept through | Then |
|---|---|---|
| `perl_diagnostics::types::DiagnosticSeverity` | 0.14.0 | Removed; use `perl_diagnostics::DiagnosticSeverity` |
| `perl_diagnostics::types::DiagnosticTag` | 0.14.0 | Removed; use `perl_diagnostics::DiagnosticTag` |
| `perl_workspace::WorkspaceIndex` | 0.14.0 | Removed; use `perl_workspace::workspace::WorkspaceIndex` |

---

## Published crate count

| Version | Published crates | Change |
|---|---|---|
| 0.12.4 | 132 | baseline |
| 0.13.0 (Wave 1) | 119 | −13 (perl-module-*) |
| 0.13.0 (Wave 2) | 113 | −6 (perl-workspace-*) |
| 0.13.0 (Wave 3) | 109 | −4 (lexer satellites) |
| 0.13.0 (Wave 4) | 90 | −19 (parser/AST satellites) |
| 0.13.0 (Wave B) | 87 | −4 (perl-symbol-*), +1 (perl-symbol NEW) |
| 0.13.0 (Wave E) | 85 | −3 (diagnostic crates), +1 (perl-diagnostics RENAMED) |
| 0.13.0 (Wave F) | 78 | −8 (perl-lsp-feature-*), +1 (perl-lsp-rs-core NEW) |
| 0.13.0 (Wave G1) | 53 | −25 (LSP providers) |
| 0.13.0 (Wave G2) | 49 | −5 (LSP runtime, −1 net: perl-lsp-text-utils kept) |
| 0.13.0 (Wave G3) | 43 | −7 (LSP governance/infra) |
| 0.13.0 (Wave H) | 32 | −11 (perl-dap-*) |
| 0.13.0 (Wave Final) | **30** | −3 (remaining deferrals) |

---

## Why

The microcrate architecture delivered on agent-friendly work units but did not deliver on
decoupled versioning, smaller publish surface, or faster compile times. The fundamental
constraint is that crates.io forbids path-only dependencies in published crates, so every
internal architectural seam expressed as a crate boundary became a permanent public
artifact and a semver contract. The full analysis is in
[ADR-0041](adr/0041-microcrate-collapse.md).

## Timeline

- The collapse ran across 14 PRs from 2026-04-15 to 2026-04-21.
- v0.13.0 is the first release with the new 30-crate surface.
- There is no extended migration window — old crate names are not re-published as shims.

If you have questions, open a discussion on
[tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410).
