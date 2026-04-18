# ADR — work-7c16e7ae

## Status
Proposed

## Context

GitHub issue #3431 reports that the VS Code extension's `snippets/perl.json` lacks snippets for modern Perl patterns and framework-specific idioms. Investigation revealed two independent snippet systems with gaps:

1. **VS Code native snippets** (`vscode-extension/snippets/perl.json`) — 58 snippets, served directly by VS Code extension
2. **LSP completion snippets** (`crates/perl-lsp-completion/src/completion/snippets.rs`) — 56 snippets, served via `textDocument/completion` protocol to all LSP clients

Additionally, `crates/perl-lexer/src/keywords/mod.rs` has `LSP_COMPLETION_KEYWORDS` which is missing modern Perl keywords (`class`, `method`, `field`, `defer`, `given`, `when`, `catch`, `finally`, `say`) that the lexer already recognizes.

**Critical factual error in prior plan**: The initial plan claimed "`state` and `say` are already present in `LSP_COMPLETION_KEYWORDS`" — this is **incorrect**. `say` is NOT in that list and must be added.

## Decision

### Scope: Fix both VS Code snippets AND LSP completion snippets AND keywords

The original issue mentions VS Code snippets, but the LSP snippet system has identical gaps. These are separate code paths serving different use cases:
- VS Code native snippets: VS Code users not using LSP server
- LSP snippets: All LSP clients (VS Code with LSP server, Neovim, Emacs, etc.)

Updating only one would leave the other incomplete. Adding keywords to `LSP_COMPLETION_KEYWORDS` activates dead `keyword_doc()` entries and enables keyword completion.

### Trigger Naming Strategy

- **VS Code**: Use short, keyword-style prefixes following existing conventions (`class`, `method`, `field`, `defer`, `given`, `when`, `catch`, etc.) — NOT descriptive names like `perlclass`
- **LSP**: Use descriptive names to avoid confusion with actual keywords (`perlclass`, `perlmethod`, `perlfield`, `deferblock`, etc.)
- **Avoid duplication**: VS Code already has `try` snippet — LSP should NOT add `trycatch` (would duplicate `try` prefix in VS Code via LSP)

### Keyword Addition

Add to `LSP_COMPLETION_KEYWORDS` (must remain alphabetically sorted):
- `catch` — Perl 5.34+ try/catch
- `class` — Perl 5.38+ class declaration
- `defer` — Perl 5.36+ deferred block
- `field` — Perl 5.38+ field declaration
- `finally` — Perl 5.34+ try/catch/finally
- `given` — Perl 5.10+ switch (experimental)
- `method` — Perl 5.38+ method within class
- `say` — Perl 5.10+ say (previously thought to exist, confirmed missing)
- `when` — Perl 5.10+ case (experimental)

Note: `state` is already present and does not need to be added.

## Alternatives Considered

### Alternative 1: VS Code snippets only (per original issue)
- **Rejected because**: LSP clients (Neovim, Emacs, VS Code with LSP server) would have no improvement. The gap is systemic, not VS Code-specific.
- **Tradeoff**: Narrower scope, faster to implement, but incomplete fix

### Alternative 2: Keywords only (no snippets)
- **Rejected because**: Keywords without snippets don't provide the template/boilerplate experience users expect. Snippets turn a keyword into a usable code pattern.
- **Tradeoff**: Simpler change, but doesn't address the UX gap described in the issue

### Alternative 3: Full expansion with descriptive names everywhere
- **Rejected because**: VS Code snippets follow a convention of short keyword-style prefixes. Adding `perlclass`, `perlmethod` would break consistency and discoverability.
- **Tradeoff**: More descriptive, but inconsistent with existing VS Code snippet style

## Consequences

### Benefits
- Modern Perl users (5.34+ class syntax, 5.38+ native classes) get IDE support
- DBI patterns, Test::More extensions, Moo/Moose method modifiers become accessible
- `keyword_doc()` dead entries for `given`, `when`, `say` become functional
- Both VS Code native and LSP clients receive improvements

### Risks
1. **Keyword ordering**: `LSP_COMPLETION_KEYWORDS` must remain sorted — implementation must specify exact insertion positions
2. **Trigger conflicts**: New snippets must not duplicate existing triggers in either system
3. **Discovery degradation**: Adding ~15-20 snippets per system may make menus harder to navigate
4. **Maintenance burden**: Two snippet systems must be kept in sync

### Mitigation
- Specify exact sorted positions for keyword insertions
- Verify no trigger conflicts before implementation (run `no_duplicate_triggers` test)
- Use VS Code-compatible trigger names that match existing conventions
- Add Perl version requirements in doc fields for version-gated features
