# Issue #207: DAP Support - Comprehensive Specification Analysis

**Agent**: spec-analyzer (Generative Flow)
**Date**: 2025-10-03
**Status**: Requirements Validation Complete → Architecture Recommendation Ready
**Flow**: generative:gate:spec → FINALIZE → spec-finalizer

---

## Executive Summary

Issue #207 proposes adding Debug Adapter Protocol (DAP) support to Perl LSP, enabling integrated debugging capabilities for Perl developers. This analysis validates the 15 atomic acceptance criteria organized in 3 phases and provides comprehensive technical recommendations for implementation strategy.

**Key Findings**:
- ✅ **Specification Quality**: Comprehensive requirements with 15 testable acceptance criteria
- ✅ **Architecture Alignment**: Integrates cleanly with existing Perl LSP infrastructure
- ⚠️ **Implementation Risk**: Native approach requires significant Perl debugger integration complexity
- ✅ **Recommended Strategy**: **Phased Implementation** (Bridge → Native → Production)

**Recommended Implementation Path**: **Phased Bridge-to-Native Strategy**
- **Phase 1 (Week 1-2)**: Bridge implementation for immediate user value (AC1-AC4)
- **Phase 2 (Week 3-6)**: Native Rust adapter + Perl shim infrastructure (AC5-AC12)
- **Phase 3 (Week 7-8)**: Production hardening and comprehensive testing (AC13-AC15)

---

## 1. Requirements Validation Summary

### 1.1 Acceptance Criteria Completeness Assessment

**Overall Quality**: ✅ **EXCELLENT** - Comprehensive requirements with clear testability

#### Phase 1: Bridge Implementation (AC1-AC4)
| AC | Requirement | Testability | Completeness | Gap Analysis |
|---|---|---|---|---|
| **AC1** | VS Code debugger contribution | ✅ High | ✅ Complete | None - clear JSON schema requirements |
| **AC2** | Launch.json snippets | ✅ High | ✅ Complete | None - platform-specific validation specified |
| **AC3** | Bridge setup documentation | ✅ Medium | ✅ Complete | Consider adding migration path to native |
| **AC4** | Basic debugging workflow | ✅ High | ⚠️ Needs refinement | **GAP**: Missing specific test fixtures and golden transcripts |

**Phase 1 Gap Analysis**:
1. **AC4 Enhancement Needed**: Add specific test fixtures for bridge validation
   - Suggested fixtures: `bridge_basic.pl`, `bridge_breakpoints.pl`, `bridge_variables.pl`
   - Golden transcript validation for Perl::LanguageServer protocol compatibility
   - Cross-platform test matrix (Windows/macOS/Linux)

#### Phase 2: Native DAP Infrastructure (AC5-AC12)
| AC | Requirement | Testability | Completeness | Gap Analysis |
|---|---|---|---|---|
| **AC5** | perl-dap crate scaffolding | ✅ High | ✅ Complete | None - clear protocol requirements |
| **AC6** | Devel::TSPerlDAP CPAN module | ✅ High | ⚠️ Needs clarification | **GAP**: Missing CPAN publication timeline and dependency management |
| **AC7** | Breakpoint management | ✅ High | ✅ Complete | None - comprehensive path mapping requirements |
| **AC8** | Stack/scopes/variables | ✅ High | ✅ Complete | None - lazy expansion strategy specified |
| **AC9** | Stepping and control flow | ✅ High | ✅ Complete | None - performance targets clear |
| **AC10** | Evaluate and REPL | ✅ High | ✅ Complete | None - security requirements specified |
| **AC11** | VS Code native integration | ✅ High | ✅ Complete | None - binary distribution strategy clear |
| **AC12** | Cross-platform compatibility | ✅ High | ⚠️ Needs refinement | **GAP**: Missing WSL-specific test requirements |

**Phase 2 Gap Analysis**:
1. **AC6 Enhancement Needed**: Clarify CPAN module publication strategy
   - Who maintains `Devel::TSPerlDAP` long-term?
   - Bundled fallback vs external dependency?
   - Versioning strategy for adapter ↔ shim compatibility?

2. **AC12 Enhancement Needed**: Add WSL-specific validation requirements
   - WSL1 vs WSL2 path translation differences
   - Windows ↔ Linux path mapping validation
   - Performance implications of WSL filesystem access

#### Phase 3: Production Hardening (AC13-AC15)
| AC | Requirement | Testability | Completeness | Gap Analysis |
|---|---|---|---|---|
| **AC13** | Comprehensive integration tests | ✅ High | ✅ Complete | None - fixture matrix specified |
| **AC14** | Performance benchmarks | ✅ High | ✅ Complete | None - baseline metrics clear |
| **AC15** | Documentation complete | ✅ Medium | ✅ Complete | None - Diátaxis framework alignment |

**Phase 3 Assessment**: No significant gaps identified. All requirements align with Perl LSP testing standards.

### 1.2 Missing Requirements Identified

**Critical Additions Required**:

1. **AC16 (NEW - Security Validation)**:
   - **Requirement**: Comprehensive security validation against enterprise standards
   - **Test Coverage**: Path traversal prevention, safe eval enforcement, timeout handling
   - **Validation**: Integrate with existing `docs/SECURITY_DEVELOPMENT_GUIDE.md` practices
   - **Success Criteria**: Zero security findings in CI/CD security scanner gate

2. **AC17 (NEW - LSP Integration Non-Regression)**:
   - **Requirement**: Validate DAP integration doesn't degrade existing LSP functionality
   - **Test Coverage**: Run full LSP test suite with DAP adapter active
   - **Performance**: Verify <50ms LSP response time targets maintained
   - **Success Criteria**: 100% LSP test pass rate with DAP binary loaded

3. **AC18 (NEW - Dependency Management)**:
   - **Requirement**: Document and validate `Devel::TSPerlDAP` installation strategy
   - **Scenarios**: Auto-install via cpanm, bundled fallback, system package
   - **Compatibility**: Perl 5.16+ compatibility validation
   - **Success Criteria**: <30 second first-time setup on fresh system

4. **AC19 (NEW - VS Code Extension Packaging)**:
   - **Requirement**: Platform-specific binary distribution for `perl-dap` adapter
   - **Platforms**: x86_64/aarch64 Linux/macOS/Windows binaries
   - **Distribution**: GitHub Releases vs VSIX bundling vs auto-download
   - **Success Criteria**: <5 second first-launch download time per platform

### 1.3 Cross-Platform Compatibility Requirements

**Current Coverage**: ✅ **GOOD** - AC12 addresses most scenarios

**Additional Validation Needed**:
- **Windows**: UNC path support (`\\server\share\file.pl`), drive letter normalization, CRLF handling
- **macOS**: Symlink resolution for `/tmp`, `~/Library`, case-insensitive filesystem issues
- **Linux**: SELinux/AppArmor policy compatibility, `~/.local/bin` PATH detection
- **WSL**: WSL1 vs WSL2 path translation, `/mnt/c` filesystem performance implications

**Recommended Enhancement**: Add comprehensive cross-platform test matrix to AC12

### 1.4 Security Requirements Validation

**Current Coverage**: ✅ **EXCELLENT** - Comprehensive security considerations

**Existing Security Measures**:
- ✅ Safe evaluation mode with explicit `allowSideEffects` opt-in (AC10)
- ✅ Path traversal prevention via enterprise security framework (spec references)
- ✅ Timeout enforcement (5s default) preventing DoS (AC10)
- ✅ Privilege separation (shim runs with debuggee privileges) (spec notes)

**Alignment with `docs/SECURITY_DEVELOPMENT_GUIDE.md`**:
- ✅ **UTF-16 Position Security**: Reuse existing symmetric position conversion (PR #153)
- ✅ **LSP Error Recovery**: Apply existing secure logging patterns (Issue #144)
- ✅ **Path Security**: Leverage existing canonical path validation
- ✅ **Unicode Security**: Reuse existing boundary validation for variable rendering

**Additional Security Recommendations**:
1. **Add Security Test Suite** (AC16 NEW):
   - Fuzz testing for eval expressions (prevent code injection)
   - Path traversal attack validation (malicious source paths)
   - Timeout enforcement validation (infinite loop prevention)
   - Memory exhaustion testing (large variable rendering)

2. **Integrate with Existing Security Infrastructure**:
   ```rust
   // Reuse existing security primitives from perl-parser
   use perl_parser::security::{validate_path, sanitize_eval_expression};

   // Apply to DAP implementation
   fn evaluate_expression(expr: &str, allow_side_effects: bool) -> Result<Value> {
       let sanitized = sanitize_eval_expression(expr, allow_side_effects)?;
       // ... evaluation logic
   }
   ```

---

## 2. Technical Feasibility Analysis

### 2.1 Existing Implementation Assessment

**Finding**: ❌ **NO EXISTING DAP IMPLEMENTATION FOUND**

**Search Results**:
```bash
$ find crates -name "debug_adapter.rs" -o -name "*debug*.rs"
crates/perl-lexer/examples/debug_lexer.rs          # Lexer debugging tool
crates/perl-lexer/src/test_format_debug.rs         # Test utilities
crates/tree-sitter-perl-rs/examples/debug_parse.rs # Parser debugging
# NO DAP protocol implementation found
```

**Implication**: The spec reference to `/crates/perl-parser/src/debug_adapter.rs` (1406 lines) is **INCORRECT**. No existing DAP scaffolding exists.

**Impact on Timeline**:
- ✅ **Bridge approach** remains 1-2 days (no change)
- ⚠️ **Native approach** increases from 2-4 weeks to **3-5 weeks** (no starting point)

**Recommended Correction**: Update Issue #207 spec to remove references to existing implementation

### 2.2 Perl LSP Architecture Integration Points

**Existing Infrastructure Reusable for DAP**:

#### 2.2.1 JSON-RPC Protocol Infrastructure ✅ **EXCELLENT REUSE**

**Current LSP Server Implementation** (`crates/perl-lsp/src/main.rs`):
```rust
// Existing JSON-RPC infrastructure from perl-parser crate
use perl_parser::{JsonRpcRequest, LspServer};

// DAP can reuse same protocol handling with different schema
struct DapAdapter {
    server: DapServer,  // Similar to LspServer architecture
}
```

**Reusable Components**:
- ✅ **Message framing**: Existing `Content-Length` header parsing
- ✅ **Error handling**: Structured error response patterns
- ✅ **Async runtime**: Can extend existing tokio integration
- ✅ **Position mapping**: UTF-16 ↔ UTF-8 conversion infrastructure (PR #153)

**Integration Strategy**:
1. **Dual-Protocol Server**: Single binary handles both LSP and DAP requests
2. **Shared Infrastructure**: Reuse `JsonRpcRequest` parsing, error handling, logging
3. **Separate Routing**: Route to `LspServer` or `DapServer` based on method prefix

**Example Architecture**:
```rust
// crates/perl-lsp/src/main.rs (enhanced)
fn main() {
    let mut lsp_server = LspServer::new();
    let mut dap_server = DapServer::new();  // NEW

    loop {
        let request = read_json_rpc_request()?;

        let response = if request.method.starts_with("textDocument/") {
            lsp_server.handle_request(request)  // Existing
        } else if request.method.starts_with("debug") ||
                  request.method == "initialize" && is_dap_initialize(&request) {
            dap_server.handle_request(request)  // NEW
        } else {
            handle_unknown_method(request)
        };

        write_json_rpc_response(response)?;
    }
}
```

#### 2.2.2 AST Integration for Breakpoint Validation ✅ **STRONG FOUNDATION**

**Current Parser Infrastructure** (`crates/perl-parser/src/parser.rs`):
- ✅ **~100% Perl syntax coverage**: Accurate AST for all language constructs
- ✅ **Position tracking**: Precise span information for every AST node
- ✅ **Incremental parsing**: <1ms updates for live breakpoint adjustment (AC7)

**DAP Breakpoint Validation Strategy**:
```rust
// Leverage existing AST for breakpoint line validation
use perl_parser::{Parser, AstNode};

fn validate_breakpoint_line(source: &str, line: u32) -> BreakpointVerification {
    let ast = Parser::parse(source)?;
    let line_span = ast.line_to_span(line)?;

    // Check if line contains executable code (not comment/blank)
    if ast.is_comment_or_blank(line_span) {
        return BreakpointVerification::Invalid {
            reason: "Line contains only comments or whitespace"
        };
    }

    // Validate not inside heredoc or multi-line string
    if ast.is_inside_string_literal(line_span) {
        return BreakpointVerification::Invalid {
            reason: "Line is inside string literal or heredoc"
        };
    }

    BreakpointVerification::Verified { line: adjusted_line }
}
```

**Incremental Parsing Integration** (AC7 requirement):
```rust
// Reuse existing incremental parsing for live breakpoint updates
use perl_parser::incremental_v2::IncrementalParserV2;

impl DapServer {
    fn handle_text_change(&mut self, uri: &str, changes: Vec<TextEdit>) {
        // Apply incremental parsing (<1ms update)
        self.parser.apply_edits(uri, changes)?;

        // Re-validate affected breakpoints
        let affected_lines = self.get_affected_lines(changes);
        for line in affected_lines {
            let bp_id = self.breakpoints.get_by_line(uri, line)?;
            let verification = validate_breakpoint_line(self.parser.source(uri), line);
            self.send_breakpoint_event(bp_id, verification);
        }
    }
}
```

#### 2.2.3 Workspace Navigation Integration ✅ **DUAL INDEXING SYNERGY**

**Current Workspace Infrastructure** (`crates/perl-parser/src/workspace_index.rs`):
- ✅ **Dual indexing**: Functions indexed under both qualified and bare names (98% coverage)
- ✅ **Cross-file navigation**: Multi-root workspace support
- ✅ **Path normalization**: Windows/macOS/Linux compatibility

**DAP Stack Navigation Strategy**:
```rust
// Leverage existing workspace index for stack frame navigation
use perl_parser::workspace_index::WorkspaceIndex;

fn resolve_stack_frame_source(
    workspace: &WorkspaceIndex,
    package: &str,
    subroutine: &str,
) -> Option<Location> {
    // Use dual pattern matching (existing infrastructure)
    let qualified = format!("{}::{}", package, subroutine);

    // Search exact match first
    if let Some(def) = workspace.get_definition(&qualified) {
        return Some(def.location);
    }

    // Fallback to bare name search (dual indexing)
    workspace.get_definition(subroutine).map(|def| def.location)
}
```

**Benefits for DAP**:
- ✅ **Stack trace accuracy**: Reuse existing symbol resolution for frame locations
- ✅ **Cross-file debugging**: Navigate stack frames across package boundaries
- ✅ **Path mapping**: Leverage existing URI normalization for Windows/WSL

#### 2.2.4 UTF-16 Position Security Reuse ✅ **PRODUCTION-READY**

**Current Position Mapping** (`crates/perl-parser/src/position_mapper.rs`):
- ✅ **Symmetric conversion**: PR #153 guarantees round-trip accuracy
- ✅ **Boundary validation**: Overflow prevention in position arithmetic
- ✅ **Unicode safety**: Multi-byte character and emoji handling

**DAP Position Mapping Requirements**:
- ✅ **Breakpoint positions**: DAP uses 0-based line/column (LSP uses UTF-16)
- ✅ **Variable rendering**: Unicode-safe variable value display
- ✅ **Source mapping**: Accurate source location for stack frames

**Integration Pattern**:
```rust
// Reuse existing position mapper for DAP requests
use perl_parser::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos, PosEnc};

fn dap_breakpoint_to_byte_offset(
    rope: &Rope,
    line: u32,
    column: u32,
) -> Result<usize> {
    // DAP uses 0-based, LSP uses 0-based but UTF-16 units
    let lsp_pos = Position { line, character: column };
    lsp_pos_to_byte(rope, lsp_pos, PosEnc::Utf16)  // Reuse LSP infrastructure
}
```

### 2.3 Rust ↔ Perl Shim Integration Complexity

**Native Approach Architectural Design**:

```
┌─────────────────────────────────────────────────────────────┐
│                    VS Code Extension                        │
│  (contributes.debuggers, launch.json management)            │
└───────────────────────────┬─────────────────────────────────┘
                            │ DAP over stdio
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              perl-dap Rust Adapter (NEW CRATE)              │
│  - JSON-RPC DAP protocol handling                           │
│  - Breakpoint management and path mapping                   │
│  - Session state management (Arc<Mutex<>>)                  │
│  - Error handling (anyhow::Result)                          │
└───────────────────────────┬─────────────────────────────────┘
                            │ JSON over stdio/TCP
                            ↓
┌─────────────────────────────────────────────────────────────┐
│         Devel::TSPerlDAP Perl Shim (NEW CPAN MODULE)        │
│  - Machine-readable JSON protocol                           │
│  - Commands: set_breakpoints, continue, step, stack, vars   │
│  - Uses: PadWalker, B::Deparse, %DB::sub                    │
│  - Spawns Perl process: perl -d:TSPerlDAP script.pl         │
└─────────────────────────────────────────────────────────────┘
                            │ Perl debugger API
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              perl -d (Perl Debugger Runtime)                │
│  - $DB::single, $DB::trace, $DB::sub hooks                  │
│  - caller() stack introspection                             │
│  - Package symbol table access                              │
└─────────────────────────────────────────────────────────────┘
```

**Complexity Assessment**:

#### 2.3.1 Rust Adapter Complexity: ⚠️ **MODERATE**
- **Protocol Implementation**: DAP 1.x protocol has ~30 request types vs LSP's ~60
- **State Management**: Session state simpler than LSP (single thread model)
- **Error Handling**: Reuse existing `anyhow::Result` patterns from LSP
- **Estimated Effort**: 2-3 weeks for core adapter (AC5-AC11)

#### 2.3.2 Perl Shim Complexity: ⚠️ **HIGH**
- **Perl Debugger Integration**: `perl -d` TTY output is fragile and undocumented
- **JSON Protocol Design**: Define machine-readable protocol (not standard)
- **CPAN Module Maintenance**: Long-term maintenance burden for project
- **Estimated Effort**: 2-3 weeks for shim + 1 week for CPAN packaging (AC6)

**Total Native Implementation Effort**: **3-5 weeks** (increased from spec's 2-4 weeks due to no existing code)

### 2.4 Alternative Approaches Evaluation

#### Option A: Bridge to Perl::LanguageServer ✅ **LOWEST RISK**

**Approach**: Delegate DAP requests to existing Perl::LanguageServer DAP implementation

**Pros**:
- ✅ **Fastest time-to-value**: 1-2 days implementation (AC1-AC4)
- ✅ **Battle-tested**: Perl::LanguageServer has production DAP implementation
- ✅ **No Perl shim required**: External project maintains debugger integration
- ✅ **Immediate user value**: Users can debug Perl code within week 1

**Cons**:
- ❌ **External dependency**: Limited customization for perl-lsp workflows
- ❌ **Dual installation**: Users install both perl-lsp + Perl::LanguageServer
- ❌ **Performance unknown**: Can't optimize for perl-lsp's incremental parsing
- ❌ **Long-term uncertainty**: Dependent on external project maintenance

**Risk Assessment**: **LOW** - Well-understood integration pattern, clear rollback path

#### Option B: Native Rust + Perl Shim ✅ **HIGHEST QUALITY**

**Approach**: Build production-grade DAP adapter owned by perl-lsp project

**Pros**:
- ✅ **Full control**: Optimize for perl-lsp's AST and incremental parsing
- ✅ **Enterprise quality**: Apply perl-lsp's security and performance standards
- ✅ **LSP integration**: Tight integration with existing LSP features (hover evaluate)
- ✅ **Long-term sustainability**: No external dependency risk

**Cons**:
- ❌ **3-5 weeks effort**: Significant development investment (increased from spec estimate)
- ❌ **Perl debugger complexity**: TTY output scraping is brittle and undocumented
- ❌ **CPAN maintenance**: Long-term burden for `Devel::TSPerlDAP` module
- ❌ **Testing complexity**: Cross-platform validation matrix (Windows/macOS/Linux/WSL)

**Risk Assessment**: **MODERATE-HIGH** - Complex Perl debugger integration, unknown edge cases

#### Option C: Phased Implementation (RECOMMENDED) ✅ **OPTIMAL BALANCE**

**Approach**: Bridge first for immediate value, then migrate to native incrementally

**Phase 1 (Week 1-2)**: Bridge Implementation
- Implement AC1-AC4 with Perl::LanguageServer delegation
- Deliver immediate debugging capability to users
- Gather user feedback on DAP feature priorities

**Phase 2 (Week 3-6)**: Native Infrastructure
- Build Rust adapter (AC5) and Perl shim (AC6) in parallel
- Implement core DAP features (AC7-AC10) with LSP integration
- Migrate VS Code extension to native adapter (AC11-AC12)

**Phase 3 (Week 7-8)**: Production Hardening
- Comprehensive testing (AC13), benchmarking (AC14), documentation (AC15)
- Security validation (AC16 NEW), LSP non-regression (AC17 NEW)
- Deprecate bridge approach with migration guide

**Pros**:
- ✅ **Fastest initial value**: Users get debugging in 1-2 days
- ✅ **Incremental risk**: Validate DAP workflow before investing in native
- ✅ **User feedback loop**: Real usage informs native implementation priorities
- ✅ **Graceful migration**: Bridge remains as fallback during native development

**Cons**:
- ⚠️ **Dual implementation**: Temporary overhead maintaining both bridge and native
- ⚠️ **Migration complexity**: Users need to switch from bridge to native eventually

**Risk Assessment**: **LOW-MODERATE** - Staged risk introduction with clear rollback points

---

## 3. Performance & Security Assessment

### 3.1 Performance Requirements Validation

**Current Targets from Spec**:

| Operation | Target | Validation Command | Assessment |
|---|---|---|---|
| Breakpoint verification | <50ms | `cargo test --test dap_breakpoint_latency` | ✅ **Achievable** with AST caching |
| Step/continue operations | <100ms p95 | `cargo test --test dap_control_flow_performance` | ✅ **Achievable** with JSON protocol |
| Variable expansion | <200ms initial, <100ms per child | `cargo test --test dap_variable_rendering` | ⚠️ **Challenging** for large structures |
| Memory overhead | <1MB adapter, <5MB shim | `cargo test --test dap_memory_usage` | ✅ **Achievable** with lazy loading |
| Incremental parsing integration | <1ms breakpoint updates | `cargo test --test dap_incremental_breakpoints` | ✅ **Existing infrastructure** |

**Performance Risks Identified**:

1. **Variable Rendering for Large Data Structures** ⚠️ **MODERATE RISK**
   - **Risk**: Rendering 10KB+ arrays/hashes may exceed 200ms target
   - **Mitigation**: Implement truncation with "…" suffix (spec already specifies 1KB preview)
   - **Validation**: Property-based testing with large data structures

2. **Cross-File Breakpoint Resolution** ⚠️ **LOW RISK**
   - **Risk**: Breakpoint verification across 100K+ LOC workspace may exceed 50ms
   - **Mitigation**: Leverage existing workspace index caching (98% coverage)
   - **Validation**: Benchmark with large Perl codebases (CPAN modules)

3. **WSL Filesystem Performance** ⚠️ **MODERATE RISK**
   - **Risk**: WSL1 filesystem access may degrade step/continue latency
   - **Mitigation**: Recommend WSL2 for best performance, validate separately
   - **Validation**: Add WSL-specific performance benchmarks (AC12 enhancement)

**Recommended Performance Validation Strategy**:
```rust
// Add comprehensive performance test suite
// crates/perl-dap/tests/performance_benchmarks.rs

#[test] // AC14 - Performance benchmarks
fn test_breakpoint_verification_latency() {
    let fixtures = vec![
        ("small.pl", 100),      // 100 lines
        ("medium.pl", 1000),    // 1000 lines
        ("large.pl", 10000),    // 10K lines
    ];

    for (fixture, _lines) in fixtures {
        let start = Instant::now();
        let verification = verify_breakpoint(fixture, 42);
        let duration = start.elapsed();

        assert!(duration < Duration::from_millis(50),
                "Breakpoint verification for {} exceeded 50ms: {:?}",
                fixture, duration);
    }
}

#[test] // AC14 - Memory overhead validation
fn test_memory_usage_within_limits() {
    let session = DapSession::new();
    let initial_memory = current_memory_usage();

    // Simulate typical debugging session
    session.set_breakpoints("test.pl", vec![10, 20, 30])?;
    session.launch("test.pl")?;
    session.stack_trace()?;
    session.variables()?;

    let final_memory = current_memory_usage();
    let overhead = final_memory - initial_memory;

    assert!(overhead < 1_000_000, // 1MB adapter state
            "Adapter memory overhead exceeded 1MB: {} bytes", overhead);
}
```

### 3.2 Security Requirements Validation

**Existing Security Infrastructure Alignment**:

#### 3.2.1 Safe Evaluation (AC10) ✅ **WELL-DESIGNED**

**Current Spec Requirement**:
> Safe evaluation mode (non-mutating) by default; requires `allowSideEffects: true` for state changes

**Implementation Strategy**:
```rust
// Apply enterprise security patterns from SECURITY_DEVELOPMENT_GUIDE.md
fn evaluate_expression(
    expr: &str,
    context: &StackFrame,
    allow_side_effects: bool,
) -> Result<Value> {
    // Input validation: prevent code injection
    validate_expression_safety(expr)?;

    // Timeout enforcement: prevent DoS (5s default)
    let timeout = Duration::from_secs(5);
    let result = tokio::time::timeout(timeout, async {
        if allow_side_effects {
            // Full evaluation with write access
            context.eval_with_side_effects(expr)
        } else {
            // Safe evaluation: read-only mode
            context.eval_readonly(expr)
        }
    }).await??;

    Ok(result)
}
```

**Security Validation Tests** (AC16 NEW):
```rust
#[test] // AC16 - Security validation
fn test_safe_eval_prevents_side_effects() {
    let frame = create_test_stack_frame();

    // Should succeed: read-only expression
    let result = evaluate_expression("$var + 10", &frame, false);
    assert!(result.is_ok());

    // Should fail: side effect without opt-in
    let result = evaluate_expression("$var = 42", &frame, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("side effect"));

    // Should succeed: explicit opt-in
    let result = evaluate_expression("$var = 42", &frame, true);
    assert!(result.is_ok());
}

#[test] // AC16 - Timeout enforcement
fn test_eval_timeout_prevents_dos() {
    let frame = create_test_stack_frame();

    // Infinite loop should timeout
    let result = evaluate_expression("while(1) {}", &frame, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}
```

#### 3.2.2 Path Traversal Prevention ✅ **EXISTING INFRASTRUCTURE**

**Current Spec Reference**:
> Validate all file paths through existing enterprise security framework

**Implementation Strategy**:
```rust
// Reuse existing path security from SECURITY_DEVELOPMENT_GUIDE.md
use perl_parser::security::validate_workspace_path;

fn set_breakpoints(uri: &str, breakpoints: Vec<Breakpoint>) -> Result<()> {
    // Validate path is within workspace boundaries
    let canonical_path = validate_workspace_path(uri)?;

    // Prevent directory traversal attacks
    if canonical_path.contains("..") {
        return Err(SecurityError::PathTraversalAttempt(uri.to_string()));
    }

    // Apply breakpoints to validated path
    self.breakpoint_manager.set(canonical_path, breakpoints)
}
```

**Security Test Cases** (AC16 NEW):
```rust
#[test] // AC16 - Path traversal prevention
fn test_breakpoint_path_validation() {
    let adapter = DapAdapter::new();

    // Valid workspace path
    let result = adapter.set_breakpoints("file:///workspace/lib/Module.pm", vec![]);
    assert!(result.is_ok());

    // Path traversal attack
    let result = adapter.set_breakpoints("file:///workspace/../../../etc/passwd", vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));

    // UNC path validation (Windows)
    #[cfg(windows)]
    {
        let result = adapter.set_breakpoints("file://server/share/../sensitive", vec![]);
        assert!(result.is_err());
    }
}
```

#### 3.2.3 UTF-16 Position Security Reuse ✅ **PR #153 FOUNDATION**

**Current Infrastructure**:
- ✅ **Symmetric conversion**: Round-trip position accuracy guaranteed
- ✅ **Boundary validation**: Overflow prevention in position arithmetic
- ✅ **Unicode safety**: Multi-byte character handling

**DAP-Specific Application**:
```rust
// Reuse position security for variable rendering
use perl_parser::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos};

fn render_variable_value(value: &str, rope: &Rope) -> String {
    // Truncate large values (security: prevent memory exhaustion)
    if value.len() > 1024 {
        let truncated = &value[..1024];

        // UTF-16 safe truncation (PR #153 infrastructure)
        let safe_truncate = ensure_utf16_boundary(truncated, rope);
        format!("{}…", safe_truncate)
    } else {
        value.to_string()
    }
}
```

**Security Validation** (AC16 NEW):
```rust
#[test] // AC16 - Unicode boundary safety
fn test_variable_rendering_unicode_safety() {
    let rope = Rope::from_str("my $emoji = '😀👨‍👩‍👧‍👦🎉';");

    // Large unicode value should truncate safely
    let large_value = "😀".repeat(500); // 2000 bytes (emoji are 4-byte UTF-8)
    let rendered = render_variable_value(&large_value, &rope);

    // Should not panic on UTF-16 boundary
    assert!(rendered.len() <= 1024 + 1); // +1 for '…'
    assert!(rendered.ends_with('…'));
    assert!(is_valid_utf8(&rendered)); // No broken emoji
}
```

---

## 4. Architecture Decision & Recommendations

### 4.1 Recommended Implementation Strategy: **PHASED APPROACH**

**Decision**: Implement **Phased Bridge-to-Native Strategy** for optimal risk/reward balance

**Rationale**:
1. **User Value First**: Bridge delivers debugging in 1-2 days vs 5+ weeks for native
2. **Risk Mitigation**: Validate DAP workflow with real users before major investment
3. **Feedback Loop**: Real usage informs native implementation priorities
4. **Graceful Migration**: Bridge remains as fallback during native development

**Implementation Timeline**:

```
Week 1-2: Phase 1 - Bridge Implementation (AC1-AC4)
├─ AC1: VS Code debugger contribution (0.5 days)
├─ AC2: Launch.json snippets (0.5 days)
├─ AC3: Bridge documentation (0.5 days)
└─ AC4: Basic workflow validation (0.5 days)

Week 3-6: Phase 2 - Native Infrastructure (AC5-AC12)
├─ AC5: perl-dap crate scaffolding (1 week)
├─ AC6: Devel::TSPerlDAP Perl shim (2 weeks - CRITICAL PATH)
├─ AC7: Breakpoint management (0.5 weeks)
├─ AC8: Stack/variables/scopes (1 week)
├─ AC9: Stepping and control flow (0.5 weeks)
├─ AC10: Evaluate and REPL (0.5 weeks)
├─ AC11: VS Code native integration (0.5 weeks)
└─ AC12: Cross-platform validation (0.5 weeks)

Week 7-8: Phase 3 - Production Hardening (AC13-AC15, AC16-AC19 NEW)
├─ AC13: Comprehensive integration tests (0.5 weeks)
├─ AC14: Performance benchmarks (0.5 weeks)
├─ AC15: Documentation complete (0.5 weeks)
├─ AC16: Security validation (0.5 weeks - NEW)
├─ AC17: LSP non-regression (0.5 weeks - NEW)
├─ AC18: Dependency management (0.5 weeks - NEW)
└─ AC19: Binary packaging (0.5 weeks - NEW)

Total: 8 weeks (2 weeks faster than pure native, with immediate user value)
```

### 4.2 Crate Structure Recommendation

**Decision**: Create **new `perl-dap` crate** separate from `perl-lsp` and `perl-parser`

**Rationale**:
1. **Clean separation**: DAP protocol is distinct from LSP protocol
2. **Optional dependency**: Users can install `perl-lsp` without DAP support
3. **Focused testing**: DAP-specific test infrastructure without polluting LSP tests
4. **Independent versioning**: DAP features can evolve separately from LSP

**Proposed Workspace Structure**:
```
crates/
├── perl-parser/          # Core parser (unchanged)
├── perl-lsp/             # LSP server binary (unchanged)
├── perl-dap/             # NEW - DAP adapter binary
│   ├── src/
│   │   ├── main.rs       # DAP adapter entry point
│   │   ├── protocol.rs   # DAP protocol implementation
│   │   ├── session.rs    # Debug session management
│   │   ├── breakpoints.rs# Breakpoint manager
│   │   ├── variables.rs  # Variable rendering
│   │   └── shim.rs       # Perl shim communication
│   ├── tests/
│   │   ├── integration_tests.rs  # AC13
│   │   ├── performance_benchmarks.rs  # AC14
│   │   └── security_validation.rs  # AC16 NEW
│   └── Cargo.toml
├── perl-lexer/           # Tokenizer (unchanged)
└── perl-corpus/          # Test corpus (unchanged)

vscode-extension/
├── package.json          # Add contributes.debuggers (AC1, AC11)
├── launch-snippets.json  # AC2
└── dap-binaries/         # Platform binaries for perl-dap (AC19)
    ├── linux-x64/
    ├── linux-arm64/
    ├── darwin-x64/
    ├── darwin-arm64/
    ├── win32-x64/
    └── win32-arm64/
```

**Dependencies for `perl-dap` Crate**:
```toml
# crates/perl-dap/Cargo.toml
[dependencies]
perl-parser = { path = "../perl-parser", version = "0.8.9" }  # AST integration
lsp-types = "0.97.0"      # Reuse LSP types (Position, Range, etc.)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"            # Error handling
thiserror = "2.0"         # Error types
tokio = { version = "1.0", features = ["full"] }  # Async runtime
tracing = "0.1"           # Logging infrastructure
ropey = "1.6"             # Rope for position mapping (reuse perl-parser)

[dev-dependencies]
proptest = "1.0"          # Property-based testing (AC13)
criterion = "0.5"         # Performance benchmarking (AC14)
tempfile = "3.0"          # Test fixtures
```

### 4.3 Integration with Existing LSP Infrastructure

**Decision**: **Tight integration** with existing perl-parser components, **loose coupling** with perl-lsp binary

**Integration Points**:

#### 4.3.1 Breakpoint Validation via AST ✅ **HIGH PRIORITY**
```rust
// crates/perl-dap/src/breakpoints.rs
use perl_parser::{Parser, AstNode};

pub struct BreakpointManager {
    parser: Arc<Parser>,  // Shared with LSP server
}

impl BreakpointManager {
    pub fn verify_breakpoint(&self, uri: &str, line: u32) -> BreakpointVerification {
        let ast = self.parser.parse_file(uri)?;

        // Reuse existing AST traversal infrastructure
        if ast.is_comment_or_blank_line(line) {
            return BreakpointVerification::Invalid {
                reason: "Comment or blank line"
            };
        }

        BreakpointVerification::Verified { line }
    }
}
```

#### 4.3.2 Incremental Parsing for Live Breakpoint Updates ✅ **HIGH PRIORITY**
```rust
// crates/perl-dap/src/session.rs
use perl_parser::incremental_v2::IncrementalParserV2;

pub struct DapSession {
    parser: IncrementalParserV2,  // <1ms updates (existing)
    breakpoints: HashMap<String, Vec<Breakpoint>>,
}

impl DapSession {
    pub fn on_text_change(&mut self, uri: &str, changes: Vec<TextEdit>) {
        // Apply incremental parsing (<1ms target - AC7)
        self.parser.apply_edits(uri, &changes)?;

        // Re-verify affected breakpoints
        let affected_lines = self.calculate_affected_lines(&changes);
        for line in affected_lines {
            let verification = self.breakpoints.verify(uri, line);
            self.send_breakpoint_event(uri, line, verification);
        }
    }
}
```

#### 4.3.3 Workspace Navigation for Stack Frames ✅ **MEDIUM PRIORITY**
```rust
// crates/perl-dap/src/stack.rs
use perl_parser::workspace_index::WorkspaceIndex;

pub struct StackTraceProvider {
    workspace: Arc<WorkspaceIndex>,  // Shared with LSP
}

impl StackTraceProvider {
    pub fn resolve_frame_location(&self, frame: &StackFrame) -> Option<Location> {
        // Use dual pattern matching (98% coverage)
        let qualified = format!("{}::{}", frame.package, frame.subroutine);

        self.workspace.get_definition(&qualified)
            .or_else(|| self.workspace.get_definition(&frame.subroutine))
            .map(|def| def.location)
    }
}
```

#### 4.3.4 UTF-16 Position Mapping Reuse ✅ **HIGH PRIORITY**
```rust
// crates/perl-dap/src/protocol.rs
use perl_parser::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos, PosEnc};

pub fn dap_breakpoint_to_byte_offset(
    rope: &Rope,
    line: u32,
    column: u32,
) -> Result<usize> {
    // DAP uses 0-based line/column, LSP infrastructure compatible
    let pos = Position { line, character: column };
    lsp_pos_to_byte(rope, pos, PosEnc::Utf16)  // PR #153 security
}
```

### 4.4 Risk Assessment Summary

| Risk Category | Severity | Mitigation Strategy | Timeline Impact |
|---|---|---|---|
| **No existing DAP code** | ⚠️ **MODERATE** | Phased approach validates workflow first | +1 week native timeline |
| **Perl debugger complexity** | ⚠️ **HIGH** | Invest in robust `Devel::TSPerlDAP` shim | +1 week native timeline |
| **CPAN module maintenance** | ⚠️ **MODERATE** | Bundled fallback + clear versioning strategy | AC18 NEW addresses |
| **Cross-platform testing** | ⚠️ **MODERATE** | WSL-specific validation matrix | AC12 enhancement |
| **Variable rendering performance** | ⚠️ **LOW** | Lazy loading + 1KB truncation | Already specified |
| **Bridge dependency risk** | ✅ **LOW** | Temporary bridge with native migration path | Phased approach |
| **LSP regression risk** | ⚠️ **MODERATE** | Comprehensive LSP test suite validation | AC17 NEW |
| **Security vulnerabilities** | ✅ **LOW** | Reuse enterprise security infrastructure | AC16 NEW |

**Overall Risk Level**: **MODERATE** with clear mitigation strategies

---

## 5. Implementation Roadmap

### 5.1 Phase 1: Bridge Implementation (Week 1-2)

**Goal**: Deliver immediate debugging capability to users

**Acceptance Criteria**: AC1-AC4

**Implementation Tasks**:

#### AC1: VS Code Extension Debugger Contribution
```json
// vscode-extension/package.json
{
  "contributes": {
    "debuggers": [
      {
        "type": "perl",
        "label": "Perl Debug",
        "program": "./out/debugAdapter.js",
        "runtime": "node",
        "configurationAttributes": {
          "launch": {
            "required": ["program"],
            "properties": {
              "program": { "type": "string", "description": "Perl script to debug" },
              "args": { "type": "array", "description": "Command line arguments" },
              "perlPath": { "type": "string", "description": "Path to Perl executable", "default": "perl" },
              "includePaths": { "type": "array", "description": "Additional @INC paths" },
              "env": { "type": "object", "description": "Environment variables" },
              "cwd": { "type": "string", "description": "Working directory" }
            }
          }
        }
      }
    ]
  }
}
```

**Test Validation**:
```bash
# Validate extension loads debugger contribution
cd vscode-extension && npm test
# Expected: Extension contributes "perl" debugger type
```

#### AC2: Launch.json Snippets
```json
// vscode-extension/launch-snippets.json
{
  "Perl: Launch": {
    "prefix": "perl-launch",
    "body": {
      "type": "perl",
      "request": "launch",
      "name": "Launch Perl Script",
      "program": "${workspaceFolder}/${1:script.pl}",
      "args": [],
      "perlPath": "perl",
      "includePaths": ["${workspaceFolder}/lib"],
      "cwd": "${workspaceFolder}"
    }
  },
  "Perl: Attach (TCP)": {
    "prefix": "perl-attach",
    "body": {
      "type": "perl",
      "request": "attach",
      "name": "Attach to Perl Process",
      "host": "localhost",
      "port": 5000,
      "pathMapping": {
        "${workspaceFolder}": "/remote/workspace"
      }
    }
  }
}
```

**Test Validation**:
```bash
# Validate snippets on all platforms
cargo test --test dap_launch_snippets -- windows macos linux
# Expected: Launch configurations work on all platforms
```

#### AC3: Bridge Setup Documentation
```markdown
<!-- docs/DAP_BRIDGE_SETUP_GUIDE.md -->
# Using perl-lsp with Perl::LanguageServer DAP (Bridge Mode)

## Installation

1. Install Perl::LanguageServer:
   ```bash
   cpanm Perl::LanguageServer
   ```

2. Verify installation:
   ```bash
   perl -MPerlLanguageServer -e 'print "OK\n"'
   ```

3. Configure VS Code launch.json (see snippets above)

## Troubleshooting

### Path Mapping Issues
- **Symptom**: Breakpoints not hitting
- **Cause**: VS Code URI vs filesystem path mismatch
- **Fix**: Add pathMapping to launch.json

### Windows UNC Paths
- **Symptom**: Breakpoints fail on network shares
- **Cause**: UNC path normalization
- **Fix**: Use drive letter mapping (Z:\ instead of \\server\share)
```

**Test Validation**:
```bash
# Validate documentation completeness
cargo test --test dap_documentation_coverage -- AC3
# Expected: All troubleshooting scenarios covered
```

#### AC4: Basic Debugging Workflow
**Test Fixtures**:
```perl
# crates/perl-dap/tests/fixtures/bridge_basic.pl
#!/usr/bin/env perl
use strict;
use warnings;

my $x = 10;  # BREAKPOINT: Line 5
my $y = 20;

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;  # BREAKPOINT: Line 10
}

my $result = calculate($x, $y);
print "Result: $result\n";  # BREAKPOINT: Line 14
```

**Golden Transcript Validation**:
```rust
// crates/perl-dap/tests/bridge_workflow_tests.rs
#[test] // AC4 - Basic workflow validation
fn test_bridge_basic_workflow() {
    let adapter = BridgeAdapter::new();

    // Initialize
    let init_response = adapter.send_request("initialize", json!({
        "clientID": "vscode",
        "adapterID": "perl"
    }))?;
    assert_eq!(init_response["supportsConfigurationDoneRequest"], true);

    // Set breakpoints
    let bp_response = adapter.send_request("setBreakpoints", json!({
        "source": { "path": "bridge_basic.pl" },
        "breakpoints": [
            { "line": 5 },
            { "line": 10 },
            { "line": 14 }
        ]
    }))?;
    assert_eq!(bp_response["breakpoints"].as_array().unwrap().len(), 3);

    // Launch
    adapter.send_request("launch", json!({
        "program": "bridge_basic.pl",
        "noDebug": false
    }))?;

    // Continue to first breakpoint
    let stopped_event = adapter.wait_for_event("stopped")?;
    assert_eq!(stopped_event["reason"], "breakpoint");
    assert_eq!(stopped_event["body"]["line"], 5);

    // Get stack trace
    let stack_response = adapter.send_request("stackTrace", json!({
        "threadId": 1
    }))?;
    assert!(stack_response["stackFrames"].as_array().unwrap().len() > 0);

    // Get local variables
    let frame_id = stack_response["stackFrames"][0]["id"].as_i64().unwrap();
    let scopes_response = adapter.send_request("scopes", json!({
        "frameId": frame_id
    }))?;

    let locals_ref = scopes_response["scopes"][0]["variablesReference"].as_i64().unwrap();
    let vars_response = adapter.send_request("variables", json!({
        "variablesReference": locals_ref
    }))?;

    // Verify $x variable
    let x_var = vars_response["variables"].as_array().unwrap()
        .iter().find(|v| v["name"] == "$x").unwrap();
    assert_eq!(x_var["value"], "10");

    // Continue to completion
    adapter.send_request("continue", json!({ "threadId": 1 }))?;
    adapter.wait_for_event("terminated")?;
}
```

**Deliverables**:
- ✅ VS Code extension with Perl::LanguageServer bridge
- ✅ Launch.json snippets validated on Windows/macOS/Linux
- ✅ Bridge setup documentation with troubleshooting
- ✅ Basic workflow integration tests passing

### 5.2 Phase 2: Native Infrastructure (Week 3-6)

**Goal**: Build production-grade DAP adapter owned by perl-lsp

**Acceptance Criteria**: AC5-AC12

**Critical Path**: **AC6 (Devel::TSPerlDAP Perl Shim)** - 2 weeks

#### AC5: perl-dap Rust Crate Scaffolding (Week 3)
```rust
// crates/perl-dap/src/protocol.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub command: String,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DapResponse {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<serde_json::Value>,
}

// crates/perl-dap/src/session.rs
pub struct DapSession {
    shim: PerlShimConnection,
    breakpoints: BreakpointManager,
    state: Arc<Mutex<SessionState>>,
}

impl DapSession {
    pub async fn handle_request(&mut self, request: DapRequest) -> DapResponse {
        match request.command.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "launch" => self.handle_launch(request).await,
            "setBreakpoints" => self.handle_set_breakpoints(request).await,
            "continue" => self.handle_continue(request).await,
            // ... other commands
            _ => self.handle_unknown_command(request),
        }
    }
}
```

**Test Validation**:
```bash
# Validate protocol scaffolding
cargo test -p perl-dap --test protocol_compliance
# Expected: <50ms response time for initialize/launch (AC5)
```

#### AC6: Devel::TSPerlDAP CPAN Module (Week 4-5) - **CRITICAL PATH**

**Perl Shim Architecture**:
```perl
# Devel/TSPerlDAP.pm
package Devel::TSPerlDAP;
use strict;
use warnings;
use JSON::PP;
use IO::Socket::INET;
use PadWalker qw(peek_my);
use B::Deparse;

our $VERSION = '0.1.0';

# JSON server over stdio or TCP
sub import {
    my ($class, %opts) = @_;

    my $daemon = $opts{daemon} // 0;
    my $host = $opts{host} // '127.0.0.1';
    my $port = $opts{port} // 0;  # 0 = random port

    if ($daemon) {
        start_tcp_server($host, $port);
    } else {
        start_stdio_server();
    }
}

sub start_stdio_server {
    while (my $line = <STDIN>) {
        my $request = decode_json($line);
        my $response = handle_command($request);
        print encode_json($response), "\n";
    }
}

sub handle_command {
    my ($request) = @_;

    my $command = $request->{command};

    if ($command eq 'set_breakpoints') {
        return set_breakpoints($request->{arguments});
    } elsif ($command eq 'continue') {
        return continue_execution();
    } elsif ($command eq 'stack') {
        return get_stack_trace();
    } elsif ($command eq 'variables') {
        return get_variables($request->{arguments});
    }
    # ... other commands
}

sub set_breakpoints {
    my ($args) = @_;

    my $file = $args->{source}{path};
    my @lines = @{$args->{breakpoints}};

    # Use Perl debugger API: $DB::single
    foreach my $bp (@lines) {
        my $line = $bp->{line};
        $DB::single{$file}{$line} = 1;
    }

    return { success => 1, breakpoints => \@lines };
}

sub get_stack_trace {
    my @frames;

    my $i = 0;
    while (my ($package, $file, $line, $sub) = caller($i++)) {
        push @frames, {
            name => $sub,
            source => { path => $file },
            line => $line,
            column => 0,
        };
    }

    return { stackFrames => \@frames };
}

sub get_variables {
    my ($args) = @_;

    my $frame_id = $args->{frameId};

    # Use PadWalker to inspect lexical variables
    my $vars = peek_my($frame_id);

    my @variables;
    foreach my $name (sort keys %$vars) {
        my $value = $vars->{$name};

        push @variables, {
            name => $name,
            value => render_value($value),
            type => ref($value) || 'scalar',
        };
    }

    return { variables => \@variables };
}

sub render_value {
    my ($value) = @_;

    if (ref($value) eq 'CODE') {
        # Use B::Deparse for code refs
        my $deparse = B::Deparse->new();
        return $deparse->coderef2text($value);
    } elsif (ref($value) eq 'ARRAY') {
        return "[" . @$value . " items]";
    } elsif (ref($value) eq 'HASH') {
        return "{" . (scalar keys %$value) . " keys}";
    } else {
        # Truncate large values (AC8)
        my $str = "$value";
        return length($str) > 1024 ? substr($str, 0, 1024) . "…" : $str;
    }
}

1;
```

**CPAN Packaging** (`META.json`):
```json
{
  "name": "Devel-TSPerlDAP",
  "version": "0.1.0",
  "abstract": "Debug Adapter Protocol shim for Perl debugger",
  "author": ["Tree-sitter Perl Contributors"],
  "license": ["mit"],
  "prereqs": {
    "runtime": {
      "requires": {
        "perl": "5.016",
        "JSON::PP": "0",
        "PadWalker": "2.0",
        "B::Deparse": "0"
      }
    }
  }
}
```

**Test Coverage** (>80% requirement from AC6):
```perl
# t/01-set-breakpoints.t
use Test::More tests => 5;
use Devel::TSPerlDAP;

my $result = Devel::TSPerlDAP::set_breakpoints({
    source => { path => 'test.pl' },
    breakpoints => [ { line => 10 }, { line => 20 } ]
});

ok($result->{success}, "Set breakpoints succeeded");
is(scalar @{$result->{breakpoints}}, 2, "Two breakpoints set");

# t/02-stack-trace.t
use Test::More tests => 3;

sub outer { inner() }
sub inner {
    my $stack = Devel::TSPerlDAP::get_stack_trace();
    return $stack;
}

my $result = outer();
ok(scalar @{$result->{stackFrames}} >= 2, "Stack has at least 2 frames");
like($result->{stackFrames}[0]{name}, qr/inner/, "Top frame is 'inner'");

# t/03-variables.t
use Test::More tests => 4;

sub test_vars {
    my $x = 42;
    my @arr = (1, 2, 3);

    my $result = Devel::TSPerlDAP::get_variables({ frameId => 0 });
    my @vars = @{$result->{variables}};

    my ($x_var) = grep { $_->{name} eq '$x' } @vars;
    is($x_var->{value}, 42, '$x value correct');

    my ($arr_var) = grep { $_->{name} eq '@arr' } @vars;
    like($arr_var->{value}, qr/3 items/, '@arr rendered as array');
}

test_vars();
```

**Deliverable**: `Devel::TSPerlDAP` published to CPAN with >80% test coverage

#### AC7-AC12: Core DAP Features (Week 5-6)
**Implementation priority** (parallel with AC6 completion):
1. **AC7**: Breakpoint management (reuse perl-parser AST) - 0.5 weeks
2. **AC8**: Stack/variables/scopes (integrate with shim) - 1 week
3. **AC9**: Stepping and control flow (JSON protocol) - 0.5 weeks
4. **AC10**: Evaluate and REPL (security integration) - 0.5 weeks
5. **AC11**: VS Code native integration (update extension) - 0.5 weeks
6. **AC12**: Cross-platform validation (CI matrix) - 0.5 weeks

**Deliverables**:
- ✅ Native `perl-dap` binary with <100ms p95 latency (AC5, AC9)
- ✅ `Devel::TSPerlDAP` published to CPAN with >80% coverage (AC6)
- ✅ Breakpoint validation integrated with perl-parser AST (AC7)
- ✅ Variable rendering with lazy expansion <100ms (AC8)
- ✅ Safe evaluation with timeout enforcement (AC10)
- ✅ VS Code extension with native adapter (AC11)
- ✅ Cross-platform CI validation (Windows/macOS/Linux) (AC12)

### 5.3 Phase 3: Production Hardening (Week 7-8)

**Goal**: Enterprise-ready debugging with comprehensive testing

**Acceptance Criteria**: AC13-AC15, AC16-AC19 (NEW)

#### AC13: Comprehensive Integration Tests (Week 7)
```rust
// crates/perl-dap/tests/integration_tests.rs

#[test] // AC13 - Golden transcript validation
fn test_hello_world_golden_transcript() {
    let transcript = load_golden_transcript("hello.json");
    let adapter = DapAdapter::new();

    for message in transcript.messages {
        if message.type_ == "request" {
            let response = adapter.handle_request(message.request)?;
            assert_eq!(response, message.expected_response,
                       "Transcript mismatch at seq {}", message.seq);
        }
    }
}

#[test] // AC13 - Breakpoint matrix validation
fn test_breakpoint_edge_cases() {
    let fixtures = vec![
        ("file_start.pl", 1, true),      // First line
        ("file_end.pl", 100, true),      // Last line
        ("blank_line.pl", 10, false),    // Blank line
        ("comment_line.pl", 5, false),   // Comment line
        ("heredoc.pl", 15, false),       // Inside heredoc
        ("begin_block.pl", 3, true),     // BEGIN block
        ("end_block.pl", 50, true),      // END block
    ];

    for (fixture, line, should_verify) in fixtures {
        let result = verify_breakpoint(fixture, line);
        assert_eq!(result.verified, should_verify,
                   "Breakpoint verification mismatch for {}:{}",
                   fixture, line);
    }
}

#[test] // AC13 - Variable rendering validation
fn test_variable_rendering_edge_cases() {
    let test_cases = vec![
        ("$scalar", "42", "scalar"),
        ("@array", "[10 items]", "array"),
        ("%hash", "{5 keys}", "hash"),
        ("$unicode", "Hello 😀 World", "scalar"),
        ("$large", "data…", "scalar"),  // >10KB truncated
    ];

    for (name, expected_value, expected_type) in test_cases {
        let var = render_variable(name)?;
        assert_eq!(var.value, expected_value);
        assert_eq!(var.type_, expected_type);
    }
}
```

**Test Coverage Target**: >95% for DAP adapter, >80% for Perl shim (AC13)

#### AC14: Performance Benchmarks (Week 7)
```rust
// crates/perl-dap/benches/dap_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_breakpoint_verification(c: &mut Criterion) {
    let fixtures = vec![
        ("small.pl", 100),
        ("medium.pl", 1000),
        ("large.pl", 10000),
    ];

    for (fixture, lines) in fixtures {
        c.bench_function(&format!("verify_breakpoint_{}", fixture), |b| {
            b.iter(|| {
                verify_breakpoint(black_box(fixture), black_box(42))
            });
        });
    }
}

fn benchmark_step_continue(c: &mut Criterion) {
    c.bench_function("step_operation", |b| {
        let adapter = DapAdapter::new();
        adapter.launch("test.pl")?;

        b.iter(|| {
            adapter.step_in()
        });
    });
}

criterion_group!(benches, benchmark_breakpoint_verification, benchmark_step_continue);
criterion_main!(benches);
```

**Performance Baselines** (AC14):
- Breakpoint verification: <50ms (small), <50ms (medium), <100ms (large)
- Step/continue: <100ms p95
- Variable expansion: <200ms initial, <100ms per child
- Memory overhead: <1MB adapter, <5MB shim

#### AC15: Documentation Complete (Week 7)
**Deliverables**:
- Tutorial: `docs/DAP_GETTING_STARTED_TUTORIAL.md` (step-by-step with screenshots)
- Reference: `docs/DAP_CONFIGURATION_REFERENCE.md` (all launch.json parameters)
- Architecture: `docs/DAP_IMPLEMENTATION_ARCHITECTURE.md` (Rust adapter + Perl shim design)
- Troubleshooting: `docs/DAP_TROUBLESHOOTING_GUIDE.md` (common issues and solutions)

**Diátaxis Framework Alignment**:
- ✅ **Tutorial**: Getting started guide with hands-on examples
- ✅ **How-to**: Configuration guides for launch.json, attach scenarios
- ✅ **Reference**: Complete DAP protocol implementation reference
- ✅ **Explanation**: Architecture decisions and design rationale

#### AC16 (NEW): Security Validation (Week 8)
```rust
// crates/perl-dap/tests/security_validation.rs

#[test] // AC16 - Path traversal prevention
fn test_path_traversal_prevention() {
    let adapter = DapAdapter::new();

    // Valid path
    let result = adapter.set_breakpoints("file:///workspace/lib/Module.pm", vec![]);
    assert!(result.is_ok());

    // Traversal attack
    let result = adapter.set_breakpoints("file:///workspace/../../../etc/passwd", vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("traversal"));
}

#[test] // AC16 - Safe eval enforcement
fn test_safe_eval_prevents_side_effects() {
    let frame = create_test_frame();

    // Read-only expression OK
    let result = evaluate_expression("$var + 10", &frame, false);
    assert!(result.is_ok());

    // Side effect without opt-in fails
    let result = evaluate_expression("$var = 42", &frame, false);
    assert!(result.is_err());
}

#[test] // AC16 - Timeout enforcement
fn test_eval_timeout_prevents_dos() {
    let frame = create_test_frame();

    // Infinite loop should timeout
    let result = evaluate_expression("while(1) {}", &frame, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}
```

**Security Validation Coverage**:
- ✅ Path traversal prevention (reuse enterprise security framework)
- ✅ Safe eval enforcement (AC10 compliance)
- ✅ Timeout handling (5s default)
- ✅ Unicode boundary safety (PR #153 infrastructure)

#### AC17 (NEW): LSP Integration Non-Regression (Week 8)
```rust
// crates/perl-lsp/tests/lsp_dap_non_regression.rs

#[test] // AC17 - LSP performance maintained with DAP loaded
fn test_lsp_response_time_with_dap_active() {
    let server = LspServer::new();
    let dap_adapter = DapAdapter::new();  // Load DAP binary

    // Initialize LSP server
    server.initialize()?;

    // Measure completion response time
    let start = Instant::now();
    let response = server.completion("test.pl", Position { line: 10, character: 5 })?;
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(50),
            "LSP completion degraded with DAP active: {:?}", duration);
}

#[test] // AC17 - Full LSP test suite passes with DAP
fn test_lsp_comprehensive_suite_with_dap() {
    // Run all LSP tests with DAP adapter loaded
    let results = run_lsp_comprehensive_e2e_test_with_dap()?;

    assert_eq!(results.pass_rate, 1.0,
               "LSP tests failed with DAP active: {:?}", results);
}
```

**LSP Non-Regression Coverage**:
- ✅ 100% LSP test pass rate with DAP loaded
- ✅ <50ms LSP response time targets maintained
- ✅ No memory leaks or resource contention
- ✅ Clean separation between LSP and DAP protocol handling

#### AC18 (NEW): Dependency Management (Week 8)
**Documentation**: `docs/DAP_DEPENDENCY_MANAGEMENT.md`
```markdown
# DAP Dependency Management Guide

## Devel::TSPerlDAP Installation

### Option 1: Auto-install via cpanm (Recommended)
```bash
# VS Code extension auto-installs on first use
perl-dap --install-shim
# Runs: cpanm Devel::TSPerlDAP
```

### Option 2: Bundled Fallback
```bash
# Extension bundles Devel/TSPerlDAP.pm in resources/
# Automatically used if CPAN installation fails
```

### Option 3: System Package (Future)
```bash
# Debian/Ubuntu
sudo apt-get install libdevel-tsperldap-perl

# Fedora/RHEL
sudo dnf install perl-Devel-TSPerlDAP
```

## Compatibility Matrix

| Perl Version | Devel::TSPerlDAP | perl-dap Adapter |
|---|---|---|
| 5.16+ | 0.1.0+ | 0.1.0+ |
| 5.20+ | 0.1.0+ | 0.1.0+ |
| 5.30+ | 0.1.0+ (recommended) | 0.1.0+ |

## Versioning Strategy

- **Adapter ↔ Shim Protocol**: JSON protocol versioned independently
- **Breaking Changes**: Require coordinated release (e.g., adapter 0.2.0 + shim 0.2.0)
- **Feature Detection**: Shim advertises capabilities in `initialize` response
```

**Test Validation**:
```bash
# Validate auto-install workflow
cargo test --test dap_dependency_installation
# Expected: <30 second first-time setup
```

#### AC19 (NEW): Binary Packaging (Week 8)
**VS Code Extension Packaging Strategy**:
```json
// vscode-extension/package.json
{
  "contributes": {
    "debuggers": [
      {
        "type": "perl-rs",
        "label": "Perl Debug (Native)",
        "program": "./bin/perl-dap",  // Platform-specific binary
        "runtime": null,
        "configurationAttributes": { /* ... */ }
      }
    ]
  }
}
```

**Binary Distribution**:
```bash
# GitHub Releases strategy
vscode-extension/
├── bin/
│   ├── linux-x64/perl-dap
│   ├── linux-arm64/perl-dap
│   ├── darwin-x64/perl-dap
│   ├── darwin-arm64/perl-dap
│   ├── win32-x64/perl-dap.exe
│   └── win32-arm64/perl-dap.exe
└── extension.js  # Auto-selects correct binary

# GitHub Actions builds for all platforms
.github/workflows/release-dap-binaries.yml
```

**Auto-Download Strategy** (fallback):
```typescript
// vscode-extension/src/dapBinaryManager.ts
async function ensureDapBinary(): Promise<string> {
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = `perl-dap-${platform}-${arch}`;

    const localPath = path.join(__dirname, '../bin', binaryName);
    if (fs.existsSync(localPath)) {
        return localPath;
    }

    // Download from GitHub Releases
    const version = '0.1.0';
    const url = `https://github.com/EffortlessSteven/tree-sitter-perl/releases/download/v${version}/${binaryName}`;

    console.log(`Downloading ${binaryName} from ${url}...`);
    await downloadFile(url, localPath);

    return localPath;
}
```

**Test Validation**:
```bash
# Validate binary packaging for all platforms
cargo test --test dap_binary_packaging
# Expected: <5 second first-launch download per platform
```

**Deliverables**:
- ✅ Comprehensive integration tests with >95% coverage (AC13)
- ✅ Performance benchmarks with regression detection (AC14)
- ✅ Complete documentation suite (Tutorial/Reference/Architecture/Troubleshooting) (AC15)
- ✅ Security validation with enterprise standards (AC16 NEW)
- ✅ LSP non-regression test suite (AC17 NEW)
- ✅ Dependency management documentation and validation (AC18 NEW)
- ✅ Binary packaging for 6 platforms with auto-download (AC19 NEW)

---

## 6. Next Steps & Routing

### 6.1 Validation Summary

**Specification Quality**: ✅ **EXCELLENT**
- 15 original acceptance criteria well-defined and testable
- 4 new acceptance criteria (AC16-AC19) recommended for completeness
- Comprehensive coverage of DAP protocol, security, and cross-platform requirements

**Technical Feasibility**: ✅ **CONFIRMED WITH ADJUSTMENTS**
- ❌ **No existing DAP implementation** (spec reference incorrect)
- ✅ **Strong LSP infrastructure** reusable for DAP integration
- ✅ **Phased approach** mitigates risk and delivers fast user value
- ⚠️ **Native timeline** increased from 2-4 weeks to **3-5 weeks** (no starting point)

**Architecture Alignment**: ✅ **STRONG**
- ✅ Integrates cleanly with existing perl-parser AST infrastructure
- ✅ Reuses UTF-16 position security (PR #153)
- ✅ Leverages workspace navigation dual indexing (98% coverage)
- ✅ Applies enterprise security practices (SECURITY_DEVELOPMENT_GUIDE.md)

**Risk Assessment**: ⚠️ **MODERATE** with clear mitigation
- **Highest Risk**: Perl debugger integration complexity (AC6)
- **Mitigation**: Phased approach validates workflow before major investment
- **Contingency**: Bridge mode remains as fallback if native blocked

### 6.2 Recommended Enhancements to Issue #207

**Critical Corrections**:
1. ❌ **Remove reference to existing `debug_adapter.rs`** (does not exist)
2. ⚠️ **Update native timeline** from 2-4 weeks to **3-5 weeks**
3. ✅ **Add 4 new acceptance criteria** (AC16-AC19):
   - AC16: Security validation with enterprise standards
   - AC17: LSP integration non-regression
   - AC18: Dependency management strategy
   - AC19: Binary packaging and distribution

**Minor Enhancements**:
1. **AC4**: Add specific test fixtures and golden transcript requirements
2. **AC6**: Clarify CPAN publication timeline and versioning strategy
3. **AC12**: Add WSL-specific validation requirements

### 6.3 Routing Decision

**Status**: **FINALIZE → spec-finalizer**

**Reason**: Specification analysis complete with comprehensive architecture recommendations

**Required Actions for spec-finalizer**:
1. **Update Issue #207 specification** with corrections:
   - Remove non-existent `debug_adapter.rs` reference
   - Update native timeline to 3-5 weeks
   - Add AC16-AC19 new acceptance criteria

2. **Create testable acceptance criteria** in Issue #207:
   - Convert all ACs to `// AC:ID` tagged test functions
   - Map ACs to validation commands (e.g., `cargo test --test dap_breakpoint_latency`)
   - Add golden transcript fixtures for AC4, AC13

3. **Generate implementation plan** in Issue #207:
   - Phased timeline (Week 1-2: Bridge, Week 3-6: Native, Week 7-8: Hardening)
   - Critical path identification (AC6 Perl shim = 2 weeks)
   - Dependency management strategy (AC18)

4. **Link to architectural specifications**:
   - Reference: `docs/CRATE_ARCHITECTURE_GUIDE.md` for crate structure
   - Reference: `docs/LSP_IMPLEMENTATION_GUIDE.md` for protocol patterns
   - Reference: `docs/SECURITY_DEVELOPMENT_GUIDE.md` for security integration
   - Reference: `docs/INCREMENTAL_PARSING_GUIDE.md` for breakpoint validation

**Evidence Provided**:
- ✅ **Requirements validation**: 15 ACs assessed, 4 new ACs proposed
- ✅ **Technical feasibility**: Comprehensive architecture integration analysis
- ✅ **Performance assessment**: Validation against existing LSP targets
- ✅ **Security validation**: Enterprise security framework alignment
- ✅ **Risk mitigation**: Phased approach with clear contingency
- ✅ **Implementation roadmap**: 8-week phased delivery plan

---

## 7. Appendices

### Appendix A: Validation Commands Summary

**Phase 1 (Bridge) Validation**:
```bash
# AC1: Extension debugger contribution
cd vscode-extension && npm test

# AC2: Launch.json snippets
cargo test --test dap_launch_snippets -- windows macos linux

# AC3: Documentation completeness
cargo test --test dap_documentation_coverage -- AC3

# AC4: Basic workflow validation
cargo test --test bridge_workflow_tests
```

**Phase 2 (Native) Validation**:
```bash
# AC5: Protocol scaffolding
cargo test -p perl-dap --test protocol_compliance

# AC6: Perl shim tests
cd Devel-TSPerlDAP && prove -lv t/

# AC7: Breakpoint management
cargo test -p perl-dap --test breakpoint_validation

# AC8: Variables and scopes
cargo test -p perl-dap --test variable_rendering

# AC9: Stepping and control flow
cargo test -p perl-dap --test control_flow_performance

# AC10: Evaluate and REPL
cargo test -p perl-dap --test eval_security

# AC11: VS Code native integration
cd vscode-extension && npm test -- native

# AC12: Cross-platform compatibility
cargo test -p perl-dap --test cross_platform_validation
```

**Phase 3 (Hardening) Validation**:
```bash
# AC13: Integration tests
cargo test -p perl-dap --test integration_tests

# AC14: Performance benchmarks
cargo bench -p perl-dap

# AC15: Documentation completeness
cargo test --test dap_documentation_complete

# AC16 (NEW): Security validation
cargo test -p perl-dap --test security_validation

# AC17 (NEW): LSP non-regression
cargo test -p perl-lsp --test lsp_dap_non_regression

# AC18 (NEW): Dependency management
cargo test --test dap_dependency_installation

# AC19 (NEW): Binary packaging
cargo test --test dap_binary_packaging
```

### Appendix B: Architecture Integration Points

**Reusable Perl LSP Components**:
1. **JSON-RPC Protocol** (`perl-parser::JsonRpcRequest`)
2. **AST Integration** (`perl-parser::Parser`)
3. **Incremental Parsing** (`perl-parser::incremental_v2::IncrementalParserV2`)
4. **Workspace Index** (`perl-parser::workspace_index::WorkspaceIndex`)
5. **Position Mapping** (`perl-parser::textdoc::{lsp_pos_to_byte, byte_to_lsp_pos}`)
6. **Security Framework** (`docs/SECURITY_DEVELOPMENT_GUIDE.md`)

**New Components Required**:
1. **perl-dap crate** (new): DAP protocol implementation
2. **Devel::TSPerlDAP** (new): CPAN module for Perl shim
3. **VS Code debugger contribution** (new): Extension configuration
4. **DAP documentation** (new): Tutorial/Reference/Architecture/Troubleshooting

### Appendix C: Risk Mitigation Strategies

| Risk | Severity | Mitigation | Validation |
|---|---|---|---|
| No existing code | ⚠️ **MODERATE** | Phased approach validates first | Bridge delivers value week 1-2 |
| Perl debugger complexity | ⚠️ **HIGH** | Invest 2 weeks in robust shim | >80% test coverage (AC6) |
| CPAN maintenance burden | ⚠️ **MODERATE** | Bundled fallback + versioning | AC18 dependency management |
| Cross-platform testing | ⚠️ **MODERATE** | WSL-specific validation matrix | AC12 enhancement |
| Variable rendering perf | ⚠️ **LOW** | Lazy loading + 1KB truncation | AC8 already specifies |
| Bridge dependency | ✅ **LOW** | Temporary with native migration | Phased approach |
| LSP regression | ⚠️ **MODERATE** | Full LSP test suite validation | AC17 NEW |
| Security vulnerabilities | ✅ **LOW** | Enterprise security framework | AC16 NEW |

---

## Conclusion

Issue #207 (Debugger DAP Support) is a **well-structured specification** with 15 comprehensive acceptance criteria. This analysis recommends:

1. **Phased Implementation Strategy**: Bridge (Week 1-2) → Native (Week 3-6) → Hardening (Week 7-8)
2. **Four New Acceptance Criteria**: AC16 (Security), AC17 (LSP non-regression), AC18 (Dependency mgmt), AC19 (Binary packaging)
3. **Timeline Adjustment**: Native approach requires **3-5 weeks** (not 2-4) due to no existing code
4. **Strong LSP Integration**: Leverage existing parser, incremental parsing, workspace navigation, and security infrastructure

**Recommended Routing**: **FINALIZE → spec-finalizer** for final testable acceptance criteria creation.

**Next Agent Actions**:
- Update Issue #207 with corrections (remove non-existent code reference, adjust timeline)
- Add AC16-AC19 new acceptance criteria
- Create `// AC:ID` tagged test functions
- Link to architectural specifications (CRATE_ARCHITECTURE_GUIDE, LSP_IMPLEMENTATION_GUIDE, SECURITY_DEVELOPMENT_GUIDE)

---

**Generative Gate Status**: ✅ **PASS** - Comprehensive specification analysis complete

**Evidence**:
- Requirements validation: 15 original + 4 new ACs
- Technical feasibility: Comprehensive architecture integration analysis
- Performance/security: Enterprise standards validation
- Implementation roadmap: 8-week phased delivery plan
- Routing decision: Clear path to spec-finalizer with actionable recommendations
