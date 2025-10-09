# Issue #207: Debugger DAP Support

## Context

The Perl LSP ecosystem currently lacks integrated debugging capabilities through the Debug Adapter Protocol (DAP). While the LSP server provides comprehensive language features (~91% LSP 3.18 compliance), developers require breakpoint-based debugging workflows to complement the existing diagnostic and code navigation capabilities. **No existing DAP implementation exists in the codebase** - this will be a greenfield implementation leveraging existing Perl LSP infrastructure (AST integration, incremental parsing, workspace navigation, and security framework).

### Affected Components
- **perl-parser** (`/crates/perl-parser/`): Core parser providing AST integration for breakpoint validation and incremental parsing hooks
- **perl-lsp** (`/crates/perl-lsp/`): LSP server binary that will integrate DAP capabilities with LSP coordination
- **vscode-extension** (`/vscode-extension/`): VS Code extension requiring `contributes.debuggers` configuration
- **New crate** (required): `perl-dap` for production-grade native DAP adapter implementation
- **External dependency** (required): CPAN module `Devel::TSPerlDAP` for machine-readable Perl runtime shim

### Perl LSP Workflow Impact
- **Parse Stage**: No direct impact - debugging uses existing AST infrastructure
- **Index Stage**: Breakpoint mapping requires accurate file-to-symbol indexing with cross-file awareness
- **Navigate Stage**: Stack frame navigation leverages existing go-to-definition and workspace symbol infrastructure
- **Complete Stage**: REPL evaluate feature complements existing completion providers with runtime context
- **Analyze Stage**: Debugging diagnostics (breakpoint verification, runtime errors) extend existing diagnostic pipeline

### Performance Implications
- **Breakpoint Verification**: <50ms latency for setBreakpoints requests (target: <100ms p95)
- **Step/Continue Operations**: <100ms response time for continue/next/stepIn/stepOut commands (p95)
- **Variable Expansion**: Lazy loading with <200ms initial scope retrieval and <100ms per child expansion
- **Large Codebase Impact**: Breakpoint indexing must scale to 100K+ LOC files with workspace-aware path mapping
- **Memory Overhead**: <1MB per debug session for adapter state, <5MB for Perl shim process
- **Incremental Parsing**: Breakpoint validation leverages existing <1ms incremental parsing for live breakpoint adjustment

### Enterprise Security Considerations
- **Safe Evaluation**: Default to non-mutating eval mode; require explicit `allowSideEffects: true` opt-in for state-changing expressions
- **Path Traversal Prevention**: Validate all file paths through existing enterprise security framework (see `docs/SECURITY_DEVELOPMENT_GUIDE.md`)
- **Timeout Enforcement**: Hard timeouts on evaluate requests (<5s default) to prevent DoS from infinite loops
- **Privilege Separation**: Perl shim runs with debuggee privileges, adapter runs with minimal permissions

### Technical Constraints
- **Dual Implementation Strategy**: Bridge approach (delegate to Perl::LanguageServer DAP) for immediate availability + native approach (Rust adapter + Perl shim) for production quality
- **Cross-Platform Compatibility**: Windows path normalization (drive letters, UNC paths, CRLF), macOS/Linux symlink handling, WSL path translation
- **Single-Thread Model**: Perl debugger is single-threaded; model as one "main" thread with future fork support as separate "threads"
- **TTY Output Brittleness**: Existing `debug_adapter.rs` scrapes `perl -d` output with regex patterns - this is fragile and incomplete for production
- **Path Mapping Complexity**: URI ↔ filesystem path translation with case-insensitivity awareness, symlink resolution, multi-root workspace support
- **LSP Integration**: DAP adapter must integrate with existing LSP server infrastructure without degrading LSP performance

## User Story

As a **Perl developer using perl-lsp in VS Code**, I want **integrated debugging capabilities through the Debug Adapter Protocol** so that **I can set breakpoints, step through code, inspect variables, and evaluate expressions without switching to external debugging tools or scraping TTY output**.

### Stakeholder Impact
- **Primary Users**: Perl developers using VS Code, Neovim, Emacs, and other DAP-compatible editors
- **Secondary Users**: Enterprise teams requiring integrated debugging workflows for large Perl codebases
- **Tooling Integration**: CI/CD pipelines, automated testing frameworks, remote debugging scenarios

## Acceptance Criteria

### Phase 1: Bridge Implementation (Quick Win - 1-2 days)

**AC1**: VS Code extension contributes a `perl` debugger type that delegates to Perl::LanguageServer's existing DAP implementation
- Extension `package.json` includes `contributes.debuggers` with `type: "perl"`
- Debug adapter launcher resolves Perl path, sets `PERL5LIB`, project `-Ilib`, and script arguments
- Spawns Perl::LanguageServer in DAP mode and proxies stdio communication

**AC2**: Provide VS Code `launch.json` snippets for both launch and attach configurations
- Launch snippet: `perl (launch)` with `program`, `args`, `perlPath`, `includePaths`, `env`, `cwd` parameters
- Attach snippet: `perl (attach tcp)` with `host`, `port`, `pathMapping` parameters
- Snippets validated on Windows, macOS, and Linux with multi-root workspace support

**AC3**: Documentation covers bridge setup: "Using perl-lsp with Perl::LanguageServer DAP"
- Installation instructions for Perl::LanguageServer
- Configuration examples with recommended settings
- Troubleshooting guide for common path mapping issues

**AC4**: Basic debugging workflow functional through bridge
- Set/clear breakpoints in source files
- Continue, step in, step out, step over operations
- Stack trace and local variables visible
- REPL evaluate expressions in current frame context

### Phase 2: Native DAP Infrastructure (3-5 weeks)

**AC5**: Create `perl-dap` Rust crate with JSON-RPC DAP server over stdio
- Implement DAP protocol scaffolding with `initialize`, `launch`, `attach`, `disconnect` requests
- Response times <50ms for initialization, <100ms for launch/attach (p95)
- Thread-safe architecture with `Arc<Mutex<>>` for session state management
- Error handling with `anyhow::Result<T>` and structured error responses per DAP specification

**AC6**: Implement CPAN module `Devel::TSPerlDAP` as machine-readable Perl runtime shim
- JSON server over stdio or TCP (configurable via `-MDevel::TSPerlDAP=daemon,host=127.0.0.1,port=0`)
- Commands: `set_breakpoints`, `continue`, `next`, `step_in`, `step_out`, `pause`, `stack`, `scopes`, `variables`, `evaluate`, `source`
- Uses `PadWalker` for lexicals, `B::Deparse` for code ref previews, `%DB::sub` + `caller` for stack traces
- Unit tests with >80% code coverage and integration tests with real Perl scripts

**AC7**: Breakpoint management with accurate path mapping
- `setBreakpoints` request sets/clears breakpoints in source files
- Path mapping handles Windows drive letters, UNC paths, symlinks, and case-insensitive filesystems
- Breakpoint verification confirms line validity (not comment-only, blank, or invalid locations)
- Breakpoints survive file edits with incremental parsing integration (<1ms updates)

**AC8**: Stack trace, scopes, and variables with lazy expansion
- `threads` request returns one "Main Thread" (Perl single-thread model)
- `stackTrace` request returns accurate frames using `caller()` + `%DB::sub` data
- `scopes` request provides "Locals" (PadWalker), "Package" (symbol table), "Globals" (optional)
- `variables` request renders scalars/arrays/hashes with lazy child expansion (<100ms per expansion)
- Variable values use `B::Deparse` for code refs and truncate large data with "…" (max 1KB preview)

**AC9**: Stepping and control flow operations
- `continue`, `next`, `stepIn`, `stepOut`, `pause` requests map to `$DB::single`/`$DB::trace` and Perl debugger commands
- Response latency <100ms p95 on sample scripts (100-1000 lines)
- Pause operation sends interrupt signal (SIGINT on Unix, Ctrl+C on Windows) with <200ms response
- Step operations filter internal frames (shim frames, debugger infrastructure) from user-visible stack

**AC10**: Evaluate and REPL functionality
- `evaluate` request evaluates expressions in selected stack frame context
- Safe evaluation mode (non-mutating) by default; requires `allowSideEffects: true` for state changes
- Timeout enforcement (5s default, configurable) prevents infinite loops and DoS
- Hover evaluate integration shows expression values on hover (LSP + DAP coordination)

**AC11**: VS Code extension native DAP integration
- `contributes.debuggers` with `type: "perl-rs"` for native adapter
- Adapter binary `perl-dap` bundled with extension or auto-downloaded per platform
- Launch configuration: `request: "launch"`, `program`, `args`, `perlPath`, `includePaths`, `env`, `cwd`
- Attach configuration: `request: "attach"`, `host`, `port`, `pathMapping` for remote debugging

**AC12**: Cross-platform compatibility validated
- Windows: CRLF handling, drive letter normalization, UNC path support, WSL path translation
- macOS/Linux: Symlink resolution, case-sensitive path handling, UNIX signal handling
- Platform-specific adapter binaries for x86_64/aarch64 Linux/macOS/Windows

### Phase 3: Production Hardening & Testing (2 weeks)

**AC13**: Comprehensive integration tests with golden transcripts
- Test fixtures: `hello.pl` (basic), `args.pl` (command-line args), `eval.pl` (dynamic code)
- Golden DAP transcript validation: request/response sequences match expected protocol flows
- Breakpoint matrix tests: file start/end, blank lines, comment lines, heredocs, BEGIN/END blocks
- Variable rendering tests: scalars, arrays, hashes, deep nesting, Unicode, large data (>10KB)
- **Validation**: `cargo test -p perl-dap --test integration_tests` (>95% coverage target)

**AC14**: Performance benchmarks establish baselines
- Benchmark suite: small (100 lines), medium (1000 lines), large (10K+ lines) Perl scripts
- Metrics: setBreakpoints latency, step/continue response times, variable expansion latency, memory overhead
- Regression tests prevent performance degradation (CI/CD integration with `cargo bench`)
- **Validation**: `cargo bench -p perl-dap` (baselines: <50ms breakpoints, <100ms step/continue p95)

**AC15**: Documentation and examples complete
- Tutorial: "Getting Started with Perl DAP Debugging" with step-by-step screenshots
- Reference: "DAP Configuration Options" with all launch.json parameters documented
- Architecture: "DAP Implementation Design" explaining Rust adapter + Perl shim architecture
- Troubleshooting: Common issues (path mapping failures, breakpoint verification errors, timeout handling)
- **Validation**: `cargo test --test dap_documentation_complete` (Diátaxis framework compliance)

**AC16**: Security validation against enterprise standards (NEW)
- Path traversal prevention: Validate all file paths through existing enterprise security framework (`docs/SECURITY_DEVELOPMENT_GUIDE.md`)
- Safe evaluation enforcement: Default to non-mutating eval mode with explicit `allowSideEffects` opt-in (AC10 compliance)
- Timeout enforcement: Hard timeouts on evaluate requests (<5s default, configurable) preventing DoS
- Unicode boundary safety: Reuse PR #153 symmetric position conversion for variable rendering and breakpoint mapping
- **Validation**: `cargo test -p perl-dap --test security_validation` (zero security findings in CI/CD gate)

**AC17**: LSP integration non-regression testing (NEW)
- Full LSP test suite validation: Run comprehensive LSP tests with DAP adapter active
- Performance validation: Verify <50ms LSP response time targets maintained with DAP loaded
- Memory leak detection: No resource contention between LSP and DAP protocol handling
- Protocol separation: Clean routing between LSP (`textDocument/*`) and DAP (`debug*`) methods
- **Validation**: `cargo test -p perl-lsp --test lsp_dap_non_regression` (100% LSP test pass rate with DAP)

**AC18**: Dependency management and installation strategy (NEW)
- CPAN module publication: `Devel::TSPerlDAP` published to CPAN with >80% test coverage
- Auto-install workflow: `perl-dap --install-shim` runs `cpanm Devel::TSPerlDAP` on first use
- Bundled fallback: Extension bundles `Devel/TSPerlDAP.pm` in resources/ for CPAN installation failures
- Versioning strategy: Adapter ↔ shim protocol versioning with feature detection in `initialize` response
- Perl compatibility: Validate Perl 5.16+ compatibility (5.30+ recommended)
- **Validation**: `cargo test --test dap_dependency_installation` (<30 second first-time setup target)

**AC19**: Binary packaging and cross-platform distribution (NEW)
- Platform binaries: Build `perl-dap` for x86_64/aarch64 Linux/macOS/Windows (6 platforms)
- GitHub Releases strategy: Automated builds via GitHub Actions with versioned releases
- VS Code extension packaging: Bundle platform-specific binaries or auto-download from GitHub Releases
- First-launch optimization: <5 second download time per platform with fallback to bundled binary
- **Validation**: `cargo test --test dap_binary_packaging` (validate all platform binaries loadable)

## Technical Implementation Notes

### Affected Crates
- **perl-parser** (`/crates/perl-parser/`): Breakpoint validation integration, incremental parsing hooks for live breakpoint adjustment
- **perl-lsp** (`/crates/perl-lsp/`): DAP server integration, LSP + DAP coordination for hover evaluate
- **perl-dap** (new crate): Native DAP adapter implementation with Rust-based JSON-RPC server
- **vscode-extension** (`/vscode-extension/`): Debugger contribution, launch.json snippets, platform binary management

### LSP Workflow Stages
- **Parsing**: Existing AST used for breakpoint line validation and source mapping
- **Indexing**: Workspace symbol index used for breakpoint file resolution and multi-root support
- **Navigation**: Stack frame navigation leverages go-to-definition for "jump to frame" functionality
- **Completion**: REPL evaluate can leverage completion providers for expression assistance
- **Analysis**: Breakpoint diagnostics extend existing diagnostic pipeline (verify, invalid, unbound)

### Performance Considerations
- **Incremental Parsing**: Leverage existing <1ms incremental parsing for breakpoint validation on file edits
- **LSP Response Times**: DAP operations must not degrade LSP server response times (<50ms target)
- **Memory Usage**: Debug session state <1MB, Perl shim process <5MB, total overhead <10MB per session
- **Large Codebase Scaling**: Breakpoint indexing must handle 100K+ LOC files efficiently with workspace-aware caching
- **Concurrent Sessions**: Support multiple debug sessions without resource contention (session isolation)

### Parsing Requirements
- **Breakpoint Validation**: AST-based line validation to prevent breakpoints on invalid lines (comments, blank, heredoc interiors)
- **Source Mapping**: UTF-16 ↔ UTF-8 position conversion for accurate breakpoint placement (LSP uses UTF-16 offsets)
- **Incremental Updates**: Live breakpoint adjustment as code changes without full re-parse (<1ms target)
- **Syntax Coverage**: ~100% Perl syntax coverage ensures accurate breakpoint placement in all language constructs

### Cross-File Navigation
- **Path Mapping**: Dual indexing strategy supports qualified (`Package::function`) and bare (`function`) names
- **Workspace Navigation**: Multi-root workspace support for breakpoints across project boundaries
- **Symlink Resolution**: Platform-aware symlink resolution for accurate file identity (macOS/Linux)
- **URI Normalization**: Consistent URI ↔ filesystem path translation (Windows drive letters, UNC paths)

### Protocol Compliance
- **DAP Specification**: Implement core DAP 1.x protocol (initialize, launch, attach, breakpoints, stepping, variables, evaluate)
- **JSON-RPC 2.0**: Proper message framing with `Content-Length` headers (existing LSP infrastructure reusable)
- **Error Handling**: Structured error responses with actionable error messages per DAP specification
- **Event Sequencing**: Proper event ordering (initialized → stopped → continued → terminated) with seq number tracking

### Tree-sitter Integration
- **AST Integration**: Leverage existing tree-sitter AST for source mapping and breakpoint validation
- **Highlight Testing**: Validate breakpoint UI highlighting via `cd xtask && cargo run highlight` (optional enhancement)

### Testing Strategy
- **TDD with `// AC:ID` Tags**: Each acceptance criterion maps to test functions with `// AC1`, `// AC2`, etc.
- **Parser/LSP/Lexer Smoke Testing**: Validate DAP integration doesn't regress existing parser/LSP functionality
- **LSP Protocol Compliance**: Integration tests validate DAP messages conform to specification
- **Benchmark Baseline Establishment**: Performance benchmarks for setBreakpoints, step/continue, variable expansion
- **Golden Transcript Tests**: Record/replay DAP message sequences for regression detection
- **Cross-Platform CI**: Validate Windows/macOS/Linux compatibility in CI pipeline

### Implementation Phases

**RECOMMENDED STRATEGY: Phased Bridge-to-Native Approach**

This phased approach provides immediate user value while mitigating implementation risk:

#### Phase 1: Bridge Implementation (Week 1-2, Quick Win)
- **Goal**: Immediate debugging capability for users without backend development
- **Deliverable**: VS Code extension delegates to Perl::LanguageServer DAP (AC1-AC4)
- **Timeline**: 1-2 days implementation
- **Risk**: **LOW** - Dependent on external project; limited customization for perl-lsp workflows
- **User Value**: Debugging available immediately; gather user feedback on DAP feature priorities

#### Phase 2: Native Infrastructure (Week 3-6, Core Capability)
- **Goal**: Production-grade DAP adapter owned by perl-lsp project
- **Deliverable**: Rust `perl-dap` crate + CPAN `Devel::TSPerlDAP` shim (AC5-AC12)
- **Timeline**: **3-5 weeks** (corrected from original 2-4 weeks due to greenfield implementation)
- **Critical Path**: AC6 (Perl shim) requires 2 weeks for robust debugger integration
- **Risk**: **MODERATE-HIGH** - Complexity of Perl debugger integration; cross-platform compatibility challenges
- **Migration**: Bridge remains as fallback during native development

#### Phase 3: Production Hardening (Week 7-8, Quality Assurance)
- **Goal**: Enterprise-ready debugging with comprehensive testing and documentation
- **Deliverable**: Integration tests, performance benchmarks, user documentation, security validation (AC13-AC19)
- **Timeline**: 2 weeks (increased from 1 week to include AC16-AC19 new requirements)
- **Risk**: **LOW-MODERATE** - Edge cases in variable rendering, path mapping, timeout handling, and LSP integration
- **Quality Gates**: Security validation (AC16), LSP non-regression (AC17), dependency management (AC18), binary packaging (AC19)

**Total Timeline: 8 weeks** (2 weeks faster than pure native approach, with immediate user value in week 1-2)

### Recommended Technology Stack
- **Rust**: `tokio` (async runtime), `serde_json` (JSON serialization), `anyhow` (error handling), `tracing` (logging)
- **Perl**: `PadWalker` (lexical inspection), `B::Deparse` (code ref decompilation), `IO::Socket::INET` (TCP), `perl5db` (core debugger)
- **Packaging**: VS Code extension downloads platform binaries (Linux/macOS/Windows x86_64/aarch64); CPAN shim as dependency or bundled fallback

### Risk Mitigation
- **Source Filters / eval**: Safe evaluation mode prevents unintended state changes; explicit opt-in required for side effects
- **Forks / Multi-Process**: Model as multiple "threads" in future; out of MVP scope
- **Remote Debugging**: TCP attach support deferred until local debugging is stable
- **mod_perl**: Complex environment; deferred to post-MVP (future enhancement)
- **Performance**: Benchmarking and profiling integrated into CI to prevent regression

### Known Limitations (MVP Scope)
- **Single-Thread Model**: Perl debugger is single-threaded; fork/thread support deferred
- **Conditional Breakpoints**: DAP protocol supports them, but implementation deferred to Phase 2
- **Exception Breakpoints**: Deferred to post-MVP (requires Perl exception hook integration)
- **Watchpoints**: Data breakpoints not in MVP scope
- **Remote Debugging**: Local debugging only in MVP; TCP attach in Phase 2
- **Hot Code Reload**: Not supported in MVP (requires eval-based patching)

### Success Metrics
- **Functional**: All **19 acceptance criteria** (AC1-AC19) validated with >95% test pass rate
- **Performance**: <100ms p95 latency for step/continue, <50ms for breakpoint operations
- **Quality**: >80% code coverage for DAP adapter and Perl shim, >95% for integration tests
- **Security**: Zero security findings in CI/CD security scanner gate (AC16)
- **LSP Integration**: 100% LSP test pass rate with DAP adapter active (AC17)
- **Adoption**: >100 VS Code extension users successfully debugging Perl code within 4 weeks of release
- **Cross-Platform**: Validated on 6 platforms (x86_64/aarch64 Linux/macOS/Windows) (AC19)
