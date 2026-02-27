use crate::ast::{Node, NodeKind};
use std::collections::HashMap;
use std::sync::Arc;

/// Represents a Perl type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PerlType {
    /// Scalar types
    Scalar(ScalarType),
    /// Array type
    Array(Box<PerlType>),
    /// Hash type with key and value types
    Hash {
        /// The type of keys in the hash
        key: Box<PerlType>,
        /// The type of values in the hash
        value: Box<PerlType>,
    },
    /// Reference to another type
    Reference(Box<PerlType>),
    /// Subroutine type with parameter and return types
    Subroutine {
        /// Types of the subroutine parameters
        params: Vec<PerlType>,
        /// Types of the subroutine return values
        returns: Vec<PerlType>,
    },
    /// Object type with class name
    Object(String),
    /// Glob/typeglob type
    Glob,
    /// Union of multiple possible types
    Union(Vec<PerlType>),
    /// Unknown or any type
    Any,
    /// Void/no return value
    Void,
}

/// Represents specific scalar types in Perl
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// String value (e.g., "hello")
    String,
    /// Integer value (e.g., 42)
    Integer,
    /// Floating-point value (e.g., 3.14)
    Float,
    /// Boolean value (true/false context)
    Boolean,
    /// Undefined value
    Undef,
    /// Mixed scalar type (can be any scalar)
    Mixed,
}

/// Type constraint for type checking
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    /// The expected type based on context
    pub expected: PerlType,
    /// The actual inferred type
    pub actual: PerlType,
    /// Source location where the constraint was generated
    pub location: TypeLocation,
    /// Human-readable explanation for the constraint
    pub reason: String,
}

/// Location information for type errors
#[derive(Debug, Clone)]
pub struct TypeLocation {
    /// File path where the type issue occurred
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Surrounding code context for error messages
    pub context: String,
}

/// Type environment for tracking variable types
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Variable types in current scope
    variables: HashMap<String, PerlType>,
    /// Subroutine signatures
    subroutines: HashMap<String, PerlType>,
    /// Parent scope (for nested scopes)
    parent: Option<Box<TypeEnvironment>>,
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnvironment {
    /// Creates a new empty type environment
    pub fn new() -> Self {
        Self { variables: HashMap::new(), subroutines: HashMap::new(), parent: None }
    }

    /// Creates a new type environment with a parent scope
    pub fn with_parent(parent: TypeEnvironment) -> Self {
        Self {
            variables: HashMap::new(),
            subroutines: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Sets the type for a variable in the current scope
    pub fn set_variable(&mut self, name: String, ty: PerlType) {
        self.variables.insert(name, ty);
    }

    /// Gets the type of a variable, searching parent scopes if needed
    pub fn get_variable(&self, name: &str) -> Option<&PerlType> {
        self.variables.get(name).or_else(|| self.parent.as_ref().and_then(|p| p.get_variable(name)))
    }

    /// Sets the type signature for a subroutine in the current scope
    pub fn set_subroutine(&mut self, name: String, ty: PerlType) {
        self.subroutines.insert(name, ty);
    }

    /// Gets the type signature of a subroutine, searching parent scopes if needed
    pub fn get_subroutine(&self, name: &str) -> Option<&PerlType> {
        self.subroutines
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_subroutine(name)))
    }
}

/// Main type inference engine
pub struct TypeInferenceEngine {
    /// Global type environment
    global_env: TypeEnvironment,
    /// Type constraints collected during inference
    constraints: Vec<TypeConstraint>,
    /// Built-in function signatures
    builtins: HashMap<String, PerlType>,
    /// Type aliases from use statements
    _type_aliases: HashMap<String, PerlType>,
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferenceEngine {
    /// Creates a new type inference engine with built-in function signatures
    pub fn new() -> Self {
        let mut engine = Self {
            global_env: TypeEnvironment::new(),
            constraints: Vec::new(),
            builtins: HashMap::new(),
            _type_aliases: HashMap::new(),
        };

        // Initialize built-in function types
        engine.init_builtins();

        engine
    }

    fn init_builtins(&mut self) {
        use PerlType::*;
        use ScalarType::*;

        // String functions
        self.builtins.insert(
            "length".to_string(),
            Subroutine { params: vec![Scalar(String)], returns: vec![Scalar(Integer)] },
        );

        self.builtins.insert(
            "substr".to_string(),
            Subroutine {
                params: vec![Scalar(String), Scalar(Integer), Scalar(Integer)],
                returns: vec![Scalar(String)],
            },
        );

        // Array functions
        self.builtins.insert(
            "push".to_string(),
            Subroutine { params: vec![Array(Box::new(Any)), Any], returns: vec![Scalar(Integer)] },
        );

        self.builtins.insert(
            "pop".to_string(),
            Subroutine { params: vec![Array(Box::new(Any))], returns: vec![Any] },
        );

        // Hash functions
        self.builtins.insert(
            "keys".to_string(),
            Subroutine {
                params: vec![Hash { key: Box::new(Scalar(String)), value: Box::new(Any) }],
                returns: vec![Array(Box::new(Scalar(String)))],
            },
        );

        // I/O functions
        self.builtins.insert(
            "print".to_string(),
            Subroutine { params: vec![Any], returns: vec![Scalar(Boolean)] },
        );

        self.builtins.insert(
            "open".to_string(),
            Subroutine {
                params: vec![Glob, Scalar(String), Scalar(String)],
                returns: vec![Scalar(Boolean)],
            },
        );

        // Reference functions
        self.builtins.insert(
            "ref".to_string(),
            Subroutine { params: vec![Any], returns: vec![Scalar(String)] },
        );

        // Type checking functions
        self.builtins.insert(
            "defined".to_string(),
            Subroutine { params: vec![Any], returns: vec![Scalar(Boolean)] },
        );
    }

    /// Infer types for an AST
    pub fn infer(&mut self, ast: &Node) -> Result<PerlType, Vec<TypeConstraint>> {
        // We need to use a temporary environment that has global_env as parent,
        // or just use global_env directly if we want to persist top-level declarations?
        // Usually top-level declarations should persist.
        // But we can't borrow self.global_env mutably and also self methods easily if they need self.
        // infer_node takes &mut self and &mut env.

        // Let's create a temporary scope that is a child of the current global env,
        // OR just use global_env if we want top-level effects (like subs) to be visible.
        // But wait, `infer_node` modifies `env`. If we pass a clone, modifications are lost.
        // We WANT modifications to persist for the duration of `infer` call, but do we want them in `self.global_env`?
        // Yes, if we want to query them later via `get_subroutine`.

        // However, `infer_node` calls `self.infer_node` recursively.
        // Let's avoid cloning `global_env` inside `infer`.

        // To satisfy borrow checker (if we pass &mut self.global_env to self.infer_node),
        // we might have issues. `infer_node` takes `&mut self`.
        // So `self` is borrowed mutably. We can't pass a reference to a field of `self`.

        // Solution: Temporarily take `global_env` out of `self`, use it, then put it back?
        // Or change `infer_node` signature?

        // For now, let's just make `infer` work by swapping.
        let mut env = std::mem::take(&mut self.global_env);
        let result = self.infer_node(ast, &mut env);
        self.global_env = env;

        let ty = result?;

        // Check constraints
        if !self.constraints.is_empty() {
            let violations: Vec<_> = self
                .constraints
                .iter()
                .filter(|c| !self.types_compatible(&c.expected, &c.actual))
                .cloned()
                .collect();

            if !violations.is_empty() {
                return Err(violations);
            }
        }

        Ok(ty)
    }

    /// Infer type for a single node
    fn infer_node(
        &mut self,
        node: &Node,
        env: &mut TypeEnvironment,
    ) -> Result<PerlType, Vec<TypeConstraint>> {
        use PerlType::*;
        use ScalarType::*;

        match &node.kind {
            NodeKind::Program { statements } => {
                let mut last_type = Void;
                for stmt in statements {
                    last_type = self.infer_node(stmt, env)?;
                }
                Ok(last_type)
            }

            // Handle expression statements by returning the type of the expression
            NodeKind::ExpressionStatement { expression } => self.infer_node(expression, env),

            NodeKind::Number { value } => {
                if value.contains('.') || value.contains('e') || value.contains('E') {
                    Ok(Scalar(Float))
                } else {
                    Ok(Scalar(Integer))
                }
            }

            NodeKind::String { .. } => Ok(Scalar(String)),

            NodeKind::Undef => Ok(Scalar(Undef)),

            NodeKind::Variable { sigil, name } => {
                // First check if we have a known type for this variable (use name without sigil)
                if let Some(ty) = env.get_variable(name) {
                    return Ok(ty.clone());
                }

                // Otherwise, infer from sigil
                match sigil.as_str() {
                    "$" => {
                        // Scalar variable
                        Ok(Scalar(Mixed))
                    }
                    "@" => {
                        // Array variable - store type for later retrieval
                        let array_type = Array(Box::new(Any));
                        self.global_env.set_variable(name.to_string(), array_type.clone());
                        Ok(array_type)
                    }
                    "%" => {
                        // Hash variable - store type for later retrieval
                        let hash_type =
                            Hash { key: Box::new(Scalar(String)), value: Box::new(Any) };
                        self.global_env.set_variable(name.to_string(), hash_type.clone());
                        Ok(hash_type)
                    }
                    "*" => {
                        // Glob variable
                        Ok(Glob)
                    }
                    _ => {
                        // Unknown variable type
                        Ok(Any)
                    }
                }
            }

            NodeKind::ArrayLiteral { elements } => {
                if elements.is_empty() {
                    Ok(Array(Box::new(Any)))
                } else {
                    // Infer element type from first element
                    let elem_type = self.infer_node(&elements[0], env)?;

                    // Check all elements have compatible types
                    for elem in &elements[1..] {
                        let ty = self.infer_node(elem, env)?;
                        if !self.types_compatible(&elem_type, &ty) {
                            // Mixed types, use Any
                            return Ok(Array(Box::new(Any)));
                        }
                    }

                    Ok(Array(Box::new(elem_type)))
                }
            }

            NodeKind::HashLiteral { pairs } => {
                if pairs.is_empty() {
                    return Ok(Hash { key: Box::new(Scalar(String)), value: Box::new(Any) });
                }

                // Collect all key and value types
                let mut key_types = Vec::new();
                let mut value_types = Vec::new();

                for (key, val) in pairs {
                    key_types.push(self.infer_node(key, env)?);
                    value_types.push(self.infer_node(val, env)?);
                }

                // Unify key types (typically strings)
                let key_type = self.unify_types(&key_types);

                // Unify value types - use smart unification
                let value_type = self.unify_types(&value_types);

                Ok(Hash { key: Box::new(key_type), value: Box::new(value_type) })
            }

            NodeKind::Binary { left, op, right } => {
                let left_ty = self.infer_node(left, env)?;
                let right_ty = self.infer_node(right, env)?;

                match op.as_str() {
                    // Arithmetic operators
                    "+" | "-" | "*" | "/" | "%" | "**" => {
                        // Expect numeric types
                        self.add_constraint(Scalar(Mixed), left_ty.clone(), "arithmetic operator");
                        self.add_constraint(Scalar(Mixed), right_ty.clone(), "arithmetic operator");
                        Ok(Scalar(Float))
                    }

                    // String operators
                    "." | "x" => {
                        self.add_constraint(Scalar(String), left_ty.clone(), "string operator");
                        Ok(Scalar(String))
                    }

                    // Comparison operators
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "eq" | "ne" | "lt" | "gt" | "le"
                    | "ge" | "<=>" | "cmp" => Ok(Scalar(Boolean)),

                    // Logical operators
                    "&&" | "||" | "and" | "or" | "xor" => Ok(Scalar(Boolean)),

                    // Assignment operators
                    "=" | "+=" | "-=" | "*=" | "/=" | ".=" => {
                        env.set_variable(self.extract_var_name(left), right_ty.clone());
                        Ok(right_ty)
                    }

                    _ => Ok(Any),
                }
            }

            NodeKind::Unary { op, operand } => {
                let operand_ty = self.infer_node(operand, env)?;

                match op.as_str() {
                    "!" | "not" => Ok(Scalar(Boolean)),
                    "-" | "+" => {
                        self.add_constraint(Scalar(Mixed), operand_ty.clone(), "numeric operator");
                        Ok(operand_ty)
                    }
                    "\\" => Ok(Reference(Box::new(operand_ty))),
                    _ => Ok(Any),
                }
            }

            NodeKind::FunctionCall { name, args: _ } => {
                let func_name = name.clone();

                // Check built-in functions
                if let Some(sig) = self.builtins.get(&func_name) {
                    if let Subroutine { returns, .. } = sig {
                        if returns.len() == 1 {
                            return Ok(returns[0].clone());
                        } else if returns.is_empty() {
                            return Ok(Void);
                        } else {
                            return Ok(Array(Box::new(returns[0].clone())));
                        }
                    }
                }

                // Check user-defined functions
                if let Some(ty) = env.get_subroutine(&func_name) {
                    if let Subroutine { returns, .. } = ty {
                        if returns.len() == 1 {
                            return Ok(returns[0].clone());
                        } else if returns.is_empty() {
                            return Ok(Void);
                        } else {
                            return Ok(Array(Box::new(returns[0].clone())));
                        }
                    }
                }

                // Unknown function, return Any
                Ok(Any)
            }

            NodeKind::Subroutine { name, body, .. } => {
                // Create new scope for subroutine
                let mut sub_env = TypeEnvironment::with_parent(env.clone());

                // Default to accepting any parameters for now
                let param_types = vec![Any];

                // Infer return type from body
                // In Perl, the return value is the value of the last statement
                // or explicit return statement.
                // Our `infer_node` for Block already returns the type of the last statement.
                // We should also scan for `Return` nodes, but for now we just use the block result
                // which handles the implicit return of the last statement.
                // Note: explicit returns inside control flow (if/else) are tricky to unify without
                // a full control flow graph, but `infer_node` recurses so it should pick up types.
                // Wait, `infer_node` for `Program`/`Block` returns the type of the *last* statement.
                // It does NOT unify types from intermediate `return` statements.

                // For better accuracy, we should probably scan the body for `Return` nodes?
                // But `infer_node(body)` calls `infer_node` recursively.
                // If we want to support explicit returns not at the end, we need to handle `Return` node
                // to return a special type or track returns in `TypeInferenceEngine`.

                // For now, let's trust `infer_node(body)` to return the implicit return type of the block.
                let return_type = self.infer_node(body, &mut sub_env)?;

                let sub_type = Subroutine { params: param_types, returns: vec![return_type] };

                // Register subroutine in environment
                if let Some(sub_name) = name {
                    env.set_subroutine(sub_name.clone(), sub_type.clone());
                }

                Ok(sub_type)
            }

            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                // Determine type from variable sigil and initializer
                let var_type = if let NodeKind::Variable { sigil, name } = &variable.kind {
                    // Strip sigil from name for storage
                    let clean_name = name.trim_start_matches(['$', '@', '%']);

                    // Infer type based on sigil and initializer
                    let inferred_type = if sigil == "@" {
                        // Array variable - infer element type from initializer if available
                        if let Some(init) = initializer {
                            self.infer_node(init, env)?
                        } else {
                            PerlType::Array(Box::new(PerlType::Any))
                        }
                    } else if sigil == "%" {
                        // Hash variable - infer key/value types from initializer if available
                        if let Some(init) = initializer {
                            // Check if initializer is an ArrayLiteral (which is how hash literals in parens are parsed)
                            if let NodeKind::ArrayLiteral { elements } = &init.kind {
                                // Convert array elements to hash type
                                if elements.is_empty() {
                                    PerlType::Hash {
                                        key: Box::new(PerlType::Scalar(ScalarType::String)),
                                        value: Box::new(PerlType::Any),
                                    }
                                } else if elements.len() % 2 == 0 {
                                    // Treat as key-value pairs
                                    let mut value_types = Vec::new();
                                    for i in (1..elements.len()).step_by(2) {
                                        value_types.push(self.infer_node(&elements[i], env)?);
                                    }
                                    let value_type = self.unify_types(&value_types);
                                    PerlType::Hash {
                                        key: Box::new(PerlType::Scalar(ScalarType::String)),
                                        value: Box::new(value_type),
                                    }
                                } else {
                                    // Odd number of elements - still treat as hash
                                    PerlType::Hash {
                                        key: Box::new(PerlType::Scalar(ScalarType::String)),
                                        value: Box::new(PerlType::Any),
                                    }
                                }
                            } else {
                                // Normal hash literal or other expression
                                self.infer_node(init, env)?
                            }
                        } else {
                            PerlType::Hash {
                                key: Box::new(PerlType::Scalar(ScalarType::String)),
                                value: Box::new(PerlType::Any),
                            }
                        }
                    } else {
                        // Scalar variable - infer from initializer
                        if let Some(init) = initializer {
                            self.infer_node(init, env)?
                        } else {
                            PerlType::Scalar(ScalarType::Undef)
                        }
                    };

                    // Store in both environments using the name WITHOUT sigil
                    self.global_env.set_variable(clean_name.to_string(), inferred_type.clone());
                    env.set_variable(clean_name.to_string(), inferred_type.clone());

                    inferred_type
                } else {
                    PerlType::Any
                };

                Ok(var_type)
            }

            NodeKind::If { condition, then_branch, else_branch, .. } => {
                let _cond_ty = self.infer_node(condition, env)?;

                let then_ty = self.infer_node(then_branch, env)?;

                let else_ty = if let Some(else_node) = else_branch {
                    self.infer_node(else_node, env)?
                } else {
                    Void
                };

                // Return union type if branches have different types
                if self.types_compatible(&then_ty, &else_ty) {
                    Ok(then_ty)
                } else if then_ty == Void {
                    Ok(else_ty)
                } else if else_ty == Void {
                    Ok(then_ty)
                } else {
                    Ok(Union(vec![then_ty, else_ty]))
                }
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    self.infer_node(val, env)
                } else {
                    Ok(Void)
                }
            }

            NodeKind::Block { statements } => {
                let mut last_type = Void;
                for stmt in statements {
                    last_type = self.infer_node(stmt, env)?;
                }
                Ok(last_type)
            }

            _ => Ok(Any), // Default for unhandled nodes
        }
    }

    /// Parse a subroutine signature
    #[allow(dead_code)]
    fn parse_signature(
        &mut self,
        _sig: &Node,
        _env: &mut TypeEnvironment,
    ) -> Result<Vec<PerlType>, Vec<TypeConstraint>> {
        // Simplified signature parsing
        // In a full implementation, this would parse Perl 5.20+ signatures
        Ok(vec![PerlType::Any])
    }

    /// Extract variable name from a node
    fn extract_var_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { name, .. } => name.trim_start_matches(['$', '@', '%']).to_string(),
            _ => String::new(),
        }
    }

    /// Extract function name from a node
    #[allow(dead_code)]
    fn extract_func_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Identifier { name } => name.clone(),
            _ => String::new(),
        }
    }

    /// Add a type constraint
    fn add_constraint(&mut self, expected: PerlType, actual: PerlType, reason: &str) {
        self.constraints.push(TypeConstraint {
            expected,
            actual,
            location: TypeLocation {
                file: String::new(),
                line: 0,
                column: 0,
                context: String::new(),
            },
            reason: reason.to_string(),
        });
    }

    /// Unify a collection of types into a single type
    fn unify_types(&self, types: &[PerlType]) -> PerlType {
        use PerlType::*;
        use ScalarType::*;

        if types.is_empty() {
            return Any;
        }

        if types.len() == 1 {
            return types[0].clone();
        }

        // Special case: all scalar types - handle first to get proper numeric unification
        let all_scalars = types.iter().all(|t| matches!(t, Scalar(_)));
        if all_scalars {
            // Check if all are numeric
            let all_numeric = types.iter().all(|t| matches!(t, Scalar(Integer) | Scalar(Float)));
            if all_numeric {
                // If any float, return float, else integer
                if types.iter().any(|t| matches!(t, Scalar(Float))) {
                    return Scalar(Float);
                } else {
                    return Scalar(Integer);
                }
            }

            // Check if all are strings
            let all_strings = types.iter().all(|t| matches!(t, Scalar(String)));
            if all_strings {
                return Scalar(String);
            }

            // Mixed scalar types
            return Scalar(Mixed);
        }

        // Check if all types are the same/compatible (after handling numeric unification)
        let first = &types[0];
        if types.iter().all(|t| self.types_compatible(first, t)) {
            return first.clone();
        }

        // Special case: all arrays with same element type
        let all_arrays = types.iter().all(|t| matches!(t, Array(_)));
        if all_arrays {
            let element_types: Vec<PerlType> =
                types
                    .iter()
                    .filter_map(|t| {
                        if let Array(elem) = t { Some(elem.as_ref().clone()) } else { None }
                    })
                    .collect();

            return Array(Box::new(self.unify_types(&element_types)));
        }

        // Special case: all hashes
        let all_hashes = types.iter().all(|t| matches!(t, Hash { .. }));
        if all_hashes {
            let mut key_types = Vec::new();
            let mut value_types = Vec::new();

            for t in types {
                if let Hash { key, value } = t {
                    key_types.push(key.as_ref().clone());
                    value_types.push(value.as_ref().clone());
                }
            }

            return Hash {
                key: Box::new(self.unify_types(&key_types)),
                value: Box::new(self.unify_types(&value_types)),
            };
        }

        // Heterogeneous types - create union or return Any
        if types.len() <= 3 {
            // Small number of types, use union
            Union(types.to_vec())
        } else {
            // Too many types, use Any
            Any
        }
    }

    /// Check if two types are compatible
    fn types_compatible(&self, t1: &PerlType, t2: &PerlType) -> bool {
        use PerlType::*;

        match (t1, t2) {
            (Any, _) | (_, Any) => true,
            (Scalar(s1), Scalar(s2)) => self.scalars_compatible(s1, s2),
            (Array(e1), Array(e2)) => self.types_compatible(e1, e2),
            (Hash { key: k1, value: v1 }, Hash { key: k2, value: v2 }) => {
                self.types_compatible(k1, k2) && self.types_compatible(v1, v2)
            }
            (Reference(r1), Reference(r2)) => self.types_compatible(r1, r2),
            (Union(types), other) | (other, Union(types)) => {
                types.iter().any(|t| self.types_compatible(t, other))
            }
            _ => t1 == t2,
        }
    }

    /// Check if two scalar types are compatible
    fn scalars_compatible(&self, s1: &ScalarType, s2: &ScalarType) -> bool {
        use ScalarType::*;

        match (s1, s2) {
            (Mixed, _) | (_, Mixed) => true,
            (Integer, Float) | (Float, Integer) => true, // Numeric coercion
            (String, Integer) | (Integer, String) => true, // String-number coercion in Perl
            (String, Float) | (Float, String) => true,
            _ => s1 == s2,
        }
    }

    /// Gets the inferred type for a variable by name
    pub fn get_type_at(&self, name: &str) -> Option<PerlType> {
        self.global_env.get_variable(name).cloned()
    }

    /// Gets the inferred type signature for a subroutine
    pub fn get_subroutine(&self, name: &str) -> Option<PerlType> {
        self.global_env.get_subroutine(name).cloned()
    }

    /// Returns all type constraint violations as errors
    pub fn get_type_errors(&self) -> Vec<TypeConstraint> {
        self.constraints
            .iter()
            .filter(|c| !self.types_compatible(&c.expected, &c.actual))
            .cloned()
            .collect()
    }
}

/// Type-based code completion suggestions
pub struct TypeBasedCompletion {
    /// Shared reference to the type inference engine
    engine: Arc<TypeInferenceEngine>,
}

impl TypeBasedCompletion {
    /// Creates a new completion provider with a shared type inference engine
    pub fn new(engine: Arc<TypeInferenceEngine>) -> Self {
        Self { engine }
    }

    /// Returns completions based on the inferred type of a variable
    pub fn get_completions(&self, var_name: &str, _context: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        if let Some(var_type) = self.engine.get_type_at(var_name) {
            match var_type {
                PerlType::Array(_) => {
                    // Array methods
                    completions.push(CompletionItem {
                        label: "push".to_string(),
                        detail: "push(@array, $item)".to_string(),
                        documentation: "Append items to array".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "pop".to_string(),
                        detail: "pop(@array)".to_string(),
                        documentation: "Remove and return last element".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "shift".to_string(),
                        detail: "shift(@array)".to_string(),
                        documentation: "Remove and return first element".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "unshift".to_string(),
                        detail: "unshift(@array, $item)".to_string(),
                        documentation: "Prepend items to array".to_string(),
                    });
                }
                PerlType::Hash { .. } => {
                    // Hash methods
                    completions.push(CompletionItem {
                        label: "keys".to_string(),
                        detail: "keys(%hash)".to_string(),
                        documentation: "Get all hash keys".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "values".to_string(),
                        detail: "values(%hash)".to_string(),
                        documentation: "Get all hash values".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "exists".to_string(),
                        detail: "exists($hash{$key})".to_string(),
                        documentation: "Check if key exists".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "delete".to_string(),
                        detail: "delete($hash{$key})".to_string(),
                        documentation: "Delete hash entry".to_string(),
                    });
                }
                PerlType::Scalar(ScalarType::String) => {
                    // String methods
                    completions.push(CompletionItem {
                        label: "length".to_string(),
                        detail: "length($string)".to_string(),
                        documentation: "Get string length".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "substr".to_string(),
                        detail: "substr($string, $offset, $length)".to_string(),
                        documentation: "Extract substring".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "index".to_string(),
                        detail: "index($string, $substring)".to_string(),
                        documentation: "Find substring position".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "uc".to_string(),
                        detail: "uc($string)".to_string(),
                        documentation: "Convert to uppercase".to_string(),
                    });
                    completions.push(CompletionItem {
                        label: "lc".to_string(),
                        detail: "lc($string)".to_string(),
                        documentation: "Convert to lowercase".to_string(),
                    });
                }
                PerlType::Object(class) => {
                    // Object methods would be looked up from class definition
                    completions.push(CompletionItem {
                        label: "isa".to_string(),
                        detail: format!("${}->isa($class)", var_name),
                        documentation: format!("Check if object is instance of {}", class),
                    });
                    completions.push(CompletionItem {
                        label: "can".to_string(),
                        detail: format!("${}->can($method)", var_name),
                        documentation: "Check if object has method".to_string(),
                    });
                }
                _ => {}
            }
        }

        completions
    }
}

/// A code completion suggestion
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// Short display label for the completion
    pub label: String,
    /// Additional detail shown alongside the label (e.g., signature)
    pub detail: String,
    /// Full documentation for the completion item
    pub documentation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_scalar_type_inference() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            my $x = 42;
            my $y = "hello";
            my $z = 3.14;
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let result = engine.infer(&ast);

        assert!(result.is_ok());
        assert_eq!(engine.get_type_at("x"), Some(PerlType::Scalar(ScalarType::Integer)));
        assert_eq!(engine.get_type_at("y"), Some(PerlType::Scalar(ScalarType::String)));
        assert_eq!(engine.get_type_at("z"), Some(PerlType::Scalar(ScalarType::Float)));
    }

    #[test]
    fn test_array_type_inference() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            my @numbers = (1, 2, 3);
            my @strings = ("a", "b", "c");
            my @mixed = (1, "hello", 3.14);
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let _result = engine.infer(&ast);

        assert!(matches!(engine.get_type_at("numbers"), Some(PerlType::Array(_))));
        assert!(matches!(engine.get_type_at("strings"), Some(PerlType::Array(_))));
        assert!(matches!(engine.get_type_at("mixed"), Some(PerlType::Array(_))));
    }

    #[test]
    fn test_hash_type_inference() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            my %numbers = (a => 1, b => 2, c => 3);
            my %mixed = (num => 42, float => 3.14);
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let _result = engine.infer(&ast);

        // Check that hash types are properly inferred
        let numbers_type = engine.get_type_at("numbers");
        assert!(
            matches!(numbers_type, Some(PerlType::Hash { .. })),
            "Expected hash type for numbers, got {:?}",
            numbers_type
        );
        if let Some(PerlType::Hash { value, .. }) = numbers_type {
            assert!(matches!(
                value.as_ref(),
                &PerlType::Scalar(ScalarType::Integer) | &PerlType::Any
            ));
        }

        let mixed_type = engine.get_type_at("mixed");
        assert!(
            matches!(mixed_type, Some(PerlType::Hash { .. })),
            "Expected hash type for mixed, got {:?}",
            mixed_type
        );
        if let Some(PerlType::Hash { value, .. }) = mixed_type {
            // Mixed types (int and float) should unify to Float
            assert!(matches!(
                value.as_ref(),
                &PerlType::Scalar(ScalarType::Float)
                    | &PerlType::Scalar(ScalarType::Mixed)
                    | &PerlType::Any
            ));
        }
    }

    #[test]
    fn test_hash_merge_type_inference() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            my %base = (a => 1, b => 2);
            my %extended = (%base, c => 3, d => 4);
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let _result = engine.infer(&ast);

        // Both hashes should have integer values
        if let Some(PerlType::Hash { value, .. }) = engine.get_type_at("base") {
            assert!(matches!(
                value.as_ref(),
                &PerlType::Scalar(ScalarType::Integer) | &PerlType::Any
            ));
        }

        if let Some(PerlType::Hash { value, .. }) = engine.get_type_at("extended") {
            assert!(matches!(
                value.as_ref(),
                &PerlType::Scalar(ScalarType::Integer) | &PerlType::Any
            ));
        }
    }

    #[test]
    fn test_function_return_type() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            sub get_length {
                my $str = shift;
                return length($str);
            }
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let _result = engine.infer(&ast);

        // Check that get_length returns an integer
        if let Some(PerlType::Subroutine { returns, .. }) =
            engine.global_env.get_subroutine("get_length")
        {
            assert_eq!(returns.len(), 1);
            assert_eq!(returns[0], PerlType::Scalar(ScalarType::Integer));
        }
    }

    #[test]
    fn test_type_based_completions() {
        let mut engine = TypeInferenceEngine::new();

        let code = r#"
            my @items = (1, 2, 3);
            my %config = (name => "test", value => 42);
        "#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let _result = engine.infer(&ast);

        let completion = TypeBasedCompletion::new(Arc::new(engine));

        // Get array completions
        let array_completions = completion.get_completions("items", "");
        assert!(array_completions.iter().any(|c| c.label == "push"));
        assert!(array_completions.iter().any(|c| c.label == "pop"));

        // Get hash completions
        let hash_completions = completion.get_completions("config", "");
        assert!(hash_completions.iter().any(|c| c.label == "keys"));
        assert!(hash_completions.iter().any(|c| c.label == "values"));
    }
}
