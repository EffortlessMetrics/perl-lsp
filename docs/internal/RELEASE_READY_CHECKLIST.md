# Release Ready Checklist

Weekly truth source while GitHub Actions are disabled. Each item is either ✅ done or ⬜ blocking.

**Gate receipt**: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh`

---

## Rail A: Workspace Semantics (determinism + configurability)

"Works the same in every editor, every time."

| Status | Requirement |
|--------|-------------|
| ✅ | `initialize` captures workspaceFolders → rootUri → rootPath fallback |
| ✅ | Multi-root folder ordering is stable (in initialization order) |
| ✅ | Module resolution precedence documented in code |
| ✅ | Precedence: open docs → workspace folders → include_paths → system @INC (opt-in) |
| ✅ | `WorkspaceConfig` with `include_paths`, `use_system_inc`, `resolution_timeout_ms` |
| ✅ | `workspace/didChangeConfiguration` updates config without restart |
| ✅ | Regression tests for precedence + legacy rootPath (15 tests) |

**Rail A verdict**: ✅ **Complete** — 3fbdbb32

---

## Rail B: Index Lifecycle (correctness under change)

"Cross-file features stay correct as files churn."

| Status | Requirement |
|--------|-------------|
| ⬜ | Index state machine: building / ready / degraded |
| ⬜ | Handlers degrade gracefully when index is building |
| ✅ | didOpen/didChange/didClose update index incrementally |
| ✅ | File watchers registered for `.pl`, `.pm`, `.t` |
| ⬜ | Bounded caches with eviction (AST cache, symbol cache) |
| ⬜ | Resource cap documented (max files, max symbols) |

**Rail B verdict**: ⬜ **In progress** — state machine + bounded caches needed

---

## Rail C: Latency Budget (predictable responsiveness)

"Never block the editor."

| Status | Requirement |
|--------|-------------|
| ✅ | No scan-under-lock (snapshot discipline) |
| ✅ | Module resolution timeout: 50ms default, configurable |
| ✅ | Filesystem probing has explicit timeouts |
| ⬜ | SLO documented: P95 completion <50ms, P95 definition <30ms |
| ⬜ | Early-exit for large results (e.g., workspace symbol search caps) |

**Rail C verdict**: ⬜ **In progress** — SLO doc + early-exit caps needed

---

## Rail D: Feature Completeness (market-expected LSP set)

"Users don't bounce because something basic is missing."

| Status | Feature |
|--------|---------|
| ✅ | completion |
| ✅ | hover |
| ✅ | signatureHelp |
| ✅ | definition / typeDefinition / implementation |
| ✅ | references + documentHighlight |
| ✅ | rename (single + workspace) |
| ✅ | documentSymbol + workspace symbol search |
| ✅ | formatting (document/range) with perltidy fallback |
| ✅ | semanticTokens |
| ✅ | codeAction (pragmas, extract, imports) |
| ✅ | callHierarchy |
| ✅ | codeLens |

**Rail D verdict**: ✅ **Complete** — ~91% functional per tracking

---

## Rail E: Packaging + Install Story

"Someone installs it without you."

| Status | Requirement |
|--------|-------------|
| ✅ | `cargo install perl-lsp` works |
| ✅ | Binary release documented |
| ✅ | `gate-local.sh` runnable for contributors |
| ⬜ | VS Code setup snippet documented |
| ⬜ | Neovim (nvim-lspconfig) setup snippet documented |
| ⬜ | Emacs (lsp-mode) setup snippet documented |
| ⬜ | Config schema documented (JSON schema or markdown reference) |

**Rail E verdict**: ⬜ **In progress** — editor snippets + config schema needed

---

## Competitive Delta

| Differentiator | perl-lsp | Perl::LanguageServer | Perl Navigator |
|----------------|----------|----------------------|----------------|
| Single binary, no CPAN deps | ✅ | ❌ | ❌ |
| DAP debugger | ✅ (Phase 1 bridge) | ✅ (mature) | ❌ |
| Deterministic parsing | ✅ | ❌ | ❌ |
| System @INC navigation | opt-in | ✅ | ✅ |
| perlcritic integration | ✅ (external + fallback) | ✅ | ✅ |

---

## Next Sprint (2 chunky PRs)

### PR 1: Index Lifecycle v1
- [ ] `IndexState` enum: `Building`, `Ready`, `Degraded`
- [ ] Handlers return degraded responses while building
- [ ] Bounded AST/symbol caches with LRU eviction
- [ ] Resource cap constants in WorkspaceConfig
- [ ] Tests for graceful degradation

### PR 2: Editor Setup + Config Schema
- [ ] `docs/EDITOR_SETUP.md` with VS Code / Neovim / Emacs snippets
- [ ] `docs/CONFIG_SCHEMA.md` or JSON schema for settings
- [ ] SLO documentation in `docs/PERFORMANCE_SLO.md`
- [ ] Early-exit caps for workspace symbol search

---

**Last updated**: 2025-12-29
