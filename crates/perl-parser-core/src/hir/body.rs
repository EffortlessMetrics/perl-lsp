//! HIR body graph: arena-based expression/statement/block representation.
//!
//! This module provides the first vertical slice of HIR body infrastructure,
//! implementing the arena-based graph described in ADR #2564. It introduces:
//!
//! - Typed arena indices: [`HirBodyId`], [`HirExprId`], [`HirStmtId`], [`HirBlockId`]
//! - Per-body arenas and a source map: [`HirBody`], [`BodySourceMap`]
//! - Body owners (program root + subroutine): [`BodyOwnerKind`]
//! - Expression/statement/block node taxonomy: [`HirExpr`], [`HirStmt`], [`HirBlock`]
//! - A lowering entry point for one Perl source string: [`lower_body`]
//!
//! # Specimen
//!
//! The first vertical slice lowers exactly one specimen end-to-end:
//!
//! ```text
//! my $x = $a + $b;
//! ```
//!
//! This produces a [`HirBody`] with:
//! - One [`HirStmt::Let`] binding `$x`
//! - One [`HirExpr::Binary`] `+` node with children reading `$a` and `$b`
//! - One [`HirExpr::Assign`] connecting the decl place to the binary value
//! - All nodes carry exact byte-offset source ranges in [`BodySourceMap`]

use crate::SourceLocation;

// ──────────────────────────────────────────────────────────────────────────────
// Typed arena indices
// ──────────────────────────────────────────────────────────────────────────────

/// Stable index into the workspace body registry (one per body owner).
///
/// Currently unused at runtime — reserved so that cross-body references can be
/// introduced later without changing existing index types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HirBodyId(pub u32);

/// Typed index into a [`HirBody`]'s expression arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HirExprId(pub u32);

/// Typed index into a [`HirBody`]'s statement arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HirStmtId(pub u32);

/// Typed index into a [`HirBody`]'s block arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HirBlockId(pub u32);

// ──────────────────────────────────────────────────────────────────────────────
// Arena
// ──────────────────────────────────────────────────────────────────────────────

/// Typed append-only arena indexed by a strongly-typed ID.
///
/// The ID type must implement a constructor from `u32`; see the `impl` blocks
/// below for each concrete instantiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena<T> {
    items: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Arena<T> {
    /// Push a value and return its index.
    pub fn alloc(&mut self, value: T) -> u32 {
        let id = self.items.len() as u32;
        self.items.push(value);
        id
    }

    /// Return a reference to the value at `index`.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<&T> {
        self.items.get(index as usize)
    }

    /// Return the number of allocated items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return true when no items have been allocated.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over all allocated items in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Source map
// ──────────────────────────────────────────────────────────────────────────────

/// Source map for one [`HirBody`].
///
/// Each arena entry in [`HirBody`] has a corresponding `SourceLocation` here,
/// indexed by the same raw `u32` that the typed ID wraps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BodySourceMap {
    /// Source range for each expression, indexed by [`HirExprId`] value.
    pub expr_ranges: Vec<SourceLocation>,
    /// Source range for each statement, indexed by [`HirStmtId`] value.
    pub stmt_ranges: Vec<SourceLocation>,
    /// Source range for each block, indexed by [`HirBlockId`] value.
    pub block_ranges: Vec<SourceLocation>,
}

impl BodySourceMap {
    /// Look up the source range for an expression.
    pub fn expr_range(&self, id: HirExprId) -> Option<SourceLocation> {
        self.expr_ranges.get(id.0 as usize).copied()
    }

    /// Look up the source range for a statement.
    pub fn stmt_range(&self, id: HirStmtId) -> Option<SourceLocation> {
        self.stmt_ranges.get(id.0 as usize).copied()
    }

    /// Look up the source range for a block.
    pub fn block_range(&self, id: HirBlockId) -> Option<SourceLocation> {
        self.block_ranges.get(id.0 as usize).copied()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Body owner
// ──────────────────────────────────────────────────────────────────────────────

/// What syntactic construct owns this body.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyOwnerKind {
    /// Top-level program root (file-level statement sequence).
    ProgramRoot,
    /// Named subroutine body (`sub foo { ... }`).
    Subroutine {
        /// Subroutine name, or `None` for anonymous subs.
        name: Option<String>,
    },
    /// Method body (`method foo { ... }`).
    Method {
        /// Method name.
        name: String,
    },
}

/// Stable key for a body in the per-file body registry.
///
/// The ordinal disambiguates multiple anonymous subroutines or method bodies
/// with the same name in one file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyOwner {
    /// What owns this body.
    pub kind: BodyOwnerKind,
    /// Zero-based ordinal for disambiguation within one file.
    pub ordinal: u32,
}

impl BodyOwner {
    /// Create a `BodyOwner` key.
    pub fn new(kind: BodyOwnerKind, ordinal: u32) -> Self {
        Self { kind, ordinal }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Expression nodes
// ──────────────────────────────────────────────────────────────────────────────

/// Sigil for a variable reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sigil {
    /// `$` — scalar.
    Scalar,
    /// `@` — array.
    Array,
    /// `%` — hash.
    Hash,
    /// `&` — code ref / sub.
    Code,
    /// `*` — typeglob.
    Glob,
}

impl Sigil {
    /// Parse from a sigil character string as produced by the AST.
    fn from_str(s: &str) -> Self {
        match s {
            "$" => Sigil::Scalar,
            "@" => Sigil::Array,
            "%" => Sigil::Hash,
            "&" => Sigil::Code,
            "*" => Sigil::Glob,
            _ => Sigil::Scalar, // graceful fallback; should not occur
        }
    }
}

/// How a variable is accessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessMode {
    /// Value is being read.
    Read,
    /// Variable is the target of an assignment (lexical place / lvalue).
    Write,
    /// Variable is both read and written — compound assignment or `++`/`--`.
    ReadModifyWrite,
}

/// Variable origin classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableKind {
    /// `my`-declared lexical in scope.
    Lexical,
    /// Package / stash variable.
    Package,
}

/// A variable reference node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirVariable {
    /// Variable sigil.
    pub sigil: Sigil,
    /// Variable name without sigil.
    pub name: String,
    /// Lexical or package origin.
    pub kind: VariableKind,
    /// How this node uses the variable.
    pub access: AccessMode,
}

/// How an assignment is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignMode {
    /// Simple `=` assignment.
    Simple,
    /// Compound assignment: `+=`, `-=`, `*=`, etc.
    ///
    /// The LHS is both read (to compute the new value) and written (to store
    /// the result).  The LHS variable node carries [`AccessMode::ReadModifyWrite`].
    ReadModifyWrite,
}

/// How a unary operator accesses its operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryMode {
    /// Pure read (e.g. unary minus, `!`).
    Read,
    /// Read-modify-write (e.g. `++`, `--`).
    ReadModifyWrite,
}

/// Binary operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    /// Numeric addition `+`.
    Add,
    /// Numeric subtraction `-`.
    Sub,
    /// Numeric multiplication `*`.
    Mul,
    /// Numeric division `/`.
    Div,
    /// String concatenation `.`.
    Concat,
    /// Other/unknown operator — preserves the original text.
    Other(String),
}

impl BinaryOp {
    fn from_str(s: &str) -> Self {
        match s {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "." => BinaryOp::Concat,
            other => BinaryOp::Other(other.to_string()),
        }
    }
}

/// One expression node in the HIR body graph.
///
/// Every variant that has child expressions carries explicit [`HirExprId`]
/// references — there are no flat shells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirExpr {
    /// Variable read or write-place reference.
    Variable(HirVariable),

    /// Binary expression: `lhs OP rhs`, both children are explicit IDs.
    Binary {
        /// Left-hand operand.
        lhs: HirExprId,
        /// Operator.
        op: BinaryOp,
        /// Right-hand operand.
        rhs: HirExprId,
    },

    /// Assignment expression: `lhs = rhs`, both sides are explicit IDs.
    ///
    /// For `my $x = …`, the `lhs` is a `Variable` node with `access: Write`
    /// representing the declared place, and `rhs` is the initializer.
    Assign {
        /// Assignment target (place / lvalue).
        lhs: HirExprId,
        /// Value being assigned.
        rhs: HirExprId,
        /// Assignment mode.
        mode: AssignMode,
    },

    /// Unary expression: `OP operand`.
    Unary {
        /// Operand.
        operand: HirExprId,
        /// How the operator accesses the operand.
        mode: UnaryMode,
        /// Operator text, for diagnostics.
        op: String,
    },

    /// Function/method call expression (first-pass model).
    ///
    /// Arguments that are individually lowerable carry explicit IDs; everything
    /// else is `Opaque`.
    Call {
        /// Argument expressions in source order.
        args: Vec<HirExprId>,
        /// The AST node kind name, for diagnostics.
        ast_kind: String,
    },

    /// Opaque expression — used when the AST shape is not yet modeled.
    Opaque {
        /// The AST node kind name for diagnostics.
        ast_kind: String,
    },
}

// ──────────────────────────────────────────────────────────────────────────────
// Statement nodes
// ──────────────────────────────────────────────────────────────────────────────

/// Storage class for a variable declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclStorageClass {
    /// `my` — lexical.
    My,
    /// `our` — package alias.
    Our,
    /// `local` — dynamic.
    Local,
    /// `state` — persistent lexical.
    State,
}

impl DeclStorageClass {
    fn from_str(s: &str) -> Self {
        match s {
            "my" => DeclStorageClass::My,
            "our" => DeclStorageClass::Our,
            "local" => DeclStorageClass::Local,
            "state" => DeclStorageClass::State,
            _ => DeclStorageClass::My,
        }
    }
}

/// One statement node in the HIR body graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirStmt {
    /// Expression statement: evaluate the expression for its side effects.
    Expr(HirExprId),

    /// Variable declaration: `my $x = …`
    ///
    /// The `init` expression, when present, is the full initializer (which may
    /// itself be an [`HirExpr::Assign`] that links the declared place to its
    /// value).
    Let {
        /// Variable name without sigil.
        name: String,
        /// Sigil of the declared variable.
        sigil: Sigil,
        /// Storage class: `my`, `our`, `local`, `state`.
        storage: DeclStorageClass,
        /// Optional initializer expression ID.
        init: Option<HirExprId>,
    },
}

// ──────────────────────────────────────────────────────────────────────────────
// Block node
// ──────────────────────────────────────────────────────────────────────────────

/// A sequenced list of statements — the building block of every body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HirBlock {
    /// Ordered statement IDs in source order.
    pub stmts: Vec<HirStmtId>,
}

// ──────────────────────────────────────────────────────────────────────────────
// HirBody
// ──────────────────────────────────────────────────────────────────────────────

/// Arena-based expression/statement/block graph for one body owner.
///
/// This is the unit of lowering produced by [`lower_body`]. It is separate
/// from [`crate::hir::HirFile`]'s flat item list — flat items remain for
/// compile-time fact extraction; bodies are the new representation for data-flow
/// analysis, context propagation, and PIR-A lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirBody {
    /// All expression nodes, indexed by [`HirExprId`].
    pub exprs: Arena<HirExpr>,
    /// All statement nodes, indexed by [`HirStmtId`].
    pub stmts: Arena<HirStmt>,
    /// All block nodes (ordered statement sequences), indexed by [`HirBlockId`].
    pub blocks: Arena<HirBlock>,
    /// Source map: maps each expr/stmt/block index to its [`SourceLocation`].
    pub source_map: BodySourceMap,
    /// Root block — the entry point for the body.
    pub root_block: HirBlockId,
    /// What syntactic construct owns this body.
    pub owner: BodyOwnerKind,
}

impl HirBody {
    /// Look up an expression node by ID.
    pub fn expr(&self, id: HirExprId) -> Option<&HirExpr> {
        self.exprs.get(id.0)
    }

    /// Look up a statement node by ID.
    pub fn stmt(&self, id: HirStmtId) -> Option<&HirStmt> {
        self.stmts.get(id.0)
    }

    /// Look up a block node by ID.
    pub fn block(&self, id: HirBlockId) -> Option<&HirBlock> {
        self.blocks.get(id.0)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Body builder (internal)
// ──────────────────────────────────────────────────────────────────────────────

struct BodyBuilder {
    exprs: Arena<HirExpr>,
    stmts: Arena<HirStmt>,
    blocks: Arena<HirBlock>,
    source_map: BodySourceMap,
}

impl BodyBuilder {
    fn new() -> Self {
        Self {
            exprs: Arena::default(),
            stmts: Arena::default(),
            blocks: Arena::default(),
            source_map: BodySourceMap::default(),
        }
    }

    fn alloc_expr(&mut self, expr: HirExpr, range: SourceLocation) -> HirExprId {
        let idx = self.exprs.alloc(expr);
        self.source_map.expr_ranges.push(range);
        HirExprId(idx)
    }

    fn alloc_stmt(&mut self, stmt: HirStmt, range: SourceLocation) -> HirStmtId {
        let idx = self.stmts.alloc(stmt);
        self.source_map.stmt_ranges.push(range);
        HirStmtId(idx)
    }

    fn alloc_block(&mut self, block: HirBlock, range: SourceLocation) -> HirBlockId {
        let idx = self.blocks.alloc(block);
        self.source_map.block_ranges.push(range);
        HirBlockId(idx)
    }

    fn finish(self, root_block: HirBlockId, owner: BodyOwnerKind) -> HirBody {
        HirBody {
            exprs: self.exprs,
            stmts: self.stmts,
            blocks: self.blocks,
            source_map: self.source_map,
            root_block,
            owner,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Lowering
// ──────────────────────────────────────────────────────────────────────────────

use crate::{Node, NodeKind};

/// Lower the top-level program-root body from a parsed AST node.
///
/// This is the entry point for the first vertical slice. It lowers the
/// statements inside a `Program` node into an arena-based [`HirBody`].
///
/// Currently handled constructs:
/// - `my $x = EXPR;` — [`HirStmt::Let`] with an [`HirExpr::Assign`] initializer
/// - `$a + $b` — [`HirExpr::Binary`] with explicit child IDs
/// - Variable references — [`HirExpr::Variable`]
/// - Everything else — [`HirExpr::Opaque`] / [`HirStmt::Expr`] fallback
pub fn lower_body(ast: &Node) -> HirBody {
    let mut builder = BodyBuilder::new();

    let stmts = match &ast.kind {
        NodeKind::Program { statements } => statements.as_slice(),
        _ => std::slice::from_ref(ast),
    };

    let root_range = ast.location;
    let mut root_block = HirBlock::default();

    for stmt_node in stmts {
        let stmt_id = lower_statement(&mut builder, stmt_node);
        root_block.stmts.push(stmt_id);
    }

    let root_id = builder.alloc_block(root_block, root_range);
    builder.finish(root_id, BodyOwnerKind::ProgramRoot)
}

fn lower_statement(builder: &mut BodyBuilder, node: &Node) -> HirStmtId {
    let range = node.location;

    match &node.kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            // Extract variable name and sigil from the inner Variable node.
            let (sigil_str, var_name) = match &variable.kind {
                NodeKind::Variable { sigil, name } => (sigil.as_str(), name.clone()),
                _ => ("$", String::from("<unknown>")),
            };
            let sigil = Sigil::from_str(sigil_str);
            let storage = DeclStorageClass::from_str(declarator);

            let init_expr_id = initializer.as_ref().map(|init_node| {
                // The initializer is the full RHS expression.
                // For `my $x = $a + $b`, the AST may represent this as:
                //   VariableDeclaration { variable: $x, initializer: Binary(+, $a, $b) }
                // We model the assignment as:
                //   Assign { lhs: Variable($x, Write), rhs: lower_expr(initializer) }

                // Allocate the write-place for $x
                let place_expr = HirExpr::Variable(HirVariable {
                    sigil: Sigil::from_str(sigil_str),
                    name: var_name.clone(),
                    kind: VariableKind::Lexical,
                    access: AccessMode::Write,
                });
                let place_id = builder.alloc_expr(place_expr, variable.location);

                // Lower the RHS value expression
                let rhs_id = lower_expr(builder, init_node);

                // Wrap in an Assign node spanning the full declaration range
                let assign_range =
                    SourceLocation { start: variable.location.start, end: init_node.location.end };
                let assign_expr =
                    HirExpr::Assign { lhs: place_id, rhs: rhs_id, mode: AssignMode::Simple };
                builder.alloc_expr(assign_expr, assign_range)
            });

            builder.alloc_stmt(
                HirStmt::Let { name: var_name, sigil, storage, init: init_expr_id },
                range,
            )
        }

        // Expression statement fallback
        _ => {
            let expr_id = lower_expr(builder, node);
            builder.alloc_stmt(HirStmt::Expr(expr_id), range)
        }
    }
}

fn lower_expr(builder: &mut BodyBuilder, node: &Node) -> HirExprId {
    let range = node.location;

    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            let var = HirVariable {
                sigil: Sigil::from_str(sigil),
                name: name.clone(),
                kind: VariableKind::Lexical,
                access: AccessMode::Read,
            };
            builder.alloc_expr(HirExpr::Variable(var), range)
        }

        NodeKind::Binary { op, left, right } => {
            let lhs_id = lower_expr(builder, left);
            let rhs_id = lower_expr(builder, right);
            let binary_op = BinaryOp::from_str(op);
            builder.alloc_expr(HirExpr::Binary { lhs: lhs_id, op: binary_op, rhs: rhs_id }, range)
        }

        NodeKind::Assignment { lhs, rhs, .. } => {
            let lhs_id = lower_expr(builder, lhs);
            let rhs_id = lower_expr(builder, rhs);
            builder.alloc_expr(
                HirExpr::Assign { lhs: lhs_id, rhs: rhs_id, mode: AssignMode::Simple },
                range,
            )
        }

        _ => {
            let kind_name = node.kind.kind_name().to_string();
            builder.alloc_expr(HirExpr::Opaque { ast_kind: kind_name }, range)
        }
    }
}
