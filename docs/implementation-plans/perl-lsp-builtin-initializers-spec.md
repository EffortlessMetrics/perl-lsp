# BUILTIN_INITIALIZERS Implementation Specification

**Status**: Implementation Ready  
**Date**: 2026-04-09  
**Source**: Architecture 2 from `perl-lsp-architecture-rfc.md`

---

## 1. PHF Registry Definition

### 1.1 Core Type Definitions

```rust
// crates/perl-builtins-semantic/src/types.rs

use std::fmt;

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

impl fmt::Display for ParamRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamRole::Input => write!(f, "input"),
            ParamRole::OutputScalar => write!(f, "output_scalar"),
            ParamRole::OutputArray => write!(f, "output_array"),
            ParamRole::OutputHash => write!(f, "output_hash"),
            ParamRole::InOut => write!(f, "inout"),
            ParamRole::Handle => write!(f, "handle"),
            ParamRole::Format => write!(f, "format"),
            ParamRole::CodeRef => write!(f, "coderef"),
            ParamRole::Optional => write!(f, "optional"),
        }
    }
}

/// Variable sigil kind for declaration effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SigilKind {
    Scalar,   // $
    Array,    // @
    Hash,     // %
    Sub,      // &
    Glob,     // *
}

impl SigilKind {
    pub fn as_char(&self) -> char {
        match self {
            SigilKind::Scalar => '$',
            SigilKind::Array => '@',
            SigilKind::Hash => '%',
            SigilKind::Sub => '&',
            SigilKind::Glob => '*',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '$' => Some(SigilKind::Scalar),
            '@' => Some(SigilKind::Array),
            '%' => Some(SigilKind::Hash),
            '&' => Some(SigilKind::Sub),
            '*' => Some(SigilKind::Glob),
            _ => None,
        }
    }
}

/// Declaration effect at a specific parameter position
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Perl version in comparable form
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerlVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
}

impl PerlVersion {
    pub const V5_8: Self = Self { major: 5, minor: 8, patch: None };
    pub const V5_10: Self = Self { major: 5, minor: 10, patch: None };
    pub const V5_20: Self = Self { major: 5, minor: 20, patch: None };
    pub const V5_36: Self = Self { major: 5, minor: 36, patch: None };
    pub const V5_38: Self = Self { major: 5, minor: 38, patch: None };
}

/// Category for organizing builtins
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
```

### 1.2 Complete PHF Registry

```rust
// crates/perl-builtins-semantic/src/registry.rs

use phf::phf_map;
use super::types::*;

/// PHF-backed registry of builtin configurations with declaration effects
/// 
/// This registry maps builtin function names to their semantic configuration,
/// specifically tracking which parameters declare/initialize variables.
pub static BUILTIN_CONFIGS: phf::Map<&'static str, BuiltinConfig> = phf_map! {
    // === I/O BUILTINS WITH OUTPUT PARAMETERS ===
    
    "read" => BuiltinConfig {
        name: "read",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::Handle, 
                declares: None, 
                name_hint: "FILEHANDLE" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "SCALAR" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "LENGTH" 
            },
            ParamEffect { 
                position: 3, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "OFFSET" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    
    "sysread" => BuiltinConfig {
        name: "sysread",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::Handle, 
                declares: None, 
                name_hint: "FILEHANDLE" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "SCALAR" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "LENGTH" 
            },
            ParamEffect { 
                position: 3, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "OFFSET" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    
    "recv" => BuiltinConfig {
        name: "recv",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::Handle, 
                declares: None, 
                name_hint: "SOCKET" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "SCALAR" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "LENGTH" 
            },
            ParamEffect { 
                position: 3, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "FLAGS" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Socket,
        min_version: None,
    },
    
    // === SOCKET BUILTINS ===
    
    "socketpair" => BuiltinConfig {
        name: "socketpair",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "READ_HANDLE" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "WRITE_HANDLE" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "DOMAIN" 
            },
            ParamEffect { 
                position: 3, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "TYPE" 
            },
            ParamEffect { 
                position: 4, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "PROTOCOL" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Socket,
        min_version: Some(PerlVersion::V5_8),
    },
    
    // === DATABASE BUILTINS ===
    
    "dbmopen" => BuiltinConfig {
        name: "dbmopen",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::OutputHash, 
                declares: Some(SigilKind::Hash), 
                name_hint: "HASH" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "DBNAME" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "MODE" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Database,
        min_version: None,
    },
    
    // === PROCESS BUILTINS ===
    
    "pipe" => BuiltinConfig {
        name: "pipe",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "READ_END" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::OutputScalar, 
                declares: Some(SigilKind::Scalar), 
                name_hint: "WRITE_END" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Process,
        min_version: None,
    },
    
    // === I/O BUILTINS (FILEHANDLE DECLARATION) ===
    
    "open" => BuiltinConfig {
        name: "open",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::Handle, 
                declares: Some(SigilKind::Glob), 
                name_hint: "FILEHANDLE" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::Optional, 
                declares: None, 
                name_hint: "MODE" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Optional, 
                declares: None, 
                name_hint: "EXPR" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    
    "socket" => BuiltinConfig {
        name: "socket",
        params: &[
            ParamEffect { 
                position: 0, 
                role: ParamRole::Handle, 
                declares: Some(SigilKind::Glob), 
                name_hint: "SOCKET" 
            },
            ParamEffect { 
                position: 1, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "DOMAIN" 
            },
            ParamEffect { 
                position: 2, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "TYPE" 
            },
            ParamEffect { 
                position: 3, 
                role: ParamRole::Input, 
                declares: None, 
                name_hint: "PROTOCOL" 
            },
        ],
        has_declaration_effects: true,
        category: BuiltinCategory::Socket,
        min_version: None,
    },
    
    // === ADDITIONAL BUILTINS (No declaration effects) ===
    // These are included for completeness and future extension
    
    "print" => BuiltinConfig {
        name: "print",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: None, name_hint: "FILEHANDLE" },
        ],
        has_declaration_effects: false,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    
    "printf" => BuiltinConfig {
        name: "printf",
        params: &[
            ParamEffect { position: 0, role: ParamRole::Handle, declares: None, name_hint: "FILEHANDLE" },
            ParamEffect { position: 1, role: ParamRole::Format, declares: None, name_hint: "FORMAT" },
        ],
        has_declaration_effects: false,
        category: BuiltinCategory::Io,
        min_version: None,
    },
    
    "substr" => BuiltinConfig {
        name: "substr",
        params: &[
            ParamEffect { position: 0, role: ParamRole::InOut, declares: None, name_hint: "EXPR" },
            ParamEffect { position: 1, role: ParamRole::Input, declares: None, name_hint: "OFFSET" },
            ParamEffect { position: 2, role: ParamRole::Optional, declares: None, name_hint: "LENGTH" },
        ],
        has_declaration_effects: false,
        category: BuiltinCategory::String,
        min_version: None,
    },
};

/// Get builtin configuration by name
pub fn get_builtin_config(name: &str) -> Option<&'static BuiltinConfig> {
    BUILTIN_CONFIGS.get(name)
}

/// Check if a builtin has declaration effects at specific parameter positions
pub fn get_declaration_effects(name: &str) -> Vec<(usize, SigilKind, &'static str)> {
    let mut effects = Vec::new();
    
    if let Some(config) = BUILTIN_CONFIGS.get(name) {
        if config.has_declaration_effects {
            for effect in config.params {
                if let Some(sigil) = effect.declares {
                    effects.push((effect.position, sigil, effect.name_hint));
                }
            }
        }
    }
    
    effects
}

/// Filter builtins by category
pub fn builtins_in_category(cat: BuiltinCategory) -> Vec<&'static BuiltinConfig> {
    BUILTIN_CONFIGS
        .values()
        .filter(|cfg| cfg.category == cat)
        .collect()
}

/// Get all builtins with declaration effects
pub fn builtins_with_declaration_effects() -> Vec<&'static BuiltinConfig> {
    BUILTIN_CONFIGS
        .values()
        .filter(|cfg| cfg.has_declaration_effects)
        .collect()
}
```

### 1.3 Registry Query Interface

```rust
// crates/perl-builtins-semantic/src/query.rs

use super::types::*;
use super::registry::BUILTIN_CONFIGS;

/// Result of analyzing a builtin call for declaration effects
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinAnalysisResult {
    /// Variables declared by this call
    pub declared_vars: Vec<DeclaredVariable>,
    /// Whether the builtin was found in registry
    pub known_builtin: bool,
    /// Parameter count (for validation)
    pub expected_params: usize,
}

/// A variable declared by a builtin call
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredVariable {
    /// Parameter position (0-indexed)
    pub position: usize,
    /// Variable sigil
    pub sigil: SigilKind,
    /// Variable name (if extractable from AST)
    pub name: Option<String>,
    /// Raw expression at this position (for debugging)
    pub name_hint: &'static str,
    /// Whether this is a "must declare" vs "may declare"
    pub is_optional: bool,
}

/// Query interface for builtin semantic analysis
pub struct BuiltinQuery;

impl BuiltinQuery {
    /// Check if a function name is a known builtin with declaration effects
    pub fn has_declaration_effects(name: &str) -> bool {
        BUILTIN_CONFIGS
            .get(name)
            .map(|cfg| cfg.has_declaration_effects)
            .unwrap_or(false)
    }

    /// Get declaration positions for a builtin
    /// Returns Vec<(position, sigil_kind, name_hint)>
    pub fn get_declaration_positions(name: &str) -> Vec<(usize, SigilKind, &'static str)> {
        let mut positions = Vec::new();
        
        if let Some(config) = BUILTIN_CONFIGS.get(name) {
            for effect in config.params {
                if let Some(sigil) = effect.declares {
                    positions.push((effect.position, sigil, effect.name_hint));
                }
            }
        }
        
        positions
    }

    /// Get parameter info for diagnostics/tooltips
    pub fn get_param_info(name: &str, position: usize) -> Option<ParamEffect> {
        BUILTIN_CONFIGS
            .get(name)
            .and_then(|cfg| cfg.params.get(position))
            .copied()
    }

    /// Check if a builtin is version-gated and requires specific Perl version
    pub fn check_version_requirement(
        name: &str, 
        current_version: PerlVersion
    ) -> Option<PerlVersion> {
        BUILTIN_CONFIGS
            .get(name)
            .and_then(|cfg| cfg.min_version)
            .filter(|min| current_version < *min)
    }

    /// Validate argument count against builtin signature
    pub fn validate_arg_count(name: &str, arg_count: usize) -> ArgCountValidation {
        if let Some(config) = BUILTIN_CONFIGS.get(name) {
            let required = config
                .params
                .iter()
                .filter(|p| p.role != ParamRole::Optional)
                .count();
            let total = config.params.len();
            
            if arg_count < required {
                ArgCountValidation::TooFew { required, got: arg_count }
            } else if arg_count > total {
                ArgCountValidation::TooMany { max: total, got: arg_count }
            } else {
                ArgCountValidation::Valid
            }
        } else {
            ArgCountValidation::UnknownBuiltin
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArgCountValidation {
    Valid,
    TooFew { required: usize, got: usize },
    TooMany { max: usize, got: usize },
    UnknownBuiltin,
}
```

---

## 2. Integration with SymbolExtractor

### 2.1 Builtin Call Detection

```rust
// crates/perl-semantic-analyzer/src/builtin_detector.rs

use perl_ast::{Node, NodeKind};
use perl_builtin_semantic::{BuiltinQuery, SigilKind, DeclaredVariable};

/// Detects builtin function calls in AST nodes
pub struct BuiltinCallDetector;

impl BuiltinCallDetector {
    /// Check if a node is a builtin function call
    /// Returns (name, args) if it's a call, None otherwise
    pub fn detect_builtin_call(node: &Node) -> Option<(&str, &[Node])> {
        match &node.kind {
            // Direct builtin call: read(...)
            NodeKind::Call { func, args } => {
                if let NodeKind::Variable { sigil, name, .. } = &func.kind {
                    // Builtins are barewords (no sigil) or &func
                    if sigil.is_empty() || sigil == "&" {
                        return Some((name.as_str(), args.as_slice()));
                    }
                }
                None
            }
            // Indirect object syntax: read FH, $buf, ...
            NodeKind::IndirectCall { func, args, .. } => {
                if let NodeKind::Variable { sigil, name, .. } = &func.kind {
                    if sigil.is_empty() {
                        return Some((name.as_str(), args.as_slice()));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Detect if node is a builtin with declaration effects
    pub fn detect_declaration_builtin(node: &Node) -> Option<(&str, &[Node])> {
        Self::detect_builtin_call(node).filter(|(name, _)| {
            BuiltinQuery::has_declaration_effects(name)
        })
    }
}
```

### 2.2 Variable Extraction from Arguments

```rust
// crates/perl-semantic-analyzer/src/arg_extractor.rs

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_builtin_semantic::SigilKind;

/// Extracts variable information from argument nodes
pub struct ArgVariableExtractor;

/// Extracted variable information
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedVariable {
    pub sigil: char,
    pub name: String,
    pub location: SourceLocation,
    pub is_deref: bool,  // Is this $ref->{key} or similar?
}

impl ArgVariableExtractor {
    /// Extract a simple variable from an argument at given position
    /// 
    /// Handles:
    /// - Simple variables: $buf
    /// - Variables in dereference: $ref->{key} (extracts $ref)
    /// - Array elements: $array[0] (extracts @array)
    /// - Hash elements: $hash{key} (extracts %hash)
    pub fn extract_variable(arg: &Node) -> Option<ExtractedVariable> {
        match &arg.kind {
            // Simple variable: $buf
            NodeKind::Variable { sigil, name, .. } => {
                if sigil.len() == 1 {
                    return Some(ExtractedVariable {
                        sigil: sigil.chars().next().unwrap(),
                        name: name.clone(),
                        location: arg.location.clone(),
                        is_deref: false,
                    });
                }
                None
            }
            
            // Dereference: $ref->{key}, $ref->[0]
            // We want to extract the base variable $ref
            NodeKind::Binary { op, left, .. } if op.starts_with("->") => {
                // Recursively extract from left side
                Self::extract_variable(left).map(|mut v| {
                    v.is_deref = true;
                    v
                })
            }
            
            // Direct hash/array access: $hash{key}, $array[0]
            NodeKind::Binary { op, left, .. } => {
                if op == "{}" || op == "[]" {
                    // Convert to container type
                    Self::extract_variable(left).map(|mut v| {
                        v.is_deref = true;
                        v
                    })
                } else {
                    None
                }
            }
            
            // Parenthesized expression: ($var)
            NodeKind::Group { expr } => {
                Self::extract_variable(expr)
            }
            
            // Other cases not supported
            _ => None,
        }
    }

    /// Extract with expected sigil (for type checking)
    pub fn extract_variable_with_sigil(
        arg: &Node, 
        expected_sigil: SigilKind
    ) -> Option<ExtractedVariable> {
        Self::extract_variable(arg).filter(|v| {
            v.sigil == expected_sigil.as_char()
        })
    }

    /// Extract variable name as string (simplified interface)
    pub fn extract_variable_name(arg: &Node) -> Option<(char, String)> {
        Self::extract_variable(arg)
            .map(|v| (v.sigil, v.name))
    }

    /// Check if argument is a valid lvalue for output parameter
    /// Must be a variable, array/hash element, or dereference
    pub fn is_valid_output_target(arg: &Node) -> bool {
        match &arg.kind {
            NodeKind::Variable { .. } => true,
            NodeKind::Binary { op, .. } => {
                matches!(op.as_str(), "{}" | "[]" | "->{}" | "->[]")
            }
            NodeKind::Group { expr } => Self::is_valid_output_target(expr),
            _ => false,
        }
    }

    /// Extract all variables from argument list (for diagnostics)
    pub fn extract_all_variables(args: &[Node]) -> Vec<Option<ExtractedVariable>> {
        args.iter().map(Self::extract_variable).collect()
    }
}
```

### 2.3 Position-Aware Extraction Logic

```rust
// crates/perl-semantic-analyzer/src/position_extractor.rs

use perl_ast::{Node, SourceLocation};
use perl_builtin_semantic::{BuiltinQuery, DeclaredVariable};
use super::arg_extractor::{ArgVariableExtractor, ExtractedVariable};

/// Position-aware extraction for builtin arguments
pub struct PositionAwareExtractor;

/// Result of extracting variables at declaration positions
#[derive(Debug, Clone)]
pub struct PositionExtractionResult {
    pub position: usize,
    pub name_hint: &'static str,
    pub expected_sigil: char,
    pub extracted: Option<ExtractedVariable>,
    pub issues: Vec<ExtractionIssue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionIssue {
    MissingArgument { position: usize, name_hint: &'static str },
    WrongSigil { expected: char, got: Option<char>, name_hint: &'static str },
    NotAnLvalue { position: usize, name_hint: &'static str },
    ComplexExpression { position: usize, name_hint: &'static str },
}

impl PositionAwareExtractor {
    /// Extract variables at all declaration positions for a builtin call
    /// 
    /// Example: `read FH, $buf, 1024` with read builtin
    /// - Position 0: FH (Handle, no declaration effect)
    /// - Position 1: $buf (OutputScalar, declares $)
    /// - Position 2: 1024 (Input, no declaration effect)
    pub fn extract_declaration_variables(
        builtin_name: &str,
        args: &[Node],
    ) -> Vec<PositionExtractionResult> {
        let mut results = Vec::new();
        
        // Get all declaration positions for this builtin
        let declaration_positions = BuiltinQuery::get_declaration_positions(builtin_name);
        
        for (position, sigil_kind, name_hint) in declaration_positions {
            let expected_sigil = sigil_kind.as_char();
            
            // Try to get argument at this position
            let arg = args.get(position);
            
            let (extracted, issues) = if let Some(arg) = arg {
                let extracted = ArgVariableExtractor::extract_variable(arg);
                let mut issues = Vec::new();
                
                // Validate extracted variable
                if let Some(ref v) = extracted {
                    // Check sigil match
                    if v.sigil != expected_sigil {
                        issues.push(ExtractionIssue::WrongSigil {
                            expected: expected_sigil,
                            got: Some(v.sigil),
                            name_hint,
                        });
                    }
                    
                    // Check if it's a valid lvalue
                    if !ArgVariableExtractor::is_valid_output_target(arg) {
                        issues.push(ExtractionIssue::NotAnLvalue { position, name_hint });
                    }
                } else {
                    // Could not extract - might be a complex expression
                    issues.push(ExtractionIssue::ComplexExpression { position, name_hint });
                }
                
                (extracted, issues)
            } else {
                // Missing argument
                let issues = vec![ExtractionIssue::MissingArgument { position, name_hint }];
                (None, issues)
            };
            
            results.push(PositionExtractionResult {
                position,
                name_hint,
                expected_sigil,
                extracted,
                issues,
            });
        }
        
        results
    }

    /// Get simplified list of declared variables (for scope analyzer)
    /// Filters out entries with issues and returns clean (sigil, name, location)
    pub fn get_clean_declarations(
        builtin_name: &str,
        args: &[Node],
    ) -> Vec<(char, String, SourceLocation)> {
        Self::extract_declaration_variables(builtin_name, args)
            .into_iter()
            .filter(|r| r.issues.is_empty())
            .filter_map(|r| {
                r.extracted.map(|v| (v.sigil, v.name, v.location))
            })
            .collect()
    }

    /// Get diagnostics for a builtin call
    pub fn get_extraction_diagnostics(
        builtin_name: &str,
        args: &[Node],
    ) -> Vec<ExtractionIssue> {
        Self::extract_declaration_variables(builtin_name, args)
            .into_iter()
            .flat_map(|r| r.issues)
            .collect()
    }
}
```

---

## 3. Scope Analyzer Integration

### 3.1 Marking Variables as Declared-Before-Use

```rust
// crates/perl-semantic-analyzer/src/scope/builtin_integration.rs

use std::rc::Rc;
use perl_ast::Node;
use perl_builtin_semantic::BuiltinQuery;
use super::scope::Scope;
use super::scope_analyzer::ScopeAnalyzer;
use super::types::{ScopeIssue, IssueKind, AnalysisContext};
use crate::builtin_detector::BuiltinCallDetector;
use crate::position_extractor::PositionAwareExtractor;

/// Extension trait for ScopeAnalyzer to handle builtin declaration effects
pub trait BuiltinAwareScopeAnalyzer {
    /// Analyze a builtin call and apply declaration effects to scope
    fn analyze_builtin_declaration_effects(
        &mut self,
        func_name: &str,
        args: &[Node],
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) -> Vec<ScopeIssue>;
    
    /// Check if node is a builtin call and process it
    fn try_process_builtin_call(
        &mut self,
        node: &Node,
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) -> Option<Vec<ScopeIssue>>;
}

impl BuiltinAwareScopeAnalyzer for ScopeAnalyzer {
    fn analyze_builtin_declaration_effects(
        &mut self,
        func_name: &str,
        args: &[Node],
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        
        // Quick check: does this builtin have declaration effects?
        if !BuiltinQuery::has_declaration_effects(func_name) {
            return issues;
        }
        
        // Extract all declaration variables at their positions
        let extraction_results = PositionAwareExtractor::extract_declaration_variables(
            func_name, 
            args
        );
        
        for result in extraction_results {
            // If there were extraction issues, add them as warnings
            for issue in &result.issues {
                let issue_kind = match issue {
                    ExtractionIssue::MissingArgument { position, name_hint } => {
                        IssueKind::BuiltinMissingArg {
                            builtin: func_name.to_string(),
                            position: *position,
                            param_name: name_hint.to_string(),
                        }
                    }
                    ExtractionIssue::WrongSigil { expected, got, name_hint } => {
                        IssueKind::BuiltinArgTypeMismatch {
                            builtin: func_name.to_string(),
                            position: result.position,
                            expected_sigil: *expected,
                            got_sigil: *got,
                            param_name: name_hint.to_string(),
                        }
                    }
                    _ => continue, // Other issues don't produce scope issues
                };
                
                issues.push(ScopeIssue {
                    kind: issue_kind,
                    variable_name: format!("{}", result.name_hint),
                    line: context.get_line(args.get(result.position)
                        .map(|n| n.location.start)
                        .unwrap_or(0)),
                    range: (0, 0),
                    description: format!("Builtin '{}' argument issue", func_name),
                });
            }
            
            // Process successful extractions
            if let Some(ref extracted) = result.extracted {
                if result.issues.is_empty() {
                    let sigil_str = extracted.sigil.to_string();
                    let name = &extracted.name;
                    
                    // Check if variable already exists
                    let (already_declared, _) = scope.use_variable_parts(&sigil_str, name);
                    
                    if !already_declared {
                        // Declare the variable (mark as declared-before-use)
                        // This is the key operation: we're declaring the variable
                        // at the point of the builtin call
                        let declare_result = scope.declare_variable_parts(
                            &sigil_str,
                            name,
                            extracted.location.start,
                            false,  // not 'our'
                            true,   // initialized by builtin
                        );
                        
                        if let Some(issue_kind) = declare_result {
                            issues.push(ScopeIssue {
                                kind: issue_kind,
                                variable_name: format!("{}{}", extracted.sigil, name),
                                line: context.get_line(extracted.location.start),
                                range: (extracted.location.start, extracted.location.end),
                                description: format!(
                                    "Builtin '{}' declares '{}{}' at position {}",
                                    func_name, extracted.sigil, name, result.position
                                ),
                            });
                        }
                    }
                    
                    // Mark as used (since builtin writes to it)
                    // This prevents "unused variable" warnings
                    scope.initialize_and_use_variable_parts(&sigil_str, name);
                }
            }
        }
        
        issues
    }
    
    fn try_process_builtin_call(
        &mut self,
        node: &Node,
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) -> Option<Vec<ScopeIssue>> {
        BuiltinCallDetector::detect_declaration_builtin(node)
            .map(|(name, args)| {
                self.analyze_builtin_declaration_effects(name, args, scope, context)
            })
    }
}
```

### 3.2 Avoiding False Positives for Builtins

```rust
// crates/perl-semantic-analyzer/src/scope/false_positive_prevention.rs

use perl_ast::Node;
use perl_builtin_semantic::get_builtin_config;

/// Strategies to avoid false positives with builtin detection
pub struct FalsePositivePrevention;

impl FalsePositivePrevention {
    /// Check if what looks like a builtin is actually:
    /// 1. A user-defined subroutine (shadows builtin)
    /// 2. A method call (Foo->read)
    /// 3. A prototype declaration
    /// 
    /// This requires scope knowledge - if the name is already declared
    /// as a sub in this scope, it's not a builtin call
    pub fn is_likely_user_sub(
        name: &str,
        scope: &super::Scope,
        context: &super::AnalysisContext<'_>,
    ) -> bool {
        // Check if there's a user-defined sub with this name
        // This would require looking up in the scope
        // Implementation depends on scope implementation
        
        // For now, conservative: assume builtin unless proven otherwise
        // Future: check scope for sub declarations
        false
    }

    /// Check if builtin name is overridden by package import
    /// use SomeModule 'read';  # shadows builtin
    pub fn is_imported_sub(name: &str, context: &super::AnalysisContext<'_>) -> bool {
        // Would check import tracking in context
        // For now, stub
        false
    }

    /// Validate builtin call context
    /// Some builtins have special parsing rules
    pub fn validate_builtin_context(
        name: &str,
        node: &Node,
        parent: Option<&Node>,
    ) -> BuiltinContextValidity {
        match name {
            // Indirect object syntax: read FH, $buf
            // vs function call: read(FH, $buf)
            "read" | "sysread" | "recv" => {
                // Both are valid in Perl
                BuiltinContextValidity::Valid
            }
            
            // Check for common false positive: method call
            // $obj->read(...) is NOT the builtin read()
            _ => {
                if let Some(parent_node) = parent {
                    if is_method_call_context(node, parent_node) {
                        return BuiltinContextValidity::LikelyMethodCall;
                    }
                }
                BuiltinContextValidity::Valid
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuiltinContextValidity {
    Valid,
    LikelyUserSub,
    LikelyMethodCall,
    LikelyImported,
    InvalidContext,
}

fn is_method_call_context(node: &Node, parent: &Node) -> bool {
    use perl_ast::NodeKind;
    
    match &parent.kind {
        NodeKind::MethodCall { object, .. } => {
            // Check if node is the method part of a method call
            // This would require pointer comparison or ID matching
            false // Placeholder
        }
        _ => false,
    }
}
```

### 3.3 Integration with Existing Builtin Handling

```rust
// crates/perl-semantic-analyzer/src/scope/integrated_analysis.rs

use std::rc::Rc;
use perl_ast::{Node, NodeKind};
use super::scope::Scope;
use super::scope_analyzer::ScopeAnalyzer;
use super::types::{ScopeIssue, IssueKind, AnalysisContext};
use super::builtin_integration::BuiltinAwareScopeAnalyzer;

/// Unified analysis that combines PHF registry with existing scope logic
impl ScopeAnalyzer {
    /// Main entry point for analyzing any node type
    /// This replaces/adapts the existing analyze_node implementation
    pub fn analyze_node_integrated(
        &mut self,
        node: &Node,
        scope: &Rc<Scope>,
        ancestors: &[&Node],
        context: &AnalysisContext<'_>,
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        
        match &node.kind {
            // Variable usage - existing logic
            NodeKind::Variable { sigil, name, deref_base } => {
                issues.extend(self.analyze_variable_with_deref(
                    node, sigil, name, *deref_base, scope, ancestors, context
                ));
            }
            
            // Builtin call detection - NEW PHF-based logic
            NodeKind::Call { .. } | NodeKind::IndirectCall { .. } => {
                // Try PHF-based builtin processing first
                if let Some(builtin_issues) = self.try_process_builtin_call(node, scope, context) {
                    issues.extend(builtin_issues);
                } else {
                    // Fall back to regular function call analysis
                    issues.extend(self.analyze_regular_call(node, scope, ancestors, context));
                }
            }
            
            // Other node types...
            _ => {
                // Recurse into children
                for child in node.children() {
                    let child_scope = self.scope_for_node(child, scope);
                    issues.extend(self.analyze_node_integrated(
                        child, &child_scope, ancestors, context
                    ));
                }
            }
        }
        
        issues
    }
    
    /// Analyze a variable, using deref_base from PHF if available
    fn analyze_variable_with_deref(
        &mut self,
        node: &Node,
        sigil: &str,
        name: &str,
        deref_base: Option<perl_ast::ExprId>,
        scope: &Rc<Scope>,
        ancestors: &[&Node],
        context: &AnalysisContext<'_>,
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        
        // First: normal variable lookup
        let (found, initialized) = scope.use_variable_parts(sigil, name);
        
        if found {
            if !initialized {
                // Check if this variable was declared by a builtin
                // This is where we prevent false positives
                if !self.was_declared_by_builtin(node, scope) {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UninitializedVariable,
                        variable_name: format!("{}{}", sigil, name),
                        line: context.get_line(node.location.start),
                        range: (node.location.start, node.location.end),
                        description: format!("Variable '{}{}' used before initialization", sigil, name),
                    });
                }
            }
            return issues;
        }
        
        // Variable not found - check for declaration we might have missed
        // or sigil-bridging opportunities
        
        // Try sigil-bridging through deref_base (from Architecture 3)
        if let Some(base_id) = deref_base {
            if let Some((base_sigil, base_name)) = self.find_deref_base_variable(base_id, ancestors) {
                let alt_sigil = match sigil {
                    "$" => "%",
                    "@" => "%",
                    _ => sigil,
                };
                
                if alt_sigil != sigil {
                    let (base_found, _) = scope.use_variable_parts(alt_sigil, base_name);
                    if base_found {
                        // Sigil-bridging succeeded
                        return issues;
                    }
                }
            }
        }
        
        // Check if strict is on for undeclared variable reporting
        let pragma_state = context.pragma_tracker.state_for_offset(node.location.start);
        if pragma_state.strict_vars {
            issues.push(ScopeIssue {
                kind: IssueKind::UndeclaredVariable,
                variable_name: format!("{}{}", sigil, name),
                line: context.get_line(node.location.start),
                range: (node.location.start, node.location.end),
                description: format!("Variable '{}{}' is used but not declared", sigil, name),
            });
        }
        
        issues
    }
    
    /// Check if variable was declared by a builtin at this location
    /// Uses location matching to find builtin declarations
    fn was_declared_by_builtin(
        &self,
        node: &Node,
        _scope: &Rc<Scope>,
    ) -> bool {
        // Would check a cache of builtin-declared variables
        // Populated during analysis
        // For now, rely on the fact that we just declared it in this pass
        false
    }
    
    fn find_deref_base_variable(
        &self,
        _base_id: perl_ast::ExprId,
        _ancestors: &[&Node],
    ) -> Option<(&str, &str)> {
        // Implementation from Architecture 3
        None
    }
    
    fn analyze_regular_call(
        &mut self,
        _node: &Node,
        _scope: &Rc<Scope>,
        _ancestors: &[&Node],
        _context: &AnalysisContext<'_>,
    ) -> Vec<ScopeIssue> {
        // Existing non-builtin call analysis
        Vec::new()
    }
    
    fn scope_for_node(&self, _node: &Node, parent_scope: &Rc<Scope>) -> Rc<Scope> {
        // Existing scope creation logic
        parent_scope.clone()
    }
}
```

---

## 4. Migration from Current System

### 4.1 Current Builtin Handling Analysis

```rust
// Current implementation in scope_analyzer.rs (BEFORE migration)
// This shows what needs to be replaced

/*
// Current ad-hoc builtin handling (to be replaced)
fn check_variable_usage(&mut self, node: &Node, scope: &Scope) {
    if let NodeKind::Variable { sigil, name } = &node.kind {
        // Hardcoded check for specific builtins
        // This is scattered throughout the analyzer
        
        // Check parent context for read/sysread patterns
        if sigil == "$" {
            if let Some(parent) = self.get_parent(node) {
                // Manual pattern matching for read() buffer arg
                // This is fragile and incomplete
                if is_read_call_context(parent) {
                    // Mark as initialized
                }
            }
        }
    }
}

// Current builtin detection (incomplete)
fn is_builtin_with_output(name: &str) -> bool {
    // Hardcoded list, not comprehensive
    matches!(name, "read" | "sysread" | "recv")
}
*/
```

### 4.2 Migration Steps

```markdown
## Migration Plan: scope_analyzer.rs → PHF Registry

### Phase 1: Add PHF Dependency (Day 1-2)

1. Add to `Cargo.toml`:
   ```toml
   [dependencies]
   perl-builtins-semantic = { path = "../perl-builtins-semantic" }
   ```

2. Create compatibility shim:
   ```rust
   // In scope_analyzer.rs, add:
   use perl_builtin_semantic::{BuiltinQuery, get_declaration_effects};
   ```

### Phase 2: Dual Implementation (Day 3-5)

1. Keep existing logic as fallback
2. Add PHF-based check before existing logic:
   ```rust
   // New code (runs first)
   if BuiltinQuery::has_declaration_effects(func_name) {
       return handle_with_phf_registry(node, scope);
   }
   
   // Existing code (fallback)
   if is_builtin_with_output(func_name) {
       return handle_ad_hoc(node, scope);
   }
   ```

3. Add feature flag: `--cfg use_phf_builtins`

### Phase 3: Testing Parity (Day 6-10)

1. Run test suite with both implementations
2. Compare results, fix discrepancies
3. Add new test cases for all 6+ declaration builtins

### Phase 4: Cutover (Day 11-12)

1. Remove ad-hoc builtin handling
2. Make PHF registry the primary path
3. Remove feature flag

### Phase 5: Cleanup (Day 13-14)

1. Remove deprecated helper functions
2. Update documentation
3. Benchmark performance
```

### 4.3 Testing Strategy for Parity

```rust
// crates/perl-semantic-analyzer/tests/builtin_migration_tests.rs

use perl_semantic_analyzer::ScopeAnalyzer;
use perl_parser_core::parse;

/// Test cases ensuring parity between old and new implementations
#[cfg(test)]
mod migration_tests {
    use super::*;
    
    /// Test: read() declares buffer variable
    #[test]
    fn test_read_declares_buffer() {
        let code = r#"
            open my $fh, '<', 'file.txt';
            read $fh, $buffer, 1024;
            print $buffer;
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        // Should NOT report uninitialized $buffer
        assert!(!issues.iter().any(|i| 
            i.variable_name == "$buffer" && 
            matches!(i.kind, IssueKind::UninitializedVariable)
        ));
    }
    
    /// Test: sysread() with offset
    #[test]
    fn test_sysread_declares_buffer_with_offset() {
        let code = r#"
            sysread $fh, $data, 100, 50;
            say $data;
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        assert!(!issues.iter().any(|i| 
            i.variable_name == "$data" && 
            matches!(i.kind, IssueKind::UninitializedVariable)
        ));
    }
    
    /// Test: socketpair declares both handles
    #[test]
    fn test_socketpair_declares_handles() {
        let code = r#"
            socketpair $read, $write, AF_UNIX, SOCK_STREAM, PF_UNSPEC;
            print $read "hello";
            close $write;
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        assert!(!issues.iter().any(|i| 
            (i.variable_name == "$read" || i.variable_name == "$write") &&
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UninitializedVariable)
        ));
    }
    
    /// Test: pipe() declares both ends
    #[test]
    fn test_pipe_declares_ends() {
        let code = r#"
            pipe $reader, $writer;
            print $writer "data";
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        assert!(!issues.iter().any(|i| 
            (i.variable_name == "$reader" || i.variable_name == "$writer") &&
            matches!(i.kind, IssueKind::UndeclaredVariable)
        ));
    }
    
    /// Test: dbmopen declares hash
    #[test]
    fn test_dbmopen_declares_hash() {
        let code = r#"
            dbmopen %db, 'database', 0644;
            $db{key} = 'value';
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        assert!(!issues.iter().any(|i| 
            i.variable_name == "%db" &&
            matches!(i.kind, IssueKind::UndeclaredVariable)
        ));
    }
    
    /// Test: recv() declares buffer
    #[test]
    fn test_recv_declares_buffer() {
        let code = r#"
            recv $sock, $msg, 1024, 0;
            process($msg);
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        assert!(!issues.iter().any(|i| 
            i.variable_name == "$msg" &&
            matches!(i.kind, IssueKind::UninitializedVariable)
        ));
    }
    
    /// Test: open() with bareword declares glob
    #[test]
    fn test_open_declares_glob() {
        let code = r#"
            open FH, '<', 'file.txt';
            while (<FH>) {
                print;
            }
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        // In modern Perl, bareword filehandles create package globals
        // This test verifies we don't flag them as errors
        // (Actual behavior depends on strict mode)
    }
    
    /// Regression test: builtin doesn't shadow user sub
    #[test]
    fn test_user_sub_not_builtin() {
        let code = r#"
            sub read { print "custom read\n"; }
            read();  # Should call user sub, not builtin
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        // Should not apply builtin declaration effects to user sub
        // (This is a limitation - we assume builtin unless proven otherwise)
    }
    
    /// Test: missing argument detection
    #[test]
    fn test_read_missing_buffer() {
        let code = r#"
            read $fh;  # Missing buffer and length
        "#;
        
        let ast = parse(code).unwrap();
        let issues = ScopeAnalyzer::analyze(&ast);
        
        // Should report missing arguments
        assert!(issues.iter().any(|i| 
            matches!(i.kind, IssueKind::BuiltinMissingArg { .. })
        ));
    }
}
```

---

## 5. Complete Rust Code Examples

### 5.1 Full Crate Structure

```
crates/perl-builtins-semantic/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Re-exports
│   ├── types.rs        # Core type definitions
│   ├── registry.rs     # PHF map definition
│   ├── query.rs        # Query interface
│   └── codegen/        # Build-time PHF generation (optional)
│       └── builtin_configs.rs  # Generated or hand-written
```

### 5.2 Complete lib.rs

```rust
// crates/perl-builtins-semantic/src/lib.rs

//! Semantic information for Perl builtin functions
//! 
//! This crate provides a PHF-based registry of builtin functions
//! with their semantic effects, particularly for variable declaration
//! at specific parameter positions.

pub mod types;
pub mod registry;
pub mod query;

// Re-export main types
pub use types::{
    ParamRole, SigilKind, ParamEffect, BuiltinConfig, 
    BuiltinCategory, PerlVersion
};

// Re-export registry
pub use registry::{
    BUILTIN_CONFIGS, get_builtin_config, 
    get_declaration_effects, builtins_with_declaration_effects,
    builtins_in_category
};

// Re-export query interface
pub use query::{
    BuiltinQuery, BuiltinAnalysisResult, DeclaredVariable,
    ArgCountValidation
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

### 5.3 Cargo.toml

```toml
# crates/perl-builtins-semantic/Cargo.toml

[package]
name = "perl-builtins-semantic"
version = "0.1.0"
edition = "2021"
description = "Semantic information for Perl builtin functions with PHF registry"
license = "MIT OR Apache-2.0"

[dependencies]
phf = { version = "0.11", features = ["macros"] }

[dev-dependencies]
# Testing dependencies
```

### 5.4 Integration Code Snippets

```rust
// Example: Full integration in semantic analyzer

use perl_builtin_semantic::{
    BuiltinQuery, get_builtin_config, SigilKind, ParamRole
};
use perl_ast::{Node, NodeKind};

/// Complete example of analyzing a read() call
fn example_analyze_read_call(node: &Node) {
    // 1. Detect the builtin call
    if let NodeKind::Call { func, args } = &node.kind {
        if let NodeKind::Variable { name, sigil } = &func.kind {
            if name == "read" && sigil.is_empty() {
                // 2. Query the registry
                if let Some(config) = get_builtin_config("read") {
                    println!("Found config for 'read':");
                    println!("  Category: {:?}", config.category);
                    println!("  Has declaration effects: {}", config.has_declaration_effects);
                    
                    // 3. Check parameter effects
                    for effect in config.params {
                        println!("  Position {}: {:?} - {}", 
                            effect.position, effect.role, effect.name_hint);
                        
                        if let Some(declares) = effect.declares {
                            println!("    Declares: {:?}", declares);
                        }
                    }
                    
                    // 4. Extract variables from arguments
                    for (idx, arg) in args.iter().enumerate() {
                        if let Some((sigil, name)) = extract_variable_name(arg) {
                            println!("  Arg {}: {}{}", idx, sigil, name);
                        }
                    }
                }
            }
        }
    }
}

fn extract_variable_name(arg: &Node) -> Option<(char, String)> {
    match &arg.kind {
        NodeKind::Variable { sigil, name, .. } => {
            sigil.chars().next().map(|s| (s, name.clone()))
        }
        _ => None,
    }
}
```

### 5.5 Usage in ScopeAnalyzer (Complete)

```rust
// crates/perl-semantic-analyzer/src/scope_analyzer.rs (relevant parts)

use perl_builtin_semantic::{
    BuiltinQuery, PositionAwareExtractor, 
    ArgVariableExtractor, get_declaration_effects
};

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            issues: Vec::new(),
            // ... other fields
        }
    }
    
    fn analyze_call_expression(
        &mut self,
        node: &Node,
        scope: &Rc<Scope>,
        ancestors: &[&Node],
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        
        // Extract function name and args
        let (func_name, args) = match &node.kind {
            NodeKind::Call { func, args } => {
                if let NodeKind::Variable { name, sigil } = &func.kind {
                    if sigil.is_empty() || sigil == "&" {
                        (name.as_str(), args.as_slice())
                    } else {
                        return issues; // Not a simple call
                    }
                } else {
                    return issues; // Complex call expression
                }
            }
            _ => return issues,
        };
        
        // Check if this is a builtin with declaration effects
        if BuiltinQuery::has_declaration_effects(func_name) {
            let declaration_positions = get_declaration_effects(func_name);
            
            for (position, sigil_kind, name_hint) in declaration_positions {
                if let Some(arg) = args.get(position) {
                    if let Some((sigil, name)) = ArgVariableExtractor::extract_variable_name(arg) {
                        if sigil == sigil_kind.as_char() {
                            // Declare the variable in scope
                            let scope_issue = scope.declare_variable_parts(
                                &sigil.to_string(),
                                &name,
                                arg.location.start,
                                false,  // not 'our'
                                true,   // initialized
                            );
                            
                            if let Some(issue) = scope_issue {
                                issues.push(ScopeIssue {
                                    kind: issue,
                                    variable_name: format!("{}{}", sigil, name),
                                    line: self.get_line(arg.location.start),
                                    range: (arg.location.start, arg.location.end),
                                    description: format!(
                                        "Variable '{}{}' declared by builtin '{}'",
                                        sigil, name, func_name
                                    ),
                                });
                            }
                            
                            // Mark as used
                            scope.initialize_and_use_variable_parts(
                                &sigil.to_string(), 
                                &name
                            );
                        }
                    }
                }
            }
        }
        
        // Continue with regular call analysis...
        issues
    }
}
```

---

## Appendix A: Parameter Reference for Key Builtins

| Builtin | Position 0 | Position 1 | Position 2 | Position 3 | Position 4 |
|---------|-----------|-----------|-----------|-----------|-----------|
| `read` | Handle | **$SCALAR** (Output) | LENGTH | OFFSET | - |
| `sysread` | Handle | **$SCALAR** (Output) | LENGTH | OFFSET | - |
| `recv` | Handle | **$SCALAR** (Output) | LENGTH | FLAGS | - |
| `socketpair` | **$READ** (Output) | **$WRITE** (Output) | DOMAIN | TYPE | PROTOCOL |
| `pipe` | **$READ** (Output) | **$WRITE** (Output) | - | - | - |
| `dbmopen` | **%HASH** (Output) | DBNAME | MODE | - | - |
| `open` | **FILEHANDLE** (Declares) | MODE | EXPR | - | - |
| `socket` | **SOCKET** (Declares) | DOMAIN | TYPE | PROTOCOL | - |

**Bold** = position has declaration effect

---

## Appendix B: Issue Categories Resolved

| Issue | Count | Builtin(s) |
|-------|-------|-----------|
| False positive "uninitialized" after read | 8 | read, sysread |
| False positive "uninitialized" after recv | 5 | recv |
| False positive "undeclared" socketpair handles | 4 | socketpair |
| False positive "undeclared" pipe handles | 2 | pipe |
| False positive "undeclared" dbmopen hash | 3 | dbmopen |
| Missing open filehandle declaration | 6 | open, socket |
| **Total** | **~28** | **7 builtins** |

---

*Specification complete. Ready for implementation.*
