# Research Findings — work-260a5712

## Issue Summary
GitHub issue #3430 requests VS Code UI for perltidy profile options (indentation, tab size, line length, cuddled else), a profile-selection command, and language support for `.perltidyrc` files. Currently only `perl-lsp.perltidyConfig` (a path to `.perltidyrc`) exists.

## Relevant Codebase Areas
- `crates/perl-lsp-perltidy/src/lib.rs` — `PerlTidyConfig` with all perltidy options; built-in `pbp()` and `gnu()` profiles
- `crates/perl-lsp-formatting/src/formatting.rs` — `FormattingProvider<R>` accepting `PerlTidyConfig` via builder
- `vscode-extension/package.json` lines 275–300 — current perltidy settings (only path string)
- `vscode-extension/src/extension.ts` — command registration, config change handler
- `crates/perl-lsp-formatting-types/src/lib.rs` — `FormattingOptions` with `tabSize`, `insertSpaces`

## Key Findings
The Rust-side `PerlTidyConfig` fully covers all options mentioned in the issue. The `FormattingProvider` accepts a config. Only the VS Code settings layer is missing — individual options are not exposed, and no profile-selection command exists.

## Proposed Approach
Add a nested `perl-lsp.perltidy` VS Code settings object (indentation, tabSize, lineLength, cuddledElse, profile) alongside the existing `perltidyConfig` path. Wire these through `workspace/didChangeConfiguration` to the LSP server's `FormattingProvider`. Add a `perl-lsp.selectPerlTidyProfile` quick-pick command following the `setPerlCriticSeverity` pattern.

## Top Risks
1. Settings migration: existing `perltidyConfig` path vs new individual options — need clear precedence
2. LSP server config merging: VS Code settings vs `.perltidyrc` file — need to define which wins
3. Breaking the format flow: modifying config wiring could break existing format-on-save users

## Scope
Covers: VS Code settings, profile-selection command, status menu entry, Neovim/Emacs docs.
Does NOT cover: live preview WebView (`previewPerlTidy`), `.perltidyrc` language support (syntax highlighting/validation), any Rust-side changes to `perl-lsp-perltidy`.
