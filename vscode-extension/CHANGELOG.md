# Change Log

All notable changes to the Perl Language Server extension will be documented in this file.

## [0.12.4] - 2026-04-12

### Added
- **DAP debugger launch scorecard**: the debug adapter now tracks cold-launch
  success rate and P50/P95 latency across a suite of fixture programs; results
  surface in the new `status/dap.md` page. (#4237)
- **Inherited and role method navigation**: `Go to Definition`, hover, and
  workspace completion now traverse Moo/Moose `with 'Role'` and
  `extends`/`use parent` chains. AUTOLOAD-backed method calls also resolve.
  (#4077, #4091)
- **Phase-scoped pragma diagnostics** (`PL502`, `PL503`): flags `use strict` /
  `use warnings` placed inside phase blocks (`BEGIN`, `END`, etc.) where they
  have block scope rather than file scope, with quick-fixes that move them to
  file scope. (#4131)
- **Dynamic workspace configuration remains deferred**: per-folder
  `.perl-lsp.toml` remains the supported v0.13 mechanism; fully dynamic
  per-folder scoping via `workspace/configuration` reverse requests is tracked
  as deferred follow-up work. (#3515)
- **`workspace/willDeleteFiles` warnings**: deleting a file that is referenced
  by other files in the workspace now surfaces a `Warning` diagnostic before
  the delete completes. (#4056)
- **Run Test at Cursor**: new command palette entry runs the nearest test
  subroutine or subtest under the cursor without navigating to a test lens.
  (#4025)
- **`yath` test runner preference**: the VS Code test runner uses `yath` when
  present on PATH, falling back to `prove` and `perl`. (#4031)

### Fixed
- **Rename operations are faster on large files**: the internal scope traversal
  was O(n×d) per rename; replaced with a single-pass BFS so renames on deeply
  nested files no longer stall. (#4240)
- **Windows: Run Tests / Run File no longer fail** with extended-length path
  errors — the `\\?\` prefix injected by `Path::canonicalize` is stripped
  before spawning `perl`, `prove`, or `yath`. (#4089)
- **`use if` / `use unless` conditional pragmas** no longer produce false
  missing-strict/warnings diagnostics. (#4050)
- **Eval- and sub-scoped pragmas** no longer suppress file-level PL100/PL101
  missing strict/warnings diagnostics. (#4052)
- **Non-`file://` workspace roots** (`vscode-remote://`, virtual schemes) are
  now tolerated — the server keeps them as LSP strings and skips non-filesystem
  folders during indexing without crashing. (#4059)

### Changed
- **Pre-push hook speed**: the hook now runs the fast Tier A (`pr-fast`) check
  instead of the full `ci-gate`, making pre-push validation ~3× faster for
  routine pushes. (#4088, #4110)

## [0.12.3] - 2026-04-09

The 0.12.3 release line. Aligned with the workspace `v0.12.3` cut and consolidates
the user-facing UX hardening work from April 2026.

### Fixed
- **Actionable error messages**: server startup errors now surface specific,
  actionable messages instead of a generic "corrupted" fallback
  (PR #3308, PR #3291).
- **Perl interpreter detection**: distinct error shown when the Perl interpreter
  is not found in PATH, with a clear install prompt (PR #3312).
- **Binary download errors**: download failures include the underlying OS error
  and a retry hint instead of a silent failure (PR #3306).
- **Settings schema polish**: extension settings entries cleaned up for the
  VS Code Settings UI — descriptions, defaults, and enum labels improved
  (PR #3269).
- **Enterprise/offline deployment**: documentation added for air-gapped binary
  distribution and internal mirror configuration (PR #3310).

## [0.12.2] - 2026-04-04

The 0.12.2 release line. Aligned with the workspace `v0.12.2` cut on 2026-04-04
and consolidates the v0.12.2 through v0.12.8 milestones into a single release.

### Added
- **AI inline completion**: opt-in OpenAI-compatible streaming provider with
  session management, version-aware cancellation, and a deterministic fallback
  when AI is disabled. Wired into the LSP server's `inlineCompletion` request
  via the new `experimental.perlInlineCompletionStream` extension.
- **Heredoc language injection**: SQL keyword and JSON key detection in
  heredocs with multi-heredoc-per-line support.
- **Type inference in hover**: type hints rendered alongside symbol info.
- **Dead code highlighting**: `DiagnosticTag::Unnecessary` for unreachable code.
- **Refactoring code actions**: extract variable, extract subroutine, inline
  subroutine, scoped rename.
- **POD preview panel**: `Perl: Preview POD` command opens a side panel with
  formatted POD output.
- **AST explorer debug panel**: `perl/showAst` custom LSP handler for
  inspecting parser output during diagnostics.

### Performance
- Incremental parsing pipeline (token caching + checkpoint recovery).
- Large-workspace HashMap optimization (faster startup).
- CPAN-scale benchmarks: 10K files indexed in 672ms.

### Fixed
- Inline completion duplicate backend types removed.
- Streaming completion controller gating on AI config flags.

### Changed
- **Version Bump**: 0.12.2 release aligned with the workspace and shipped
  `perllsp` asset line.

## [0.12.1] - 2026-03-30

### Fixed
- **Release Surface Recovery**: Restored the top-level source snapshot and release-facing docs after the launch regressions that slipped into the first `0.12.0` tag.
- **Hook Hygiene**: Hardened hook-test fixture isolation and worktree hook installation so placeholder identities do not leak into normal local commit flows.

### Changed
- **Version Bump**: 0.12.1 fix-forward release aligned with the workspace and shipped `perllsp` asset line.

## [0.12.0] - 2026-03-19

### Changed
- **Display Name**: Updated to "Perl Language Server (perl-lsp)" for clearer marketplace identification
- **Description**: Rewritten for marketplace SEO: highlights key features, speed, and zero-dependency install
- **Version Bump**: 0.12.0 public alpha release
- **Preview Flag**: Marked as preview for public alpha period
- **Keywords**: Added `debugger`, `refactoring`, `code-completion`, `diagnostics` for marketplace discovery
- **Categories**: Added "Testing" to reflect Test Explorer integration

### Added
- **Open VSX Publishing**: Extension now publishes to Open VSX Registry alongside VS Marketplace, enabling first-class support for VSCodium and other open-source VS Code derivatives. Added `ovsx` publish/check scripts and `@types/vscode` dev dependency fix for compatible builds.
- **Comprehensive README**: Rewritten for marketplace listing with full configuration reference table, keyboard shortcuts, troubleshooting guide, and command reference

## [0.11.0] - 2026-03-11

### Fixed
- **Corrupted Extension Icon**: Added binary rules for image files in `.gitattributes` to prevent line-ending normalization from corrupting PNGs; regenerated `icon.png` with correct PNG signature header.
- **TextMate Grammar Registration**: Registered `syntaxes/perl.tmLanguage.json` in `contributes.grammars` so VS Code actually loads the bundled grammar.
- **Stub Refactoring Commands**: Removed `Extract Subroutine`, `Extract Variable`, and `Inline Variable` from editor context and command palette menus (still available from the full command list with "in development" messaging).
- **VSCE Packaging**: Removed `--no-dependencies` flag so runtime dependencies (`vscode-languageclient`, `adm-zip`, `tar`) are properly bundled in the `.vsix`.
- **pnpm Compatibility**: Made `vsce package`/`publish` resilient in pnpm environments.

### Changed
- **Parser Bug Fixes** (6 fixes in the underlying `perl-lsp` server):
  - Handle postfix modifiers after dereference expressions (`$obj->method if $cond`)
  - Re-lex slash as regex delimiter after `split` keyword
  - Accept fat arrow as argument separator in function calls
  - Handle subscripts on package-qualified variables (`$Foo::bar[0]`)
  - Support `&{expr}` code dereference syntax
  - Expand builtin list and support forward declarations
- **Lexer Bug Fixes** (2 fixes):
  - Skip POD blocks in whitespace/comment handler
  - Make POD scanning byte-safe for UTF-8 source files
- **Regex Engine Fixes** (2 fixes):
  - Eliminate false positive nested quantifier detection
  - Clean up false positive regex detection
- **LSP Feature Polish**: Maintained 100% user-visible feature coverage and protocol compliance.
- **Documentation**: Comprehensive release readiness updates including README, roadmap, and launch article series.

### Added
- Marketplace readiness workflow via `npm run verify:marketplace` for compile + bundle + package validation.
- VS Marketplace badges and installation link in extension README.
- Publishing guide refreshed with launch checklist and pre-release recommendation.
- SRP microcrate extraction campaign: 30+ new single-responsibility microcrates extracted.
- Comprehensive unit test campaign: 80+ new test modules across all crate tiers.

## [0.10.0] - 2026-02-28

### Changed
- **Version Sync**: Extension version aligned with workspace v0.10.0.
- **LSP Coverage**: Maintained 100% user-visible feature coverage (53/53).
- **Protocol Compliance**: Maintained 100% protocol compliance (97/97).

## [0.9.0] - 2026-01-18

### Added
- 🔧 **Advanced Refactoring Support**
  - Extract method refactoring with parameter detection
  - Inline variable/expression refactoring
  - Move code refactoring for relocating code blocks
  - Transactional safety with rollback infrastructure
- 🎯 **Semantic Definition Integration**
  - Precise go-to-definition using semantic analysis instead of text search
  - Multi-symbol support: scalars, arrays, hashes, subroutines, packages
  - Lexical scoping with proper handling of nested scopes and shadowed variables
- 🔒 **Security Hardening**
  - Complete path traversal protection for execute commands
  - Command injection hardening in executeCommand
- ⚡ **Performance Optimizations**
  - O(1) symbol lookups (from linear time)
  - Stack-based scope analysis for improved performance
  - Reduced string allocations in parser
- 🎨 **Product Icons**: Added icons to extension commands
- 📋 **Context Menu**: Run Tests exposed in editor context menu

### Changed
- Cross-file Package->method resolution improved
- Better error logging for incremental document changes
- Configuration setting descriptions improved

## [0.8.0] - 2025-09-01

### Added
- **Cross-File Navigation**: Workspace indexing with dual storage pattern for qualified and bare names
- **Import Optimization**: Detect and organize imports, remove unused imports
- **Incremental Parsing V2**: Advanced edit tracking with node reuse for faster re-parsing
- **File Path Completion**: Enterprise-grade file completion with security safeguards

### Changed
- Optimized workspace indexing for large codebases
- Enhanced comment documentation extraction for hover

## [0.7.0] - 2025-08-24

### Added
- **LSP 3.17 Features**: Inlay hints, document links, selection ranges, on-type formatting
- **Code Actions**: Robust refactoring and quick fixes
- **Type Hierarchy**: View inheritance relationships
- **Rename Support**: Symbol renaming with validation

## [0.6.0] - 2025-01-29

### Added
- 🔍 **Call Hierarchy Support**
  - View incoming calls (functions that call the selected function)
  - View outgoing calls (functions called by the selected function)
  - Navigate complex call chains with ease
  - Right-click any function and select "Show Call Hierarchy"
- 💡 **Inlay Hints**
  - Parameter name hints for function calls
  - Type hints for variable declarations
  - Smart filtering to reduce visual clutter
  - Fully configurable via settings
- 🧪 **Test Explorer Integration**
  - Automatic discovery of test files (.t) and test functions
  - Visual test hierarchy in Testing panel
  - Run individual tests or entire test files
  - Real-time test results with pass/fail indicators
  - TAP (Test Anything Protocol) support
- 🐛 **Debug Adapter Protocol Support**
  - Full step-through debugging for Perl scripts
  - Breakpoints with conditional support
  - Variable inspection and watch expressions
  - Call stack navigation
  - Test debugging integration
  - Debug configurations for scripts and tests
- ⚡ **Performance Optimizations**
  - AST caching for faster parsing (100 files, 5-min TTL)
  - Symbol index for instant workspace searches
  - 10x faster symbol lookup in large projects

### Enhanced
- Added "Testing" category to extension capabilities
- Improved activation events for test files
- Better TypeScript types and error handling

### Fixed
- Improved handling of anonymous subroutines in navigation features
- Better error recovery for malformed syntax
- Fixed race conditions in document synchronization

## [0.5.0] - 2025-01-01

### Added
- Initial release of Perl Language Server for Visual Studio Code
- Full Language Server Protocol support with 8 core features:
  - Real-time syntax diagnostics
  - Code completion with context awareness
  - Go to definition
  - Find all references
  - Document symbols (outline)
  - Signature help
  - Hover information
  - Code actions (quick fixes)
- Code formatting with Perl::Tidy integration
  - Format document (Shift+Alt+F)
  - Format selection
  - Automatic .perltidyrc discovery
- Enhanced syntax highlighting
- Commands:
  - Restart Language Server
  - Show Language Server Output
- Bundled perl-lsp binary for easy installation
- Support for modern Perl features (try/catch, signatures, class/method)
