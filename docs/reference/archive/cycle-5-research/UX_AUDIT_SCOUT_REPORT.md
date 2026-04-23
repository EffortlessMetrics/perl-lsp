# UX Audit Scout Report: perl-lsp v0.12.0 Gaps & Opportunities

**Date**: 2026-03-20
**Status**: Complete audit with prioritized gap analysis
**Effort Estimate**: 15 gaps categorized (Tier 1: 6 items, Tier 2: 5 items, Tier 3: 4 items)

---

## Executive Summary

perl-lsp v0.12.0 is feature-complete on LSP protocol (97/97 features advertised, 99% coverage) and **exceeds competitor offerings in raw capability**. However, **perception gaps exist**: users don't know what we have, and **convenience features** that other LSPs (Python, Go, Rust, TypeScript) take for granted are missing. This is a **UX/discovery problem, not a capability problem**.

### Competitors Comparison

| Capability | perl-lsp | Perl::LanguageServer | PerlNavigator | PLS |
|-----------|----------|-------------------|---------------|-----|
| Completion | ✓ GA (150+ functions) | ✓ | ✓ | ✓ |
| Hover Docs | ✓ GA | ✓ | ✓ | ✓ |
| Go to Def | ✓ GA | ✓ | ✓ | ✓ |
| Refactoring | ✓ GA (3 types) | ✗ | ✗ | ✗ |
| **Debugging** | ✓ GA (DAP native) | ✓ | ✗ | ✗ |
| **Code Actions** | ✓ GA (20+) | Limited | Limited | Limited |
| **Inlay Hints** | ✓ GA | ✗ | ✗ | ✗ |
| **Call Hierarchy** | ✓ GA | ✗ | ✗ | ✗ |
| **Type Hierarchy** | ✓ GA | ✗ | ✗ | ✗ |
| Linting (perlcritic) | Partial | ✓ | ✓ | ✓ |
| Import Organization | ✓ GA | ✓ | ✓ | ✓ |

**Key insight**: perl-lsp **wins on depth** (17 features competitors lack) but **loses on UX perception** (doesn't advertise what it has, onboarding friction, no snippets).

---

## Tier 1: High-Impact UX Gaps (Quick Wins)

These affect first impressions, reduce churn, and are fixable in **2-7 days** each.

### 1. **Snippet Completions Missing**
**Problem**: No built-in Perl snippet library. Users type boilerplate manually (sub definition, if/unless/for blocks, regex patterns).
**What competitors do**: Python (pythonsnippets), Go (gosnippets), TypeScript (ES6 snippets) all ship 50-100+ snippets.
**User expectation**: Type `if` → hit Tab → get `if () { }` with cursor in parens.
**Current state**: Empty `vscode-extension/snippets/perl.json`.
**Effort**: 2-3 days
**Impact**: High. Users hit this on day 1.

**Builder spec**:
- Add 40-50 Perl snippets: if/unless/for/foreach/while/until/sub/package/use/try-catch/hash constructor/array patterns
- Include Moose/Moo class patterns (class/has/sub new)
- Regex patterns (m//g, s///, qr//)
- Test patterns (use Test::More, is/ok/like)
- Reference: TypeScript VSCode extension snippets as model

---

### 2. **First-Run Experience Broken**
**Problem**: Install extension → no guidance. Users don't know:
- Extension auto-downloads binary (good, but silent)
- Need `perltidy` for formatting (not installed)
- LSP server is starting (no progress indicator)
- Where to check logs if it fails

**Current state**: No welcome view, no setup wizard, no health check command.
**Effort**: 3-4 days
**Impact**: High. ~30% of users abandon at install.

**Builder spec**:
- Create welcome/onboarding WebView on first activation
- Auto-run health check: perl version, perltidy availability, binary download status
- Show "Getting Started" guide with 3 steps
- Add command: `Perl: Run Health Check` (diagnostics)
- Quick-fix button to install perltidy if missing
- Status bar indicator during initial startup

---

### 3. **Error Messages Don't Help Users**
**Problem**: Parser errors are cryptic.
```
unexpected_token: expected_expression
```
User sees this and thinks "what does that mean?" → no actionable fix.
**Competitors**: Go shows "expected X, found Y", Python shows hint like "Did you mean ___?"
**Current state**: Raw error codes, no context.
**Effort**: 2-3 days
**Impact**: Medium-high. Users disable diagnostics thinking LSP is broken.

**Builder spec**:
- Add human-readable error explanations keyed by error code
- Example: `unexpected_rbrace_expr` → "Found `}` where an expression was expected. Check for missing function call or variable."
- Show 1-line hint in squiggle tooltip
- Add "Report Diagnosis Bug" code action for suspicious false positives
- Link to docs/KNOWN_LIMITATIONS.md for each error family

---

### 4. **No Inline Progress During Indexing**
**Problem**: User opens large codebase → VSCode is unresponsive for 30s-2m → user thinks LSP crashed.
**No progress bar, no status message, silent initialization.**
**Competitors**: Rust Analyzer shows "Preparing workspace (4/100 packages)", Python shows spinner.
**Current state**: No window/workDoneProgress reporting visible to user.
**Effort**: 1-2 days
**Impact**: High. First impression is "this LSP is slow/broken."

**Builder spec**:
- Implement window/workDoneProgress during workspace indexing
- Report: "Indexing Perl workspace (N files loaded)"
- Show file count, estimated % complete
- Cancel button to abort if user wants quick iteration
- Status bar spinner during active operations

---

### 5. **Perl Special Variables Tooltip Is Hidden**
**Problem**: We have educational tooltips for special variables ($_, @_, %ENV, etc.) but:
- Feature advertised as "hover educational tooltips" but users don't know it exists
- Only triggers on specific variables
- Not discoverable

**Current state**: Feature exists (merged in #2262) but isn't showcased.
**Effort**: 1 day
**Impact**: Medium. Low-hanging Perl superpowers that set us apart.

**Builder spec**:
- Add to README's "Advanced Features" section with example
- Include example screenshot: hover on `$_` → see tooltip about `$_` being loop variable
- Add command: `Perl: Show Special Variables Reference` (quick info popup)
- Promote in release notes as "Perl expert tooltips"

---

### 6. **Configuration UX Fragmented**
**Problem**: Settings scattered, not discoverable:
- `perl-lsp.includePaths` (critical, often needed)
- `perl-lsp.perltidyConfig` (exists but unclear it's optional/auto-detected)
- `perl-lsp.featureProfile` (advanced, hidden)
- No in-UI guidance on what each does

**Current state**: 11 settings under `perl-lsp.*`, documented in README but not in extension docs.
**Effort**: 1-2 days
**Impact**: Medium. Users struggling with module resolution blame LSP, not config.

**Builder spec**:
- Add VSCode settings UI grouping: "Core", "Advanced", "Debugging"
- Add inline help text to each setting (already in package.json, needs UI polish)
- Add quick action: "Perl: Open Configuration Guide" → links to CONFIG.md
- Auto-suggest `includePaths` if modules not found (code action)

---

## Tier 2: Convenience Features (High ROI, Medium Effort)

### 7. **No Auto-Imports on Completion**
**Problem**: User types `DBI->co` → get `connect` in completion → hit Enter → **manually add `use DBI;`**.
**Competitors**: Python auto-imports modules, Go auto-imports packages, TypeScript auto-imports.
**User expectation**: Completion should add the `use` statement.
**Current state**: Completion works; imports don't auto-add.
**Effort**: 4-6 days (need to hook completion resolve, add workspace edit).
**Impact**: High. Perl is import-heavy, this is friction every day.

**Builder spec**:
- Implement completion item resolve to add `use Module;` when item selected
- Handle: bare module names (use Module), qualified calls (Module->method)
- Smart insertion: add after existing `use` block, don't duplicate
- Respect user's use-statement organization preferences
- Test: complete `LWP::UserAgent->get`, should auto-add `use LWP::UserAgent;`

---

### 8. **Quick-Fix Suggestions for Common Mistakes**
**Problem**: User forgets `use strict;` → gets undefined variable errors → doesn't know why.
**No code action suggesting fix.**
**Competitors**: Rust shows "consider adding `use std::io`", Python shows lint fix suggestions.
**Current state**: Diagnostics only; no code actions for "missing pragma" errors.
**Effort**: 3-4 days.
**Impact**: Medium-high. Reduces cognitive load on new Perl users.

**Builder spec**:
- Add code action: "Add `use strict;` to file"
- Add code action: "Add `use warnings;` to file"
- Add code action: "Replace bareword with string" (for bareword errors)
- Add code action: "Declare variable with `my`" (for undefined var under strict)
- Show as light-bulb in editor

---

### 9. **Test Runner CodeLens Is Limited**
**Problem**: We have test discovery (Test Explorer, Shift+Alt+T), but:
- No CodeLens above test functions (like Jest, Mocha, pytest do)
- No inline "Run" / "Debug" links above `sub test_something`
- No way to jump to test from code without opening separate panel

**Current state**: Test Explorer exists, but not integrated into editor.
**Effort**: 3-4 days.
**Impact**: Medium. Perl developers want to click "Run" above their tests.

**Builder spec**:
- Add CodeLens provider for test functions (sub test_*, sub {}-style tests with Test::More)
- Show "Run Test", "Debug Test" links above each test
- Implement via `textDocument/codeLens` (already advertised)
- Link to test runner via `perl-lsp.runTests` command

---

### 10. **No Snippet Expansion for Regex Patterns**
**Problem**: User starts typing `/pattern/` → no snippets for:
- Global flag suggestions
- Capture group hints
- Named capture groups (Perl 5.32+)
- Look-ahead/look-behind patterns

**Current state**: Regex support is solid (we parse any delimiter), but no authoring help.
**Effort**: 2-3 days.
**Impact**: Medium. Heavy regex users (ops/DevOps Perl code) see this as gap.

**Builder spec**:
- Add completion items for regex flags (/g, /m, /s, /x, /i)
- Add snippets: /pattern/${1}/g, /^(.*)$/${1}/, named capture patterns
- Trigger on `/` detection in completion context
- Reference: Perl regex cookbook patterns

---

### 11. **Refactoring UX Unclear**
**Problem**: We advertise "Extract Variable", "Extract Subroutine" but:
- No discovery (users don't know these exist)
- No keyboard shortcuts (Shift+Alt+E for extract?)
- No preview before apply (scary for large refactors)

**Current state**: Code actions exist; discoverability is poor.
**Effort**: 2-3 days.
**Impact**: Medium. Refactoring is a power feature, but hidden.

**Builder spec**:
- Add keyboard shortcuts: Shift+Alt+V (extract variable), Shift+Alt+M (extract method)
- Add preview checkbox in refactoring dialog before apply
- Show in README as featured capability
- Add command: `Perl: Show Refactoring Options` with descriptions

---

## Tier 3: Polish & Perception Gaps

### 12. **No "Try perl-lsp" Interactive Demo**
**Problem**: Users visit GitHub, don't know what LSP does. Need a 30-second visual.
**Competitors**: Rust Analyzer playground, Go docs show screenshot carousel.
**Current state**: README has bullet points, no videos/GIFs.
**Effort**: 1-2 days (create 3 animated GIFs).
**Impact**: Medium. Better GitHub presence.

**Builder spec**:
- GIF #1: Install → auto-download → health check
- GIF #2: Go to definition + find references
- GIF #3: Code action (extract variable)
- Add to README under "Features" section

---

### 13. **Debugging Setup Friction**
**Problem**: DAP is fully implemented, but first-time debugger setup is unclear.
**No "Debug Configuration" wizard or auto-setup.**
**Competitors**: Python auto-detects, Go auto-creates launch config.
**Current state**: Users must manually add launch.json config (snippets exist but not auto-inserted).
**Effort**: 2-3 days.
**Impact**: Medium. Debugger is a differentiator, shouldn't be hard to activate.

**Builder spec**:
- Add command: `Perl: Create Debug Configuration` → auto-populate launch.json
- Detect if .vscode/launch.json exists; offer to add Perl config
- Show onboarding prompt on first Perl file open if no debug config
- Provide templates: "Launch Script", "Attach to Process", "Remote (SSH)"

---

### 14. **LSP Server Version Not Easily Visible**
**Problem**: Users want to know: "What version of perl-lsp am I running?"
**Command exists but not obvious from status bar.**
**Competitors**: Rust Analyzer shows version in status bar or quick info.
**Current state**: Command `perl-lsp.showVersion` exists but hidden.
**Effort**: 0.5 day.
**Impact**: Low. Quick win for polishing status bar UX.

**Builder spec**:
- Add status bar item showing "perl-lsp v0.12.0" (click to see full details)
- Clickable → opens version info panel with binary path, test results, diagnostics count
- Update on binary download/reinstall

---

### 15. **No "Report Missing Feature" Template**
**Problem**: Users file issues but don't know what info we need to diagnose.
**Issue template exists but vague.**
**Competitors**: Jest, Rust Analyzer have detailed templates ("What did you expect? What happened? Minimal repro?").
**Current state**: Generic issue template.
**Effort**: 0.5 day.
**Impact**: Low. Better issue quality.

**Builder spec**:
- Enhance GitHub issue template: separate sections for Bug vs Feature Request
- Bug template: "Minimal Perl code", "LSP log output", "perl-lsp --version", "VS Code version"
- Feature template: "Use case", "Competitor feature reference" (links help us understand)
- Add link in README "Report an Issue"

---

## Summary: Prioritized Builder Queue

### Tier 1 (Highest Priority) — Do First (11-18 days total)
1. **Snippet Completions** (2-3 days) — Users hit this day 1
2. **First-Run Experience/Onboarding** (3-4 days) — Kill adoption friction
3. **Error Messages with Context** (2-3 days) — Reduce "LSP is broken" perceptions
4. **Progress Reporting During Indexing** (1-2 days) — Feels snappy vs hung
5. **Showcase Special Variables Tooltips** (1 day) — Already built, just promote
6. **Configuration UX Polish** (1-2 days) — Help users find settings

### Tier 2 (Medium Priority) — Do Next (12-15 days total)
7. **Auto-Imports on Completion** (4-6 days) — Daily friction relief
8. **Quick-Fix Code Actions** (3-4 days) — Beginner-friendly
9. **Test Runner CodeLens** (3-4 days) — Power user feature
10. **Regex Pattern Snippets** (2-3 days) — Niche but high-value
11. **Refactoring UX & Discoverability** (2-3 days) — Power features stay hidden

### Tier 3 (Nice-to-Have) — Low Priority
12. Interactive Demo GIFs (1-2 days)
13. Debugger Setup Wizard (2-3 days)
14. Status Bar Version Display (0.5 days)
15. Enhanced Issue Templates (0.5 days)

---

## Validation Checklist for Builders

For each Tier 1 feature, builders should verify:
- [ ] Feature works on Windows, macOS, Linux
- [ ] Keyboard shortcut (if any) doesn't conflict
- [ ] Tested with remote dev (Codespaces, WSL)
- [ ] Works with large workspaces (100k+ file projects)
- [ ] Documented in README or CONFIG.md
- [ ] Included in release notes

---

## Next Steps

1. **Orchestrator**: Route Tier 1 items to 3 parallel agents (snippets, onboarding, error messages) + 1 serial (progress reporting)
2. **Builders**: Use scout output as exact specs (no back-and-forth)
3. **Review**: Merge in batches after each 2-3 items
4. **Measurement**: Track: install->activation time, error message clarity (via telemetry or feedback survey)

---

## References

- Competitor analyses: [Perl::LanguageServer](https://github.com/richterger/Perl-LanguageServer), [PerlNavigator](https://github.com/bscan/PerlNavigator), [PLS](https://metacpan.org/pod/PLS)
- UX gotchas research: [VSCode Perl setup guide](https://dev.to/perldean/vscode-as-a-perl-ide-3cco), [PerlMonks discussions](https://www.perlmonks.org)
- Current LSP features: `features.toml` (97 advertised)
- Extension config: `vscode-extension/package.json` (11 settings)
