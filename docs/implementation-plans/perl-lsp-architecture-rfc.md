# perl-lsp Unified Architecture RFC

**Date**: 2026-04-09  
**Scope**: Addressing 113 filed issues through 3 foundational architectures  
**Status**: Design Phase

---

## Executive Summary

This RFC proposes three unified architectures to address the recurring architectural gaps identified across 113 filed issues in the perl-lsp project. Rather than one-off patches, these architectures provide comprehensive, reusable solutions:

1. **`EffectiveSemantics` Layer** - Unified version/pragma/feature matrix
2. **`BUILTIN_INITIALIZERS` PHF Registry** - Systematic builtin variable-declaration handling
3. **Parser `deref_base` Annotation** - Sigil bridging for complex dereference chains

These architectures resolve categories of issues including: version compatibility diagnostics (PL900), builtin parameter handling for variable auto-declaration (read/sysread/recv), and scope analyzer sigil-bridging for complex dereference patterns.

---

## Architecture 1: `EffectiveSemantics` Layer

### Purpose

Provide a unified, queryable matrix of Perl version, pragma, and feature state at any point in source code. Replaces ad-hoc version tracking in `version_compat.rs` and extends `PragmaState` from `perl-pragma`.

### Problem Statement

The 113 filed issues include 27 errors related to version handling (`expected_module_name` for v-strings) and scattered pragma tracking gaps. Currently:

- `PragmaState` only tracks `strict_vars`, `strict_subs`, `strict_refs`, `warnings`
- `version_compat.rs` maintains a separate `FEATURE_VERSIONS` table
- No unified query interface exists for "what features are enabled at this offset?"
- Feature-to-version mapping is duplicated between `features_enabled_by_version()` and `FEATURE_VERSIONS`

### Design

#### Location
Extend `crates/perl-pragma/src/lib.rs` with new types, or create `crates/perl-effective-semantics/` if circular dependencies arise.

#### Core Types

```rust
/// A Perl version in comparable form
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerlVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
}

impl PerlVersion {
    pub const V5_10: Self = Self { major: 5, minor: 10, patch: None };
    pub const V5_20: Self = Self { major: 5, minor: 20, patch: None };
    pub const V5_36: Self = Self { major: 5, minor: 36, patch: None };
    pub const V5_38: Self = Self { major: 5, minor: 38, patch: None };
    
    /// Parse from "v5.36", "5.036", "v5.36.0"
    pub fn parse(s: &str) -> Option<Self> { /* ... */ }
}

/// A named feature that can be enabled/disabled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    Say,
    State,
    PostfixDeref,
    Signatures,
    Try,
    Class,
    Field,
    Method,
    // Extensible: new features added here
}

impl Feature {
    /// Minimum version where feature is available
    pub fn min_version(&self) -> PerlVersion {
        match self {
            Feature::Say | Feature::State => PerlVersion::V5_10,
            Feature::PostfixDeref => PerlVersion::V5_20,
            Feature::Signatures => PerlVersion::V5_36, // stable bundle
            Feature::Try => PerlVersion { major: 5, minor: 34, patch: None },
            Feature::Class | Feature::Field | Feature::Method => PerlVersion::V5_38,
        }
    }
    
    /// Features implicitly enabled by a version declaration
    pub fn features_enabled_by_version(v: PerlVersion) -> Vec<Feature> {
        let mut features = Vec::new();
        if v >= PerlVersion::V5_10 {
            features.extend([Feature::Say, Feature::State]);
        }
        if v >= PerlVersion::V5_20 {
            features.push(Feature::PostfixDeref);
        }
        if v >= PerlVersion { major: 5, minor: 34, patch: None } {
            features.push(Feature::Try);
        }
        if v >= PerlVersion::V5_36 {
            features.push(Feature::Signatures);
        }
        if v >= PerlVersion::V5_38 {
            features.extend([Feature::Class, Feature::Field, Feature::Method]);
        }
        features
    }
}

/// Extended pragma state beyond the current PragmaState
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtendedPragmaState {
    // From existing PragmaState
    pub strict_vars: bool,
    pub strict_subs: bool,
    pub strict_refs: bool,
    pub warnings: bool,
    // New: pragma categories
    pub utf8: bool,
    pub re_strict: bool,      // re 'strict'
    pub re_eval: bool,        // re 'eval'
    pub feature_bundle: Option<FeatureBundle>, // implied vs explicit
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeatureBundle {
    Implied(PerlVersion),     // from "use v5.36"
    Explicit(Vec<Feature>),   // from "use feature qw(say state)"
}

/// The unified effective semantics at a source location
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveSemantics {
    /// Declared Perl version (from use vX.XX or use X.XXX)
    pub declared_version: Option<PerlVersion>,
    /// Effective feature set (version-implied + explicit - no)
    pub features: FxHashSet<Feature>,
    /// Extended pragma state
    pub pragmas: ExtendedPragmaState,
    /// Source location where this state begins
    pub effective_from: usize,  // byte offset
}

impl EffectiveSemantics {
    /// Query: Is a specific feature enabled?
    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }
    
    /// Query: Is a feature enabled by name (for lint integration)
    pub fn has_feature_by_name(&self, name: &str) -> bool {
        Feature::from_name(name)
            .map(|f| self.has_feature(f))
            .unwrap_or(false)
    }
    
    /// Query: Is strict mode active for a specific category?
    pub fn is_strict(&self, category: StrictCategory) -> bool {
        match category {
            StrictCategory::Vars => self.pragmas.strict_vars,
            StrictCategory::Subs => self.pragmas.strict_subs,
            StrictCategory::Refs => self.pragmas.strict_refs,
            StrictCategory::Any => self.pragmas.strict_vars 
                || self.pragmas.strict_subs 
                || self.pragmas.strict_refs,
        }
    }
    
    /// Query: What is the minimum compatible Perl version for this code?
    pub fn min_required_version(&self) -> PerlVersion {
        let mut min = PerlVersion { major: 5, minor: 8, patch: None }; // baseline
        for feature in &self.features {
            let feature_min = feature.min_version();
            if feature_min > min {
                min = feature_min;
            }
        }
        min
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictCategory {
    Vars,
    Subs,
    Refs,
    Any,
}
```

#### Storage and Population

```rust
/// Builder for creating range-indexed effective semantics
pub struct EffectiveSemanticsTracker;

impl EffectiveSemanticsTracker {
    /// Build a range-indexed map from AST
    pub fn build(ast: &Node) -> Vec<(Range<usize>, EffectiveSemantics)> {
        let mut ranges = Vec::new();
        let mut current = EffectiveSemantics::default();
        
        Self::build_ranges(ast, &mut current, &mut ranges);
        ranges.sort_by_key(|(r, _)| r.start);
        ranges
    }
    
    /// Get effective semantics at a specific byte offset
    pub fn state_for_offset(
        states: &[(Range<usize>, EffectiveSemantics)],
        offset: usize,
    ) -> EffectiveSemantics {
        let idx = states.partition_point(|(r, _)| r.start <= offset);
        if idx > 0 {
            states[idx - 1].1.clone()
        } else {
            EffectiveSemantics::default()
        }
    }
    
    fn build_ranges(
        node: &Node,
        current: &mut EffectiveSemantics,
        ranges: &mut Vec<(Range<usize>, EffectiveSemantics)>,
    ) {
        match &node.kind {
            NodeKind::Use { module, args, .. } => {
                // Version declaration: use v5.36
                if let Some(version) = Self::parse_version_use(module) {
                    current.declared_version = Some(version);
                    current.features = Feature::features_enabled_by_version(version)
                        .into_iter()
                        .collect();
                    current.pragmas.feature_bundle = Some(FeatureBundle::Implied(version));
                    ranges.push((node.location.clone(), current.clone()));
                }
                // Feature pragma: use feature 'say'
                else if module == "feature" {
                    let explicit: Vec<Feature> = args.iter()
                        .filter_map(|a| Feature::from_name(a.trim_matches('\'')))
                        .collect();
                    for f in &explicit {
                        current.features.insert(*f);
                    }
                    current.pragmas.feature_bundle = Some(FeatureBundle::Explicit(explicit));
                    ranges.push((node.location.clone(), current.clone()));
                }
                // Strict pragma
                else if module == "strict" {
                    Self::update_strict(&mut current.pragmas, args, true);
                    ranges.push((node.location.clone(), current.clone()));
                }
                // Other pragmas...
            }
            NodeKind::No { module, args, .. } => {
                // Handle "no strict", "no warnings", "no feature"
                if module == "feature" {
                    for arg in args {
                        if let Some(f) = Feature::from_name(arg.trim_matches('\'')) {
                            current.features.remove(&f);
                        }
                    }
                    ranges.push((node.location.clone(), current.clone()));
                }
                else if module == "strict" {
                    Self::update_strict(&mut current.pragmas, args, false);
                    ranges.push((node.location.clone(), current.clone()));
                }
            }
            // Block scoping: pragmas may reset after block
            NodeKind::Block { .. } | NodeKind::Program { .. } => {
                let saved = current.clone();
                for child in node.children() {
                    Self::build_ranges(child, current, ranges);
                }
                *current = saved;  // Restore state after block
            }
            _ => {
                for child in node.children() {
                    Self::build_ranges(child, current, ranges);
                }
            }
        }
    }
}
```

### Integration Points

| Component | Current | With EffectiveSemantics |
|-----------|---------|------------------------|
| `version_compat.rs` | Manual version parsing, duplicate feature tables | Query `EffectiveSemantics::state_for_offset()` |
| `PragmaTracker` | Returns `PragmaState` | Returns `EffectiveSemantics` (superset) |
| `ScopeAnalyzer` | Takes `PragmaState` | Takes `EffectiveSemantics` for richer context |
| Lints (PL900) | `features_enabled_by_version()` function | `semantics.has_feature(Feature::Say)` |

### Issues Resolved

From the 113 filed issues:

| Issue Category | Count | How Resolved |
|----------------|-------|--------------|
| `expected_module_name` (v-strings in use) | 27 | Centralized version parsing in `PerlVersion::parse()` |
| Version compatibility false positives (PL900) | ~15 | Unified feature matrix eliminates duplicate logic errors |
| Pragma state edge cases | ~8 | Block-scoped pragma restoration |
| Missing feature detection | ~12 | Explicit feature bundle tracking |

**Total**: ~62 issues (55% of 113)

### Implementation Phases

#### MVP (Week 1-2)

1. Define `PerlVersion`, `Feature`, `EffectiveSemantics` structs
2. Implement `PerlVersion::parse()` handling v-strings
3. Migrate `version_compat.rs` to use new types
4. Add tests for v-string parsing (resolves 27 `expected_module_name` errors)

#### Full Implementation (Week 3-4)

1. Extend `PragmaState` → `ExtendedPragmaState`
2. Implement `EffectiveSemanticsTracker::build_ranges()`
3. Migrate `PragmaTracker` to return `EffectiveSemantics`
4. Add query methods: `has_feature()`, `is_strict()`, `min_required_version()`

#### Rollout (Week 5-6)

1. Update all lints to use unified query interface
2. Deprecate duplicate feature tables in `version_compat.rs`
3. Performance: benchmark range-indexed lookup vs. current implementation
4. Documentation: migration guide for downstream crates

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking change to `PragmaTracker` API | High | Medium | Maintain backward compat: `impl From<EffectiveSemantics> for PragmaState` |
| Performance regression in range queries | Low | High | Benchmark against current; optimize with binary search (already used) |
| Version parsing edge cases | Medium | Medium | Extensive test corpus including all 27 failing v-string cases |
| Circular deps with `perl-ast` | Medium | Low | Keep in `perl-pragma` or extract to micro-crate |

---

## Architecture 2: `BUILTIN_INITIALIZERS` PHF Registry

### Purpose

Systematic handling of Perl builtins that auto-declare variables at specific parameter positions (read, sysread, recv, socketpair, dbmopen, etc.) via a PHF (perfect hash function) registry.

### Problem Statement

Among the 113 issues, builtin-related issues include:

- Variable initialization false positives: `read FH, $buf, 1024` should mark `$buf` as initialized
- Scope analyzer doesn't know which builtin parameters are output vs input
- No centralized registry of builtin "effects" beyond signatures
- Duplicate logic in `scope_analyzer.rs` for detecting builtins

Current state in `perl-builtins-phf` only tracks parameter *names*, not their *semantics* (input, output, declared, etc.).

### Design

#### Location
`crates/perl-builtins/src/lib.rs` or new `crates/perl-builtins-semantic/`

#### Core Types

```rust
/// Semantic role of a builtin parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamRole {
    /// Read-only input (e.g., LENGTH in read())
    Input,
    /// Output: variable is written to (e.g., SCALAR in read())
    OutputScalar,
    /// Output: array is written to (e.g., socketpair's returned handles)
    OutputArray,
    /// Output: hash is populated (e.g., dbmopen)
    OutputHash,
    /// Both input and output (e.g., buffer in sysread with offset)
    InOut,
    /// Filehandle/socket handle (may auto-create globs)
    Handle,
    /// Format string (special parsing rules)
    Format,
    /// Callback/code reference
    CodeRef,
    /// Optional: may be omitted
    Optional,
}

/// Declaration effect at a specific parameter position
#[derive(Debug, Clone, PartialEq)]
pub struct ParamEffect {
    /// 0-indexed position in argument list
    pub position: usize,
    /// Semantic role
    pub role: ParamRole,
    /// Variable type this parameter declares (if Output*)
    pub declares: Option<SigilKind>,
    /// Name hint for diagnostics (e.g., "SCALAR", "FILEHANDLE")
    pub name_hint: &'static str,
}

/// Built-in function semantic configuration
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinConfig {
    /// Function name
    pub name: &'static str,
    /// Parameter effects by position
    pub params: &'static [ParamEffect],
    /// Does this builtin declare variables at call site?
    pub has_declaration_effects: bool,
    /// Category for documentation/organization
    pub category: BuiltinCategory,
    /// Minimum Perl version required
    pub min_version: Option<PerlVersion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SigilKind {
    Scalar,   // $
    Array,    // @
    Hash,     // %
    Sub,      // &
    Glob,     // *
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCategory {
    Io,           // read, sysread, print, open
    String,       // substr, index, length
    Array,        // push, pop, splice
    Hash,         // keys, values, each
    Socket,       // socket, socketpair, recv, send
    Database,     // dbmopen, dbmclose
    Process,      // fork, wait, pipe
    Misc,         // grep, map, sort
}

/// PHF-backed registry of builtin configurations
pub static BUILTIN_CONFIGS: phf::Map<&'static str, BuiltinConfig> = phf_map! {
    // I/O builtins with output parameters
    "read" => BuiltinConfig {
        name: "read",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: None, name_hint: "FILEHANDLE" },
            ParamEffect { position: 1, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "SCALAR" },
            ParamEffect { position: 2, role: ParamRole::Input, declares: None, name_hint: "LENGTH" },
            ParamEffect { position: 3, role: ParamRole::Input, declares: None, name_hint: "OFFSET" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    "sysread" => BuiltinConfig {
        name: "sysread",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: None, name_hint: "FILEHANDLE" },
            ParamEffect { position: 1, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "SCALAR" },
            ParamEffect { position: 2, role: ParamRole::Input, declares: None, name_hint: "LENGTH" },
            ParamEffect { position: 3, role: ParamRole::Input, declares: None, name_hint: "OFFSET" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    "recv" => BuiltinConfig {
        name: "recv",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: None, name_hint: "SOCKET" },
            ParamEffect { position: 1, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "SCALAR" },
            ParamEffect { position: 2, role: ParamRole::Input, declares: None, name_hint: "LENGTH" },
            ParamEffect { position: 3, role: ParamRole::Input, declares: None, name_hint: "FLAGS" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Socket,
        min_version: None,
    },
    "socketpair" => BuiltinConfig {
        name: "socketpair",
        params: &[
            ParamEffect { position: 0, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "READ_HANDLE" },
            ParamEffect { position: 1, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "WRITE_HANDLE" },
            ParamEffect { position: 2, role: ParamRole::Input, declares: None, name_hint: "DOMAIN" },
            ParamEffect { position: 3, role: ParamRole::Input, declares: None, name_hint: "TYPE" },
            ParamEffect { position: 4, role: ParamRole::Input, declares: None, name_hint: "PROTOCOL" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Socket,
        min_version: Some(PerlVersion { major: 5, minor: 8, patch: None }),
    },
    "dbmopen" => BuiltinConfig {
        name: "dbmopen",
        params: &[
            ParamEffect { position: 0, role: ParamRole::OutputHash, declares: Some(SigilKind::Hash), name_hint: "HASH" },
            ParamEffect { position: 1, role: ParamRole::Input, declares: None, name_hint: "DBNAME" },
            ParamEffect { position: 2, role: ParamRole::Input, declares: None, name_hint: "MODE" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Database,
        min_version: None,
    },
    "pipe" => BuiltinConfig {
        name: "pipe",
        params: &[
            ParamEffect { position: 0, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "READ_END" },
            ParamEffect { position: 1, role: ParamRole::OutputScalar, declares: Some(SigilKind::Scalar), name_hint: "WRITE_END" },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Process,
        min_version: None,
    },
    // Additional builtins without declaration effects...
    "open" => BuiltinConfig {
        name: "open",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: Some(SigilKind::Glob), name_hint: "FILEHANDLE" },
            // ... remaining params
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    // ... etc for all builtins
};
```

#### Integration with SymbolExtractor

```rust
/// Trait for analyzers that need builtin-aware variable tracking
pub trait BuiltinAwareAnalyzer {
    /// Check if a function call has builtin declaration effects
    fn analyze_builtin_effects(
        &mut self,
        func_name: &str,
        args: &[Node],
        scope: &Rc<Scope>,
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        
        if let Some(config) = BUILTIN_CONFIGS.get(func_name) {
            if !config.has_declaration_effects {
                return issues;
            }
            
            for effect in config.params {
                if effect.declares.is_none() {
                    continue;
                }
                
                // Get the argument at this position
                let Some(arg) = args.get(effect.position) else {
                    continue;
                };
                
                // Extract variable name from argument
                if let Some((sigil, name)) = extract_variable_from_arg(arg) {
                    // Mark as declared and initialized
                    if let Some(issue) = scope.declare_variable_parts(
                        sigil,
                        name,
                        arg.location.start,
                        false,  // not 'our'
                        true,   // initialized by builtin
                    ) {
                        issues.push(ScopeIssue {
                            kind: issue,
                            variable_name: format!("{}{}", sigil, name),
                            line: 0, // resolved via context.get_line()
                            range: (arg.location.start, arg.location.end),
                            description: format!(
                                "Builtin '{}' declares '{}' at position {}",
                                func_name, name, effect.position
                            ),
                        });
                    }
                    
                    // Also mark as used (since builtin writes to it)
                    scope.initialize_and_use_variable_parts(sigil, name);
                }
            }
        }
        
        issues
    }
}
```

### Issues Resolved

| Issue Category | Count | How Resolved |
|----------------|-------|--------------|
| False positive "uninitialized variable" after read/sysread | ~18 | `ParamEffect::OutputScalar` marks buffer as initialized |
| False positive "undeclared variable" after recv | ~5 | `declares: Some(SigilKind::Scalar)` registers variable |
| Socketpair handle detection | ~4 | Position 0,1 marked as `OutputScalar` with `declares` |
| dbmopen hash declaration | ~3 | Position 0 marked as `OutputHash` |
| pipe handle detection | ~2 | Both positions marked as `OutputScalar` |

**Total**: ~32 issues (28% of 113)

### Implementation Phases

#### MVP (Week 1-2)

1. Define `BuiltinConfig`, `ParamEffect`, `ParamRole` types
2. Implement registry for 6 high-impact builtins: read, sysread, recv, socketpair, dbmopen, pipe
3. Add `extract_variable_from_arg()` helper for argument→variable extraction
4. Integrate with `ScopeAnalyzer` for declaration effects

#### Full Implementation (Week 3-4)

1. Extend registry to all 100+ builtins in `BUILTIN_SIGS`
2. Categorize builtins (Io, String, Array, Hash, Socket, etc.)
3. Add version-gated builtins (min_version field)
4. Integration with inlay hints: use `name_hint` for parameter labels

#### Rollout (Week 5)

1. Replace ad-hoc builtin detection in `scope_analyzer.rs`
2. Add comprehensive tests for each `has_declaration_effects` builtin
3. Document registry for external contributors

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PHF compile time increase | Medium | Low | Split into feature-gated modules |
| Runtime lookup overhead | Low | Low | PHF is O(1) with perfect hashing |
| Incomplete builtin coverage | Medium | Medium | Staged rollout: critical builtins first |
| Argument extraction edge cases | High | Medium | Extensive test cases for each builtin |

---

## Architecture 3: Parser `deref_base` Annotation

### Purpose

Enable sigil-bridging for complex dereference chains by annotating AST nodes with their "deref base" - the variable that serves as the root of a dereference chain (e.g., `ref` in `$ref->{key}`).

### Problem Statement

The `scope_analyzer.rs` already attempts sigil-bridging with this code:

```rust
// Check parent for hash/array access context
if let Some(parent) = ancestors.last() {
    if let NodeKind::Binary { op, left, .. } = &parent.kind {
        if std::ptr::eq(left.as_ref(), node) {
            if op == "{}" || op == "->{}" {
                // Check if the corresponding hash exists
                let (hash_used, hash_init) = scope.use_variable_parts("%", name);
                // ...
            }
        }
    }
}
```

Problems with current approach:

1. **Ancestor traversal is fragile**: Relies on pointer comparison (`std::ptr::eq`)
2. **No explicit deref chain tracking**: Complex chains like `$obj->{a}->{b}[0]` lose intermediate variable info
3. **Cannot track through method calls**: `$ref->method()->{key}` breaks the chain
4. **No annotation for later analysis**: Each analyzer must rediscover the relationship

### Design

#### Location
`crates/perl-parser-core/src/engine/parser/expressions.rs` (parser) and `crates/perl-ast/src/ast.rs` (AST)

#### AST Changes

```rust
// In perl-ast/src/ast.rs

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // ... existing variants
    
    /// Variable with optional dereference base annotation
    Variable {
        sigil: String,
        name: String,
        /// For `$ref->{key}` this points to the `ref` variable node
        deref_base: Option<ExprId>,  // NEW FIELD
    },
    
    /// Binary operation with deref context
    Binary {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
        /// If this is a dereference operation, the base expression
        deref_context: Option<DerefContext>,  // NEW FIELD
    },
    
    // ... other variants
}

/// Unique identifier for expressions in an AST
/// Used to cross-reference nodes without boxing
pub type ExprId = u32;

/// Context for dereference operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerefContext {
    /// The base expression being dereferenced
    pub base_id: ExprId,
    /// Type of dereference
    pub deref_type: DerefType,
    /// Chain depth (0 = direct, 1 = $a->{b}->{c}, etc.)
    pub chain_depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerefType {
    HashElement,      // $hash{key} or $ref->{key}
    ArrayElement,     // $array[index] or $ref->[index]
    MethodCall,       // $obj->method() (may return deref'able)
    PostfixSlice,     // $ref->@{...} or $ref->%[...]
    CodeDeref,        // $coderef->(@args)
}

/// Dereference chain information for scope analysis
#[derive(Debug, Clone, PartialEq)]
pub struct DerefChain {
    /// Root variable of the chain (e.g., `ref` in `$ref->{a}->{b}`)
    pub root_variable: (String, String),  // (sigil, name)
    /// Intermediate variables in the chain
    pub intermediate_vars: Vec<(String, String)>,
    /// Final access type
    pub final_access: DerefType,
    /// Complete chain depth
    pub total_depth: usize,
}
```

#### Parser Changes (expressions/postfix.rs)

```rust
impl<'a> Parser<'a> {
    /// Parse postfix chain with deref base tracking
    fn parse_postfix_chain_with_deref(&mut self, mut expr: Node) -> ParseResult<Node> {
        let mut deref_chain: Vec<Node> = vec![expr.clone()];
        let mut expr_id_counter: ExprId = 0;
        
        loop {
            match self.peek_kind() {
                Some(TokenKind::Arrow) => {
                    self.tokens.next()?; // consume ->
                    
                    match self.peek_kind() {
                        Some(TokenKind::LeftBrace) => {
                            // $ref->{key}
                            let base_id = expr_id_counter;
                            expr_id_counter += 1;
                            
                            self.tokens.next()?; // consume {
                            let key = self.parse_hash_subscript_key()?;
                            self.expect(TokenKind::RightBrace)?;
                            
                            // Annotate the key node with deref context if it's a simple variable
                            let key_with_context = if let NodeKind::Variable { sigil, name, .. } = &key.kind {
                                Node::new(
                                    NodeKind::Variable {
                                        sigil: sigil.clone(),
                                        name: name.clone(),
                                        deref_base: Some(base_id),
                                    },
                                    key.location.clone(),
                                )
                            } else {
                                key
                            };
                            
                            let start = expr.location.start;
                            let end = self.previous_position();
                            
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: "->{}".to_string(),
                                    left: Box::new(expr),
                                    right: Box::new(key_with_context),
                                    deref_context: Some(DerefContext {
                                        base_id,
                                        deref_type: DerefType::HashElement,
                                        chain_depth: deref_chain.len() as u8,
                                    }),
                                },
                                SourceLocation { start, end },
                            );
                            deref_chain.push(expr.clone());
                        }
                        
                        Some(TokenKind::LeftBracket) => {
                            // $ref->[index]
                            let base_id = expr_id_counter;
                            expr_id_counter += 1;
                            
                            self.tokens.next()?; // consume [
                            let index = self.parse_expression()?;
                            self.expect_closing_delimiter(TokenKind::RightBracket)?;
                            
                            let start = expr.location.start;
                            let end = self.previous_position();
                            
                            expr = Node::new(
                                NodeKind::Binary {
                                    op: "->[]".to_string(),
                                    left: Box::new(expr),
                                    right: Box::new(index),
                                    deref_context: Some(DerefContext {
                                        base_id,
                                        deref_type: DerefType::ArrayElement,
                                        chain_depth: deref_chain.len() as u8,
                                    }),
                                },
                                SourceLocation { start, end },
                            );
                            deref_chain.push(expr.clone());
                        }
                        
                        Some(kind) if Self::can_be_sub_name(kind) => {
                            // $obj->method() - may continue chain
                            let method = self.consume_token()?.text.to_string();
                            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                                self.parse_args()?
                            } else {
                                Vec::new()
                            };
                            
                            let start = expr.location.start;
                            let end = self.previous_position();
                            
                            expr = Node::new(
                                NodeKind::MethodCall {
                                    object: Box::new(expr),
                                    method,
                                    args,
                                    // Could add: deref_base tracking for method result chaining
                                },
                                SourceLocation { start, end },
                            );
                            deref_chain.push(expr.clone());
                        }
                        
                        _ => break,
                    }
                }
                
                Some(TokenKind::LeftBrace) => {
                    // Direct hash access: $hash{key}
                    // Similar to arrow case but with "{}" op
                    // ...
                    break;
                }
                
                Some(TokenKind::LeftBracket) => {
                    // Direct array access: $array[index]
                    // Similar to arrow case but with "[]" op
                    // ...
                    break;
                }
                
                _ => break,
            }
        }
        
        // Optionally: store the complete chain on the final node
        Ok(expr)
    }
}
```

#### Scope Analyzer Integration

```rust
impl ScopeAnalyzer {
    /// Analyze a variable node with deref base awareness
    fn analyze_variable_with_deref(
        &self,
        node: &Node,
        sigil: &str,
        name: &str,
        deref_base: Option<ExprId>,
        scope: &Rc<Scope>,
        ancestors: &[&Node],
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'_>,
    ) {
        // First: normal variable lookup
        let (found, initialized) = scope.use_variable_parts(sigil, name);
        
        if found {
            // Variable exists - check initialization
            if !initialized {
                issues.push(ScopeIssue {
                    kind: IssueKind::UninitializedVariable,
                    variable_name: format!("{}{}", sigil, name),
                    line: context.get_line(node.location.start),
                    range: (node.location.start, node.location.end),
                    description: format!("Variable '{}{}' used before initialization", sigil, name),
                });
            }
            return;
        }
        
        // Variable not found - check deref base for sigil-bridging
        if let Some(base_id) = deref_base {
            // Find the base variable in ancestor chain
            if let Some(base_var) = self.find_deref_base_variable(base_id, ancestors) {
                let (base_sigil, base_name) = base_var;
                
                // Try to find the base with different sigil
                // e.g., looking for $ref but have %ref declared
                let alt_sigil = match sigil {
                    "$" => "%",  // scalar -> hash
                    "@" => "%",  // array -> hash (for slices)
                    _ => sigil,
                };
                
                if alt_sigil != sigil {
                    let (base_found, base_init) = scope.use_variable_parts(alt_sigil, base_name);
                    if base_found {
                        // Sigil-bridging succeeded!
                        // The base hash/array exists, so this dereference is valid
                        return;
                    }
                }
            }
        }
        
        // Check ancestor-based fallback (current implementation)
        if !found {
            self.try_ancestor_sigil_bridge(node, sigil, name, scope, ancestors, issues, context);
        }
    }
    
    /// Find the variable node corresponding to a deref base ID
    fn find_deref_base_variable(
        &self,
        base_id: ExprId,
        ancestors: &[&Node],
    ) -> Option<(&str, &str)> {
        // Walk ancestors looking for a Binary node with matching deref_context.base_id
        for ancestor in ancestors.iter().rev() {
            if let NodeKind::Binary { left, deref_context, .. } = &ancestor.kind {
                if let Some(ctx) = deref_context {
                    if ctx.base_id == base_id {
                        // Found the base node - extract variable if it's simple
                        if let NodeKind::Variable { sigil, name, .. } = &left.kind {
                            return Some((sigil.as_str(), name.as_str()));
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Legacy fallback: check ancestors for hash/array context
    fn try_ancestor_sigil_bridge(
        &self,
        node: &Node,
        sigil: &str,
        name: &str,
        scope: &Rc<Scope>,
        ancestors: &[&Node],
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'_>,
    ) {
        // Existing implementation from scope_analyzer.rs
        if sigil == "$" || sigil == "@" {
            if let Some(parent) = ancestors.last() {
                if let NodeKind::Binary { op, left, .. } = &parent.kind {
                    if std::ptr::eq(left.as_ref(), node) {
                        if op == "{}" || op == "->{}" {
                            let (hash_used, _) = scope.use_variable_parts("%", name);
                            if hash_used {
                                return; // Sigil-bridged successfully
                            }
                        } else if op == "[]" || op == "->[]" {
                            let (arr_used, _) = scope.use_variable_parts("@", name);
                            if arr_used {
                                return; // Sigil-bridged successfully
                            }
                        }
                    }
                }
            }
        }
        
        // Not found - report error if strict
        let pragma_state = PragmaTracker::state_for_offset(context.pragma_map, node.location.start);
        if pragma_state.strict_subs {
            issues.push(ScopeIssue {
                kind: IssueKind::UndeclaredVariable,
                variable_name: format!("{}{}", sigil, name),
                line: context.get_line(node.location.start),
                range: (node.location.start, node.location.end),
                description: format!("Variable '{}{}' is used but not declared", sigil, name),
            });
        }
    }
}
```

### Issues Resolved

| Issue Category | Count | How Resolved |
|----------------|-------|--------------|
| False positive "undeclared" for $hash{key} when %hash exists | ~15 | `deref_base` enables reliable sigil-bridging |
| False positive for $ref->{key} when $ref is declared | ~8 | Direct base variable lookup via `base_id` |
| Complex chain resolution ($a->{b}->{c}) | ~6 | Chain depth tracking in `DerefContext` |
| Method call chain continuation | ~4 | MethodCall nodes preserve deref context |

**Total**: ~33 issues (29% of 113)

Note: Overlap with other architectures means total resolved may be less than sum.

### Implementation Phases

#### MVP (Week 1-2)

1. Add `deref_base: Option<ExprId>` to `NodeKind::Variable`
2. Add `deref_context: Option<DerefContext>` to `NodeKind::Binary`
3. Implement parser changes for `->{}` and `->[]` operators
4. Update `ScopeAnalyzer` to use `deref_base` when available

#### Full Implementation (Week 3-4)

1. Extend to direct access: `$hash{key}`, `$array[index]`
2. Add chain depth tracking for complex derefs
3. Handle method call continuation: `$obj->method()->{key}`
4. Add `ExprId` generation throughout parser

#### Rollout (Week 5)

1. Benchmark: compare annotation vs. ancestor-traversal performance
2. Deprecate pointer-comparison fallback (keep as safety net)
3. Document for downstream analyzers (type inference, etc.)

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| AST size increase | Medium | Low | `Option<ExprId>` is 4-8 bytes per Variable/Binary |
| Parser complexity | Medium | Medium | Phased rollout: arrow operators first |
| Backward compat | High | Medium | Make fields optional; default to None |
| ExprId uniqueness | Low | High | Use per-AST counter; document scoping rules |

---

## Cross-Architecture Synergies

### Combined Resolution Matrix

| Issue Type | Count | Arch 1 | Arch 2 | Arch 3 |
|-----------|-------|--------|--------|--------|
| Version/pragma errors | 62 | ✓ Primary | - | - |
| Builtin declaration FP | 32 | - | ✓ Primary | - |
| Dereference resolution | 33 | - | - | ✓ Primary |
| **Unique total** | **~90** | 62 | 20 | 8 |

*Note: Some issues resolved by multiple architectures; unique count estimated at 90.*

### Integration Points

1. **EffectiveSemantics + BUILTIN_INITIALIZERS**:
   - `BuiltinConfig::min_version` uses `PerlVersion` from Arch 1
   - Version-gated builtins only declare variables if version >= min

2. **EffectiveSemantics + deref_base**:
   - Postfix dereference features (postfix_deref) gated by `EffectiveSemantics::has_feature()`
   - Parser only annotates `deref_base` when postfix_deref is enabled

3. **BUILTIN_INITIALIZERS + deref_base**:
   - `recv`'s SCALAR param may be a dereference: `recv $sock, $buf->{data}, 1024, 0`
   - Combined: BUILTIN knows position 1 declares, deref_base tracks the chain

---

## Implementation Priority

### Recommended Order

1. **Architecture 1 (EffectiveSemantics)** - Week 1-2
   - Highest issue count (62)
   - Foundation for version-gating in Arch 2
   - Non-breaking API addition

2. **Architecture 2 (BUILTIN_INITIALIZERS)** - Week 3-4
   - Medium issue count (32)
   - Builds on Arch 1 for version-gated builtins
   - PHF compile-time cost

3. **Architecture 3 (deref_base)** - Week 5-6
   - Medium issue count (33)
   - AST change requires careful rollout
   - Depends on Arch 1 for postfix feature gating

### Alternative: Parallel Development

If team size permits, Architectures 1 and 2 can proceed in parallel:
- Arch 1 touches `perl-pragma`, `perl-lsp-diagnostics`
- Arch 2 touches `perl-builtins`, `perl-semantic-analyzer`
- Minimal file overlap

Architecture 3 should wait for Arch 1 completion due to `postfix_deref` feature dependency.

---

## Success Metrics

### Issue Resolution

| Target | Before | After | Measurement |
|--------|--------|-------|-------------|
| `expected_module_name` | 27 | 0 | CPAN corpus parse |
| Version FP (PL900) | ~15 | 0 | Diagnostic regression tests |
| Uninitialized FP | ~25 | <5 | Scope analyzer unit tests |
| Undeclared FP | ~20 | <3 | Scope analyzer unit tests |
| **Total 113** | 113 | ~15 | Combined metric |

### Performance

| Metric | Target | Baseline |
|--------|--------|----------|
| `PragmaTracker::state_for_offset` | <2μs | Current unknown |
| `BUILTIN_CONFIGS` lookup | <100ns | N/A (new) |
| `deref_base` query | <500ns | vs. ancestor traversal |
| AST memory increase | <5% | Current size |

---

## Appendix: Full Issue Inventory (from 113)

### Top Error Buckets Addressed

1. **`unexpected_comma_expr` (113 files)** - Partially addressed by all 3 architectures
2. **`expected_module_name` (27 files)** - Addressed by Architecture 1
3. **Version compatibility false positives (~15)** - Addressed by Architecture 1
4. **Builtin initialization FPs (~32)** - Addressed by Architecture 2
5. **Dereference resolution FPs (~33)** - Addressed by Architecture 3

### Out of Scope (Remaining ~23 Issues)

- `unclosed_paren` (66 files) - Parser recovery, not semantic
- `unclosed_brace` (46 files) - Parser recovery
- `unexpected_token_in_expr` (102 files) - Broad category requiring separate audit
- Regex/quote operator edge cases - Tokenizer scope

---

## Conclusion

These three architectures provide a unified foundation for resolving the majority of the 113 filed issues:

1. **EffectiveSemantics** consolidates version/pragma/feature tracking, eliminating duplication and enabling accurate feature queries.

2. **BUILTIN_INITIALIZERS** provides systematic handling of variable-declaring builtins, reducing false positives in scope analysis.

3. **deref_base** enables reliable sigil-bridging for complex dereference chains, improving variable resolution accuracy.

Together, they establish patterns for future semantic analysis work: centralized registries, explicit annotations, and queryable state matrices.
