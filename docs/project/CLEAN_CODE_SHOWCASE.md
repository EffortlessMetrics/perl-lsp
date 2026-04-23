# Clean Code Patterns Worth Stealing

Patterns from the perl-lsp codebase that could serve as reference implementations
for other Rust projects. Each section describes a real, working pattern with
code excerpts and the reasoning behind it.

---

## 1. Zero-Panic Production Code

The project enforces a strict no-fatal-constructs policy across all production
code. `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, and
`dbg!()` are all banned.

### How it is enforced

Two complementary mechanisms work together:

**Workspace-level Clippy denials** in `Cargo.toml`:

```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
```

**A custom CI tool** (`perl-ci-hygiene forbid-fatal-constructs`) scans all
`.rs` files in the `crates/` directory for `std::process::abort()` and
`std::process::exit()`. The tool allows `exit()` only in `bin/` directories
and `lifecycle.rs` (the LSP shutdown handler), and `abort()` is banned
everywhere. Test files, benchmarks, and designated utility crates are excluded.

### What they use instead

Where other projects would write `url.parse().unwrap()`, perl-lsp provides a
graceful fallback. The `perl-lsp-uri` crate (`crates/perl-lsp-uri/src/lib.rs`)
demonstrates this approach:

```rust
pub fn parse_uri(s: &str) -> Uri {
    match s.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => fallback_uri(),
    }
}
```

The `fallback_uri()` function tries several candidate URIs and always returns
a valid value -- no panics, no unwraps. This is the project's only historical
exception point for `expect()`, now refactored away entirely.

### Test helpers that replace unwrap

Since `unwrap()` is banned even in test code, the `perl-tdd-support` crate
(`crates/perl-tdd-support/src/must.rs`) provides `must()`, `must_some()`, and
`must_err()`:

```rust
#[track_caller]
pub fn must<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("unexpected Err: {e:?}"),
    }
}

#[track_caller]
pub fn must_some<T>(o: Option<T>) -> T {
    match o {
        Some(v) => v,
        None => panic!("unexpected None"),
    }
}
```

The `#[track_caller]` attribute means stack traces point to the _call site_,
not into the helper. The crate itself uses `#![allow(clippy::panic)]` because
these helpers are inherently test-only and intentionally panic on failure.

**Why this is clean**: The policy turns an entire class of production crashes
into compile errors. Combined with `#[track_caller]`, the test helpers provide
better diagnostics than `unwrap()` while staying policy-compliant.

---

## 2. Feature Governance as a Type System

The project implements a 7-crate subsystem to manage which LSP capabilities
are enabled at any given time. This is not simple feature flags -- it is a
type-safe governance layer that ensures compile-time and runtime consistency
across deployment profiles.

### The crate stack

| Crate | Role |
|-------|------|
| `perl-lsp-feature-ids` | Canonical string constants (`"lsp.completion"`, `"lsp.hover"`, etc.) |
| `perl-lsp-feature-flags` | `BuildFlags` and `AdvertisedFeatures` structs with named booleans |
| `perl-lsp-feature-contracts` | `FeatureProfileKind` enum, BDD feature rows, compliance math |
| `perl-lsp-feature-profile` | Profile parsing and normalization (`"ga"` -> `GaLock`) |
| `perl-lsp-feature-policy` | `FeatureProfile` with `build_flags()` and `runtime_flags()` |
| `perl-lsp-feature-grid` | Feature x profile matrix as JSON for reporting |
| `perl-lsp-feature-governance` | Composition root re-exporting the full API |

### How it works

Feature identifiers are centralized constants:

```rust
// perl-lsp-feature-ids/src/lib.rs
pub const LSP_COMPLETION: &str = "lsp.completion";
pub const LSP_HOVER: &str = "lsp.hover";
pub const LSP_DEFINITION: &str = "lsp.definition";
```

These flow through `BuildFlags` (a struct with 34 named booleans) with
preset constructors for each deployment profile:

```rust
impl BuildFlags {
    pub fn ga_lock() -> Self { /* conservative set */ }
    pub fn production() -> Self { /* standard set */ }
    pub fn all() -> Self { /* everything enabled */ }
}
```

The policy layer bridges user-facing CLI arguments to concrete flag sets:

```rust
pub fn runtime_flags(self, has_perltidy: bool) -> BuildFlags {
    let mut flags = self.build_flags();
    if has_perltidy {
        flags.formatting = true;
        flags.range_formatting = true;
    }
    flags
}
```

This means formatting is only advertised when `perltidy` is actually installed --
the capability set adapts to the runtime environment.

The grid layer produces JSON reports showing exactly which features are enabled
for each profile, with compliance percentages. Tests verify that the feature ID
catalog, the build flags, and the advertised feature sets all stay in sync:

```rust
#[test]
fn feature_ids_are_valid_in_catalog() {
    let ids = BuildFlags::all().to_feature_ids();
    let known_ids: HashSet<_> =
        all_features().iter().map(|f| f.id).collect();
    let unknown: Vec<_> = ids.iter()
        .filter(|id| !known_ids.contains(id)).collect();
    assert!(unknown.is_empty(),
        "non-catalog feature IDs emitted: {:?}", unknown);
}
```

**Why this is clean**: Feature flags usually drift. This system makes drift a
compile error or test failure. The separation into thin crates means each
concern has clear boundaries, and the governance facade provides a single
import for consumers.

---

## 3. Formal State Machines for Infrastructure

The workspace indexing lifecycle is modeled as an explicit state machine
(`crates/perl-workspace-index-state-machine/src/lib.rs`) with 8 states
and guarded transitions.

### States

```
Idle -> Initializing -> Building -> Ready
                                      |
                         Updating <---+
                                      |
                      Invalidating <--+
                                      |
                         Degraded <---+
                                      |
                            Error <---+
```

Each state carries rich context:

```rust
pub enum IndexState {
    Idle { since: Instant },
    Initializing { progress: u8, started_at: Instant },
    Building { phase: BuildPhase, indexed_count: usize,
               total_count: usize, started_at: Instant },
    Updating { updating_count: usize, started_at: Instant },
    Ready { symbol_count: usize, file_count: usize,
            completed_at: Instant },
    Degraded { reason: DegradationReason,
               available_symbols: usize, since: Instant },
    Error { message: String, since: Instant },
    // ...
}
```

### Guarded transitions

Each transition method validates the current state:

```rust
pub fn transition_to_building(&self, total_count: usize) -> TransitionResult {
    let mut state = self.state.write();
    match &*state {
        IndexState::Initializing { .. }
        | IndexState::Ready { .. }
        | IndexState::Degraded { .. } => {
            *state = IndexState::Building { /* ... */ };
            TransitionResult::Success
        }
        _ => TransitionResult::InvalidTransition {
            from: state.kind(),
            to: IndexStateKind::Building,
        },
    }
}
```

Invalid transitions return a structured error rather than panicking. The
`TransitionResult` enum distinguishes between invalid state flows and
failed guard conditions.

### Degradation modeling

The `DegradationReason` enum captures specific failure modes:

```rust
pub enum DegradationReason {
    ParseStorm { pending_parses: usize },
    IoError { message: String },
    ScanTimeout { elapsed_ms: u64 },
    ResourceLimit { kind: ResourceKind },
}
```

This means the system can degrade gracefully under load (a "parse storm" from
rapid file changes) while still serving queries from the partial index.

**Why this is clean**: Infrastructure state machines are often implicit (a mix
of booleans and status flags). Making the machine explicit, with typed
transitions and structured failure modes, eliminates an entire category of bugs
around invalid state combinations.

---

## 4. SLOs for Developer Tools

The project treats its parser like a production service with Service Level
Objectives (`crates/perl-workspace-index-slo/src/lib.rs`).

### Defined SLOs

```rust
impl Default for SloConfig {
    fn default() -> Self {
        Self {
            index_init_p95_ms: 5000,        // <5s for 10K files
            incremental_update_p95_ms: 100, // <100ms per file change
            definition_lookup_p95_ms: 50,   // <50ms per go-to-def
            completion_p95_ms: 100,         // <100ms per completion
            hover_p95_ms: 50,               // <50ms per hover
            max_error_rate: 0.01,           // <1% failure rate
            sample_window_size: 1000,
        }
    }
}
```

### How they are measured

The tracker collects latency samples in a sliding window and computes
percentiles (P50, P95, P99):

```rust
pub fn record_operation_type(
    &self, operation_type: OperationType,
    start: Instant, result: OperationResult,
) {
    let duration = start.elapsed();
    let mut trackers = self.trackers.lock();
    if let Some(tracker) = trackers.get_mut(&operation_type) {
        tracker.record(duration, result);
    }
}
```

SLO compliance is a single boolean:

```rust
let slo_met = p95_ms <= self.slo_target_ms
           && error_rate <= self.max_error_rate;
```

The `SloStatistics` struct exposes everything needed for dashboards:
`total_count`, `success_count`, `failure_count`, `error_rate`, `p50_ms`,
`p95_ms`, `p99_ms`, `avg_ms`, and `slo_met`.

**Why this is clean**: Most developer tools have no performance contracts.
By defining explicit SLOs with the same rigor as production services, the
project can detect regressions before users notice them. The sliding window
approach keeps memory bounded while still providing useful percentiles.

---

## 5. Structured Technical Debt

The project tracks technical debt in a machine-readable YAML ledger
(`.ci/debt-ledger.yaml`) with budgets, expiry dates, and CI enforcement.

### Budget system

```yaml
budgets:
  max_quarantined_tests: 10
  max_known_issues: 20
  max_technical_debt: 30
  warning_threshold_percent: 80
  critical_threshold_percent: 95
```

CI runs `just debt-check` and fails if any category exceeds its budget.

### Quarantined tests

Flaky tests are tracked with metadata, not just `#[ignore]`:

```yaml
flaky_tests:
  - name: "lsp::test_completion_timeout"
    added: "2026-01-24"
    issue: "#198"
    tier: "quarantine"        # runs but doesn't block
    quarantine_days: 14
    expires: "2026-02-07"
    owner: "maintainer-username"
    failure_pattern: "timeout waiting for completion"
    affected_platforms: ["windows", "wsl"]
```

Quarantines auto-expire. When they do, CI creates an issue -- the test must be
fixed or the quarantine explicitly renewed. This prevents the common pattern
of ignored tests accumulating silently.

### Historical tracking

The ledger includes a `history` section with weekly summaries and resolved
items, making debt trends visible:

```yaml
weekly_summaries:
  - week: "2026-W09"
    quarantined_tests: 0
    known_issues: 0
    technical_debt: 4
    added: 1
    resolved: 7
```

**Why this is clean**: Technical debt is usually invisible until it becomes
a crisis. This system makes debt a measurable, budgeted quantity with
automatic enforcement. The weekly trend data makes cleanup campaigns
demonstrably effective.

---

## 6. Dual Indexing for IDE Intelligence

When building the workspace symbol index, every function call is indexed
under both its bare name and its fully-qualified name
(`crates/perl-workspace-index/src/workspace/workspace_index.rs`):

```rust
// Determine package and bare name
let (pkg, bare_name) = if let Some(idx) = func_name.rfind("::") {
    (&func_name[..idx], &func_name[idx + 2..])
} else {
    (self.current_package.as_deref().unwrap_or("main"),
     func_name.as_str())
};

let qualified = format!("{}::{}", pkg, bare_name);

// Index under bare name
file_index.references.entry(bare_name.to_string())
    .or_default()
    .push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified)
    .or_default()
    .push(symbol_ref);
```

This means "Find All References" for `process_data` will find both
`process_data()` and `Utils::process_data()` call sites. Without dual
indexing, one form or the other would be missed, producing incomplete results.

**Why this is clean**: The extra memory cost of a second index entry per
symbol is negligible compared to the completeness improvement. The pattern
is simple, local, and easy to verify -- no complex name resolution at
query time.

---

## 7. Incremental Everything

The incremental parsing system (`crates/perl-incremental-parsing/`) is
designed around subtree reuse with priority-aware cache eviction.

### Subtree cache with priority eviction

```rust
pub struct SubtreeCache {
    pub by_content: HashMap<u64, Arc<Node>>,
    pub by_range: HashMap<(usize, usize), Arc<Node>>,
    pub lru: VecDeque<u64>,
    pub critical_symbols: HashMap<u64, SymbolPriority>,
    pub max_size: usize,
}

pub enum SymbolPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}
```

Package declarations and `use` statements are marked `Critical` and evicted
last, because they affect name resolution across the entire file. Comment
blocks are `Low` priority. This means under memory pressure, the cache
preserves the nodes that matter most for IDE features.

### Incremental state with checkpoints

The `IncrementalState` maintains lexer and parser checkpoints so re-lexing
and re-parsing can resume from the nearest safe point rather than the
beginning of the file:

```rust
pub struct IncrementalState {
    pub rope: Rope,
    pub line_index: LineIndex,
    pub lex_checkpoints: Vec<LexCheckpoint>,
    pub parse_checkpoints: Vec<ParseCheckpoint>,
    pub ast: Node,
    pub tokens: Vec<Token>,
    pub source: String,
}
```

Even the initial parse uses graceful degradation -- if parsing fails, an
`Error` node is created instead of propagating a panic:

```rust
let ast = match parser.parse() {
    Ok(ast) => ast,
    Err(e) => Node::new(
        NodeKind::Error { message: e.to_string(), /* ... */ },
        SourceLocation { start: 0, end: source.len() },
    ),
};
```

**Why this is clean**: The priority-aware eviction is a small addition to a
standard LRU cache that produces outsized benefits for IDE responsiveness.
The checkpoint system bounds re-parsing to the edited region rather than
the entire file.

---

## 8. Stable Diagnostic Codes

The project uses a rustc-inspired diagnostic code system
(`crates/perl-diagnostics-codes/src/lib.rs`) with stable, categorized codes
and documentation URLs.

### Code ranges

| Range | Category |
|-------|----------|
| PL001-PL099 | Parser diagnostics |
| PL100-PL199 | Strict/warnings |
| PL200-PL299 | Package/module |
| PL300-PL399 | Subroutine |
| PL400-PL499 | Best practices |
| PC001-PC005 | Perl::Critic violations |

### Rich metadata per code

```rust
impl DiagnosticCode {
    pub fn as_str(&self) -> &'static str { /* "PL001" */ }
    pub fn severity(&self) -> DiagnosticSeverity { /* Error, Warning, Hint */ }
    pub fn category(&self) -> DiagnosticCategory { /* Parser, StrictWarnings, ... */ }
    pub fn tags(&self) -> &'static [DiagnosticTag] { /* Unnecessary, Deprecated */ }
    pub fn documentation_url(&self) -> Option<&'static str> {
        /* "https://docs.perl-lsp.org/errors/PL001" */
    }
}
```

The diagnostic catalog crate (`crates/perl-lsp-diagnostic-catalog/`) provides
convenience constructors and message inference:

```rust
pub fn from_message(msg: &str) -> Option<DiagnosticMeta> {
    DiagnosticCode::from_message(msg).map(diagnostic_meta)
}
```

This means even free-form error messages from external tools (like
`perlcritic`) can be mapped to stable codes for consistent reporting.

**Why this is clean**: Stable diagnostic codes are a user experience
feature. Users can search for "PL100" and find documentation. The code
is a zero-dependency leaf crate, so any tool in the ecosystem can use it
without pulling in the parser.

---

## 9. Budget-Bounded Error Recovery

The parser uses a `ParseBudget` system (`crates/perl-error/src/lib.rs`) to
prevent runaway parsing on malformed or adversarial input:

```rust
pub struct ParseBudget {
    pub max_errors: usize,       // default: 100
    pub max_depth: usize,        // default: 256
    pub max_tokens_skipped: usize, // default: 1000
    pub max_recoveries: usize,   // default: 500
}
```

With named presets for different trust levels:

```rust
impl ParseBudget {
    pub fn for_ide() -> Self { Self::default() }
    pub fn strict() -> Self {
        Self { max_errors: 10, max_depth: 64,
               max_tokens_skipped: 100, max_recoveries: 50 }
    }
    pub fn unlimited() -> Self { /* usize::MAX for everything */ }
}
```

The `BudgetTracker` provides atomic check-and-consume operations:

```rust
pub fn begin_recovery(&mut self, budget: &ParseBudget) -> bool {
    if self.recoveries_attempted >= budget.max_recoveries {
        return false;
    }
    self.recoveries_attempted = self.recoveries_attempted.saturating_add(1);
    true
}
```

The `ParseOutput` struct combines the AST, collected diagnostics, and budget
usage into a single return value:

```rust
pub struct ParseOutput {
    pub ast: Node,
    pub diagnostics: Vec<ParseError>,
    pub budget_usage: BudgetTracker,
    pub terminated_early: bool,
}
```

This means the parser always produces a partial AST -- even when errors occur --
enabling IDE features to work on broken code. The budget system guarantees the
parser terminates in bounded time regardless of input quality.

**Why this is clean**: Error recovery in parsers is usually ad-hoc. The budget
system makes resource limits explicit, configurable, and testable. The
`ParseOutput` type eliminates the false dichotomy of "parsed successfully" vs.
"failed" by always returning both an AST and diagnostics.

---

## 10. The Micro-Crate as Architecture Pattern

The workspace contains 121 crates organized in dependency tiers. This is not
accidental -- it is a deliberate architectural strategy where each crate has
a single responsibility.

### Benefits observed in this codebase

**Compile-time dependency enforcement**: A crate in Tier 1 (leaf) physically
cannot depend on a Tier 4 crate. The compiler enforces architectural
boundaries that code review would miss.

**Parallel compilation**: With 121 small crates, `cargo` can parallelize
builds effectively. Changing a leaf crate only triggers rebuilds of its
dependents, not the entire workspace.

**API surface control**: Each crate's `lib.rs` exports exactly what its
consumers need. The feature governance subsystem demonstrates this -- 7 crates
with clearly defined boundaries, composed through a facade crate.

**Independent versioning and testing**: `cargo test -p perl-diagnostics-codes`
runs in milliseconds and tests exactly one concern. This makes TDD practical
even in a large codebase.

### The tier system

| Tier | Example Crates | Characteristic |
|------|----------------|----------------|
| 1 | `perl-token`, `perl-ast`, `perl-diagnostics-codes` | Zero workspace deps |
| 2 | `perl-parser-core`, `perl-lsp-transport` | One level of deps |
| 3 | `perl-workspace-index`, `perl-lsp-feature-governance` | Two levels |
| 4 | `perl-semantic-analyzer`, `perl-lsp-providers` | Three levels |
| 5 | `xtask` | Build tooling |
| 6 | `perl-lsp`, `perl-dap` | Application binaries |

**Why this is clean**: The micro-crate pattern makes architecture visible and
enforceable. Circular dependencies are impossible. Rebuild times scale with
the change, not the project size. The tradeoff is more `Cargo.toml` files to
maintain, but workspace inheritance (shared versions, shared lints) keeps that
manageable.
