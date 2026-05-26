use super::class_model::{AccessorType, ClassModelBuilder};
use super::type_facts::{
    ArrayShape, DynamicBoundary, HashShape, ObjectShape, ShapeFact, TypeEvidence, TypeFact,
};
use crate::ast::{Node, NodeKind};
use perl_semantic_facts::Confidence;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
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

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScalarType::String => write!(f, "Str"),
            ScalarType::Integer => write!(f, "Int"),
            ScalarType::Float => write!(f, "Float"),
            ScalarType::Boolean => write!(f, "Bool"),
            ScalarType::Undef => write!(f, "Undef"),
            ScalarType::Mixed => write!(f, "Scalar"),
        }
    }
}

impl fmt::Display for PerlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerlType::Scalar(s) => write!(f, "{s}"),
            PerlType::Array(elem) => {
                if matches!(elem.as_ref(), PerlType::Any) {
                    write!(f, "Array")
                } else {
                    write!(f, "Array[{elem}]")
                }
            }
            PerlType::Hash { key, value } => {
                if matches!(value.as_ref(), PerlType::Any) {
                    write!(f, "Hash")
                } else {
                    write!(f, "Hash[{key} => {value}]")
                }
            }
            PerlType::Reference(inner) => write!(f, "Ref[{inner}]"),
            PerlType::Subroutine { .. } => write!(f, "Sub"),
            PerlType::Object(class) => write!(f, "{class}"),
            PerlType::Glob => write!(f, "Glob"),
            PerlType::Union(types) => {
                let labels: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", labels.join("|"))
            }
            PerlType::Any => write!(f, "Any"),
            PerlType::Void => write!(f, "Void"),
        }
    }
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
    /// Rich variable facts in current scope
    variable_facts: HashMap<String, TypeFact>,
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
        Self {
            variables: HashMap::new(),
            variable_facts: HashMap::new(),
            subroutines: HashMap::new(),
            parent: None,
        }
    }

    /// Creates a new type environment with a parent scope
    pub fn with_parent(parent: TypeEnvironment) -> Self {
        Self {
            variables: HashMap::new(),
            variable_facts: HashMap::new(),
            subroutines: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Sets the type for a variable in the current scope
    pub fn set_variable(&mut self, name: String, ty: PerlType) {
        self.variable_facts.remove(&name);
        self.variables.insert(name, ty);
    }

    /// Sets the rich fact for a variable in the current scope.
    ///
    /// The erased type is also stored so existing type-only APIs continue to
    /// observe the same variable.
    pub fn set_variable_fact(&mut self, name: String, fact: TypeFact) {
        self.variables.insert(name.clone(), fact.erased_type());
        self.variable_facts.insert(name, fact);
    }

    /// Gets the type of a variable, searching parent scopes if needed
    pub fn get_variable(&self, name: &str) -> Option<&PerlType> {
        self.variables.get(name).or_else(|| self.parent.as_ref().and_then(|p| p.get_variable(name)))
    }

    /// Gets the rich fact for a variable, searching parent scopes if needed.
    pub fn get_variable_fact(&self, name: &str) -> Option<&TypeFact> {
        self.variable_facts
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_variable_fact(name)))
    }

    /// Gets the rich fact for a variable by name, searching parent scopes if needed.
    pub fn get_fact_at(&self, name: &str) -> Option<TypeFact> {
        self.get_variable_fact(name).cloned()
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
    /// Framework accessor return facts keyed by `(package, accessor)`.
    accessor_return_facts: HashMap<(String, String), TypeFact>,
    /// Narrow source-backed method return facts keyed by `(package, method)`.
    method_return_facts: HashMap<(String, String), TypeFact>,
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
            accessor_return_facts: HashMap::new(),
            method_return_facts: HashMap::new(),
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
        self.refresh_accessor_return_facts(ast);
        self.refresh_method_return_facts(ast);

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

    /// Infer a rich type fact for an expression using the engine's current
    /// global environment.
    pub fn infer_expr_fact(&mut self, expr: &Node) -> TypeFact {
        let mut env = std::mem::take(&mut self.global_env);
        let fact = self.infer_expr_fact_in_env(expr, &mut env);
        self.global_env = env;
        fact
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

            NodeKind::Assignment { lhs, rhs, op } => {
                if op == "=" {
                    let fact = self.assign_expr_fact(lhs, rhs, env);
                    Ok(fact.erased_type())
                } else {
                    let rhs_ty = self.infer_node(rhs, env)?;
                    Ok(rhs_ty)
                }
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
                        if op == "=" {
                            let fact = self.assign_expr_fact(left, right, env);
                            return Ok(fact.erased_type());
                        }
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

                    let fact = self.declared_variable_fact(sigil, clean_name, initializer, env);
                    let inferred_type = fact.erased_type();
                    env.set_variable_fact(clean_name.to_string(), fact);

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

            NodeKind::MethodCall { object, method, .. } => {
                // Detect ClassName->new() pattern and return Object("ClassName")
                if method == "new" {
                    if let NodeKind::Identifier { name } = &object.kind {
                        return Ok(Object(name.clone()));
                    }
                }
                Ok(Any)
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

    fn infer_expr_fact_in_env(&mut self, node: &Node, env: &mut TypeEnvironment) -> TypeFact {
        use PerlType::*;
        use ScalarType::*;

        match &node.kind {
            NodeKind::ExpressionStatement { expression } => {
                self.infer_expr_fact_in_env(expression, env)
            }
            NodeKind::Number { value } => {
                let ty = if value.contains('.') || value.contains('e') || value.contains('E') {
                    Scalar(Float)
                } else {
                    Scalar(Integer)
                };
                fact_with_evidence(ty, Confidence::High, TypeEvidence::Literal)
            }
            NodeKind::String { .. } => {
                fact_with_evidence(Scalar(String), Confidence::High, TypeEvidence::Literal)
            }
            NodeKind::Undef => {
                fact_with_evidence(Scalar(Undef), Confidence::High, TypeEvidence::Literal)
            }
            NodeKind::Variable { sigil, name } => env.get_fact_at(name).unwrap_or_else(|| {
                let ty = env.get_variable(name).cloned().unwrap_or_else(|| match sigil.as_str() {
                    "$" => Scalar(Mixed),
                    "@" => Array(Box::new(Any)),
                    "%" => Hash { key: Box::new(Scalar(String)), value: Box::new(Any) },
                    "*" => Glob,
                    _ => Any,
                });
                TypeFact::new(ty, Confidence::Low)
            }),
            NodeKind::VariableWithAttributes { variable, .. } => {
                self.infer_expr_fact_in_env(variable, env)
            }
            NodeKind::ArrayLiteral { elements } => self.array_literal_fact(elements, env),
            NodeKind::HashLiteral { pairs } => self.hash_literal_fact(pairs, env),
            NodeKind::FunctionCall { name, args } if name == "bless" => {
                self.bless_expr_fact(args, env)
            }
            NodeKind::MethodCall { object, method, .. } if method == "new" => {
                static_package_node(object)
                    .map_or_else(TypeFact::unknown, |package| constructor_fact(&package))
            }
            NodeKind::MethodCall { object, method, .. } => {
                self.method_call_expr_fact(object, method, env)
            }
            NodeKind::Assignment { lhs, rhs, op } if op == "=" => {
                self.assign_expr_fact(lhs, rhs, env)
            }
            NodeKind::Binary { left, op, right } if op == "=" => {
                self.assign_expr_fact(left, right, env)
            }
            NodeKind::Binary { left, op, right } if op == "{}" || op == "->{}" => {
                self.hash_slot_expr_fact(op, left, right, env)
            }
            NodeKind::Binary { left, op, right } if op == "[]" || op == "->[]" => {
                self.array_index_expr_fact(left, right, env)
            }
            _ => self
                .infer_node(node, env)
                .map_or_else(|_| TypeFact::unknown(), |ty| TypeFact::new(ty, Confidence::Low)),
        }
    }

    fn declared_variable_fact(
        &mut self,
        sigil: &str,
        name: &str,
        initializer: &Option<Box<Node>>,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        let mut fact = match (sigil, initializer) {
            ("%", Some(init)) => self.hash_initializer_fact(init, env),
            ("%", None) => TypeFact::unknown_hash(),
            ("@", Some(init)) => self.infer_expr_fact_in_env(init, env),
            ("@", None) => TypeFact::new(PerlType::Array(Box::new(PerlType::Any)), Confidence::Low),
            ("$", Some(init)) => self.infer_expr_fact_in_env(init, env),
            ("$", None) => TypeFact::new(PerlType::Scalar(ScalarType::Undef), Confidence::Low),
            _ => TypeFact::unknown(),
        };
        fact.evidence.push(TypeEvidence::VariableInitializer { name: name.to_string() });
        fact
    }

    fn refresh_accessor_return_facts(&mut self, ast: &Node) {
        self.accessor_return_facts.clear();

        for model in ClassModelBuilder::new().build(ast) {
            for attr in model.attributes {
                if !attribute_generates_reader(attr.is) {
                    continue;
                }
                let Some(package) = package_like_isa(attr.isa.as_deref()) else {
                    continue;
                };
                let method = attr.accessor_name.clone();
                let fact = accessor_return_fact(&method, &attr.name, &package);
                self.accessor_return_facts.insert((model.name.clone(), method), fact);
            }
        }
    }

    fn refresh_method_return_facts(&mut self, ast: &Node) {
        self.method_return_facts.clear();
        collect_method_return_facts(
            ast,
            None,
            &self.accessor_return_facts,
            &mut self.method_return_facts,
        );
    }

    fn method_call_expr_fact(
        &mut self,
        object: &Node,
        method: &str,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        let object_fact = self.infer_expr_fact_in_env(object, env);
        let Some(package) = object_package_from_fact(&object_fact) else {
            let mut fact = TypeFact::unknown();
            fact.dynamic_boundary = object_fact.dynamic_boundary;
            return fact;
        };

        let key = (package, method.to_string());
        self.accessor_return_facts
            .get(&key)
            .cloned()
            .or_else(|| self.method_return_facts.get(&key).cloned())
            .unwrap_or_else(TypeFact::unknown)
    }

    fn assign_expr_fact(&mut self, lhs: &Node, rhs: &Node, env: &mut TypeEnvironment) -> TypeFact {
        let mut rhs_fact = self.infer_expr_fact_in_env(rhs, env);

        if let Some(name) = variable_name(lhs) {
            rhs_fact.evidence.push(TypeEvidence::Assignment { name: name.to_string() });
            env.set_variable_fact(name.to_string(), rhs_fact.clone());
            return rhs_fact;
        }

        if let Some((hash_name, key, is_hashref_slot)) = static_hash_slot_target(lhs) {
            rhs_fact.evidence.push(TypeEvidence::Assignment { name: hash_name.clone() });
            let slot_evidence = if is_hashref_slot {
                TypeEvidence::HashRefSlot { base: format!("${hash_name}"), key: key.clone() }
            } else {
                TypeEvidence::HashSlot { hash: format!("${hash_name}"), key: key.clone() }
            };
            rhs_fact.evidence.push(slot_evidence);
            let mut hash_fact = env.get_fact_at(&hash_name).unwrap_or_else(TypeFact::unknown_hash);
            let mut shape = match hash_fact.shape.take() {
                Some(ShapeFact::Hash(shape)) => shape,
                _ => HashShape::new(BTreeMap::new(), None),
            };
            shape.slots.insert(key, rhs_fact.clone());
            hash_fact.ty = hash_type_from_slot_facts(shape.slots.values());
            hash_fact.confidence = Confidence::High;
            hash_fact.shape = Some(ShapeFact::Hash(shape));
            env.set_variable_fact(hash_name, hash_fact);
            return rhs_fact;
        }

        rhs_fact
    }

    fn hash_initializer_fact(&mut self, init: &Node, env: &mut TypeEnvironment) -> TypeFact {
        match &init.kind {
            NodeKind::ArrayLiteral { elements } => self.hash_literal_elements_fact(elements, env),
            NodeKind::HashLiteral { pairs } => self.hash_literal_fact(pairs, env),
            _ => self.infer_expr_fact_in_env(init, env),
        }
    }

    fn hash_literal_fact(&mut self, pairs: &[(Node, Node)], env: &mut TypeEnvironment) -> TypeFact {
        let mut slots = BTreeMap::new();
        for (key, value) in pairs {
            if let Some(key) = static_hash_key(key) {
                let mut value_fact = self.infer_expr_fact_in_env(value, env);
                value_fact
                    .evidence
                    .push(TypeEvidence::HashSlot { hash: "literal".to_string(), key: key.clone() });
                slots.insert(key, value_fact);
            }
        }

        hash_shape_fact(slots, TypeEvidence::Literal)
    }

    fn hash_literal_elements_fact(
        &mut self,
        elements: &[Node],
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        if !elements.len().is_multiple_of(2) {
            return hash_shape_fact(BTreeMap::new(), TypeEvidence::Literal);
        }

        let mut slots = BTreeMap::new();
        for pair in elements.chunks(2) {
            let [key, value] = pair else {
                continue;
            };
            let Some(key) = static_hash_key(key) else {
                return hash_shape_fact(BTreeMap::new(), TypeEvidence::Literal);
            };
            let mut value_fact = self.infer_expr_fact_in_env(value, env);
            value_fact
                .evidence
                .push(TypeEvidence::HashSlot { hash: "literal".to_string(), key: key.clone() });
            slots.insert(key, value_fact);
        }

        hash_shape_fact(slots, TypeEvidence::Literal)
    }

    fn bless_expr_fact(&mut self, args: &[Node], env: &mut TypeEnvironment) -> TypeFact {
        let Some(reference) = args.first() else {
            return TypeFact::unknown();
        };
        let Some(package_node) = args.get(1) else {
            return TypeFact::dynamic(DynamicBoundary::DynamicBlessClass);
        };
        let Some(package) = static_package_node(package_node) else {
            return TypeFact::dynamic(DynamicBoundary::DynamicBlessClass);
        };

        let reference_fact = self.infer_expr_fact_in_env(reference, env);
        let fields = object_fields_from_bless_reference(reference_fact, &package);
        let mut fact = fact_with_evidence(
            PerlType::Object(package.clone()),
            Confidence::Medium,
            TypeEvidence::BlessLiteral { package: package.clone() },
        );
        fact.shape = Some(ShapeFact::Object(ObjectShape::new(package, fields)));
        fact
    }

    fn array_literal_fact(&mut self, elements: &[Node], env: &mut TypeEnvironment) -> TypeFact {
        let mut indexed = BTreeMap::new();
        for (index, element) in elements.iter().enumerate() {
            indexed.insert(index, self.infer_expr_fact_in_env(element, env));
        }
        let ty = PerlType::Array(Box::new(hash_value_type(indexed.values())));
        let mut fact = fact_with_evidence(ty, Confidence::High, TypeEvidence::Literal);
        fact.shape = Some(ShapeFact::Array(ArrayShape::new(indexed, None)));
        fact
    }

    fn hash_slot_expr_fact(
        &mut self,
        op: &str,
        left: &Node,
        right: &Node,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        let Some(key) = static_hash_key(right) else {
            return TypeFact::dynamic(DynamicBoundary::DynamicHashKey);
        };

        let container_fact = self.infer_expr_fact_in_env(left, env);
        let evidence = if op == "->{}" {
            TypeEvidence::HashRefSlot { base: receiver_base_label(left), key: key.clone() }
        } else {
            TypeEvidence::HashSlot { hash: receiver_base_label(left), key: key.clone() }
        };

        match container_fact.shape {
            Some(ShapeFact::Hash(shape)) => shape
                .slots
                .get(&key)
                .cloned()
                .or_else(|| shape.fallback_value.as_ref().map(|fact| fact.as_ref().clone()))
                .map_or_else(
                    || {
                        let mut fact = TypeFact::unknown();
                        fact.evidence.push(evidence.clone());
                        fact
                    },
                    |mut fact| {
                        fact.evidence.push(evidence.clone());
                        fact
                    },
                ),
            Some(ShapeFact::Object(shape)) if op == "->{}" => {
                shape.fields.get(&key).cloned().map_or_else(
                    || {
                        let mut fact = TypeFact::unknown();
                        fact.evidence.push(evidence.clone());
                        fact
                    },
                    |mut fact| {
                        fact.evidence.push(evidence.clone());
                        fact
                    },
                )
            }
            _ => {
                let mut fact = TypeFact::unknown();
                fact.dynamic_boundary = container_fact.dynamic_boundary;
                fact.evidence.push(evidence);
                fact
            }
        }
    }

    fn array_index_expr_fact(
        &mut self,
        left: &Node,
        right: &Node,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        let Some(index) = static_array_index(right) else {
            return TypeFact::dynamic(DynamicBoundary::UnknownReceiver);
        };

        match self.infer_expr_fact_in_env(left, env).shape {
            Some(ShapeFact::Array(shape)) => shape
                .indexed
                .get(&index)
                .cloned()
                .or_else(|| shape.element.as_ref().map(|fact| fact.as_ref().clone()))
                .unwrap_or_else(TypeFact::unknown),
            _ => TypeFact::unknown(),
        }
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

    /// Gets the inferred rich fact for a variable by name.
    pub fn get_fact_at(&self, name: &str) -> Option<TypeFact> {
        self.global_env.get_fact_at(name)
    }

    /// Returns the current global type environment for provider-level
    /// source-backed fact lookups.
    pub fn environment(&self) -> &TypeEnvironment {
        &self.global_env
    }

    /// Returns a human-readable type label for use in hover text.
    ///
    /// Returns `None` if the variable has not been tracked by the engine.
    pub fn hover_label_for(&self, name: &str) -> Option<String> {
        self.global_env.get_variable(name).map(|ty| ty.to_string())
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

fn fact_with_evidence(ty: PerlType, confidence: Confidence, evidence: TypeEvidence) -> TypeFact {
    let mut fact = TypeFact::new(ty, confidence);
    fact.evidence.push(evidence);
    fact
}

fn constructor_fact(package: &str) -> TypeFact {
    let mut fact = fact_with_evidence(
        PerlType::Object(package.to_string()),
        Confidence::High,
        TypeEvidence::ConstructorCall { package: package.to_string() },
    );
    fact.shape = Some(ShapeFact::Object(ObjectShape::new(package.to_string(), BTreeMap::new())));
    fact
}

fn accessor_return_fact(method: &str, field: &str, package: &str) -> TypeFact {
    let mut fact = fact_with_evidence(
        PerlType::Any,
        Confidence::Medium,
        TypeEvidence::MooseIsa { attr: field.to_string(), isa: package.to_string() },
    );
    fact.evidence.push(TypeEvidence::AccessorReturn {
        method: method.to_string(),
        field: field.to_string(),
    });
    fact.shape = Some(ShapeFact::Object(ObjectShape::new(package.to_string(), BTreeMap::new())));
    fact
}

fn method_return_fact(method: &str, package: &str) -> TypeFact {
    method_return_fact_with_evidence(
        method,
        package.to_string(),
        vec![TypeEvidence::ConstructorCall { package: package.to_string() }],
    )
}

fn method_return_fact_with_evidence(
    method: &str,
    package: String,
    evidence: Vec<TypeEvidence>,
) -> TypeFact {
    let mut fact = fact_with_evidence(
        PerlType::Any,
        Confidence::Medium,
        TypeEvidence::MethodReturn { method: method.to_string(), package: package.clone() },
    );
    fact.evidence.extend(evidence);
    fact.shape = Some(ShapeFact::Object(ObjectShape::new(package, BTreeMap::new())));
    fact
}

fn collect_method_return_facts(
    node: &Node,
    package: Option<&str>,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
    out: &mut HashMap<(String, String), TypeFact>,
) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            collect_method_return_facts_from_statements(
                statements,
                package,
                accessor_return_facts,
                out,
            );
        }
        NodeKind::Package { name, block: Some(block), .. } => {
            collect_method_return_facts(block, Some(name.as_str()), accessor_return_facts, out);
        }
        NodeKind::Subroutine { name: Some(method), body, .. } => {
            if let (Some(package), Some(fact)) =
                (package, static_method_body_return_fact(method, body, accessor_return_facts))
            {
                out.insert((package.to_string(), method.clone()), fact);
            }
        }
        NodeKind::Method { name, body, .. } => {
            if let (Some(package), Some(fact)) =
                (package, static_method_body_return_fact(name, body, accessor_return_facts))
            {
                out.insert((package.to_string(), name.clone()), fact);
            }
        }
        _ => {
            for child in node.children() {
                collect_method_return_facts(child, package, accessor_return_facts, out);
            }
        }
    }
}

fn collect_method_return_facts_from_statements(
    statements: &[Node],
    package: Option<&str>,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
    out: &mut HashMap<(String, String), TypeFact>,
) {
    let mut current_package = package.map(ToOwned::to_owned);
    for statement in statements {
        match &statement.kind {
            NodeKind::Package { name, block: None, .. } => {
                current_package = Some(name.clone());
            }
            NodeKind::Package { name, block: Some(block), .. } => {
                collect_method_return_facts(block, Some(name.as_str()), accessor_return_facts, out);
            }
            _ => {
                collect_method_return_facts(
                    statement,
                    current_package.as_deref(),
                    accessor_return_facts,
                    out,
                );
            }
        }
    }
}

fn static_method_body_return_fact(
    method: &str,
    body: &Node,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<TypeFact> {
    let NodeKind::Block { statements } = &body.kind else {
        return None;
    };
    if let [statement] = statements.as_slice() {
        return static_method_return_expr_fact(method, statement, accessor_return_facts);
    }
    static_method_local_return_fact(method, statements, accessor_return_facts)
}

fn static_method_return_expr_fact(
    method: &str,
    node: &Node,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<TypeFact> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            static_method_return_expr_fact(method, expression, accessor_return_facts)
        }
        NodeKind::Return { value: Some(value) } => {
            static_method_return_expr_fact(method, value, accessor_return_facts)
        }
        _ => static_constructor_package(node)
            .map(|package| method_return_fact(method, &package))
            .or_else(|| accessor_chain_method_return_fact(method, node, accessor_return_facts)),
    }
}

fn accessor_chain_method_return_fact(
    method: &str,
    node: &Node,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<TypeFact> {
    let (package, evidence) = accessor_chain_return_package(node, accessor_return_facts)?;
    Some(method_return_fact_with_evidence(method, package, evidence))
}

fn accessor_chain_return_package(
    node: &Node,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<(String, Vec<TypeEvidence>)> {
    let NodeKind::MethodCall { object, method, .. } = &node.kind else {
        return None;
    };
    if method == "new" {
        return None;
    }

    let package = static_constructor_package(object)?;
    let mut fact = accessor_return_facts.get(&(package.clone(), method.clone()))?.clone();
    let returned_package = object_package_from_fact(&fact)?;
    let mut evidence = vec![TypeEvidence::ConstructorCall { package }];
    evidence.append(&mut fact.evidence);
    Some((returned_package, evidence))
}

fn static_method_local_return_fact(
    method: &str,
    statements: &[Node],
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<TypeFact> {
    let returned_name = returned_variable_name(statements.last()?)?;
    let mut returned_local_declared = false;
    let mut returned_package = None;

    for statement in &statements[..statements.len().saturating_sub(1)] {
        match local_return_declaration_package(statement, returned_name, accessor_return_facts) {
            Some(LocalReturnDeclaration::Lexical(candidate)) => {
                returned_local_declared = true;
                returned_package = candidate.map(LocalReturnPackage::Initializer);
                continue;
            }
            Some(LocalReturnDeclaration::NonLexical) => {
                return None;
            }
            None => {}
        }

        match local_return_assignment_package(statement, returned_name, accessor_return_facts) {
            Some(Some(candidate)) if returned_local_declared => {
                returned_package = Some(LocalReturnPackage::Assignment(candidate));
            }
            Some(_) => {
                return None;
            }
            None if local_return_statement_blocks_static_fact(statement, returned_name) => {
                return None;
            }
            None => {}
        }
    }

    returned_package.map(|returned_package| {
        let returned_fact = returned_package.into_parts(returned_name);
        method_return_fact_with_evidence(method, returned_fact.package, returned_fact.evidence)
    })
}

struct LocalReturnFact {
    package: String,
    evidence: Vec<TypeEvidence>,
}

impl LocalReturnFact {
    fn new(package: String, evidence: Vec<TypeEvidence>) -> Self {
        Self { package, evidence }
    }

    fn with_evidence(mut self, evidence: TypeEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }
}

enum LocalReturnDeclaration {
    Lexical(Option<LocalReturnFact>),
    NonLexical,
}

enum LocalReturnPackage {
    Initializer(LocalReturnFact),
    Assignment(LocalReturnFact),
}

impl LocalReturnPackage {
    fn into_parts(self, returned_name: &str) -> LocalReturnFact {
        match self {
            Self::Initializer(fact) => fact.with_evidence(TypeEvidence::VariableInitializer {
                name: returned_name.to_string(),
            }),
            Self::Assignment(fact) => {
                fact.with_evidence(TypeEvidence::Assignment { name: returned_name.to_string() })
            }
        }
    }
}

fn returned_variable_name(node: &Node) -> Option<&str> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => returned_variable_name(expression),
        NodeKind::Return { value: Some(value) } => variable_name(value),
        _ => variable_name(node),
    }
}

fn local_return_declaration_package(
    node: &Node,
    returned_name: &str,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<LocalReturnDeclaration> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            local_return_declaration_package(expression, returned_name, accessor_return_facts)
        }
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. }
            if variable_name(variable) == Some(returned_name) =>
        {
            if is_lexical_return_declarator(declarator) {
                Some(LocalReturnDeclaration::Lexical(
                    initializer.as_deref().and_then(|node| {
                        static_local_return_source_fact(node, accessor_return_facts)
                    }),
                ))
            } else {
                Some(LocalReturnDeclaration::NonLexical)
            }
        }
        _ => None,
    }
}

fn is_lexical_return_declarator(declarator: &str) -> bool {
    matches!(declarator, "my" | "state")
}

fn local_return_assignment_package(
    node: &Node,
    returned_name: &str,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<Option<LocalReturnFact>> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            local_return_assignment_package(expression, returned_name, accessor_return_facts)
        }
        NodeKind::Assignment { lhs, rhs, op }
            if op == "=" && variable_name(lhs) == Some(returned_name) =>
        {
            Some(static_local_return_source_fact(rhs, accessor_return_facts))
        }
        NodeKind::Binary { left, op, right }
            if op == "=" && variable_name(left) == Some(returned_name) =>
        {
            Some(static_local_return_source_fact(right, accessor_return_facts))
        }
        _ => None,
    }
}

fn static_local_return_source_fact(
    node: &Node,
    accessor_return_facts: &HashMap<(String, String), TypeFact>,
) -> Option<LocalReturnFact> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            static_local_return_source_fact(expression, accessor_return_facts)
        }
        _ => static_constructor_package(node)
            .map(|package| {
                LocalReturnFact::new(
                    package.clone(),
                    vec![TypeEvidence::ConstructorCall { package }],
                )
            })
            .or_else(|| {
                accessor_chain_return_package(node, accessor_return_facts)
                    .map(|(package, evidence)| LocalReturnFact::new(package, evidence))
            }),
    }
}

fn local_return_statement_blocks_static_fact(node: &Node, returned_name: &str) -> bool {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            local_return_statement_blocks_static_fact(expression, returned_name)
        }
        NodeKind::Assignment { lhs, .. } if variable_name(lhs) == Some(returned_name) => true,
        NodeKind::Binary { left, .. } if variable_name(left) == Some(returned_name) => true,
        NodeKind::Block { .. }
        | NodeKind::Eval { .. }
        | NodeKind::Do { .. }
        | NodeKind::Defer { .. }
        | NodeKind::Try { .. }
        | NodeKind::If { .. }
        | NodeKind::While { .. }
        | NodeKind::For { .. }
        | NodeKind::Foreach { .. }
        | NodeKind::Given { .. }
        | NodeKind::When { .. }
        | NodeKind::Default { .. }
        | NodeKind::StatementModifier { .. }
        | NodeKind::Return { .. }
        | NodeKind::LoopControl { .. }
        | NodeKind::Goto { .. } => true,
        _ => node_mentions_variable(node, returned_name),
    }
}

fn node_mentions_variable(node: &Node, name: &str) -> bool {
    if variable_name(node) == Some(name) {
        return true;
    }
    let mut found = false;
    node.for_each_child(|child| {
        if !found && node_mentions_variable(child, name) {
            found = true;
        }
    });
    found
}

fn attribute_generates_reader(mode: Option<AccessorType>) -> bool {
    !matches!(mode, Some(AccessorType::Bare))
}

fn package_like_isa(isa: Option<&str>) -> Option<String> {
    let candidate = isa?.trim();
    if is_simple_package_name(candidate) { Some(candidate.to_string()) } else { None }
}

fn is_simple_package_name(candidate: &str) -> bool {
    let mut segments = candidate.split("::");
    let Some(first) = segments.next() else { return false };
    if !is_package_segment(first) {
        return false;
    }

    let mut has_namespace = false;
    for segment in segments {
        has_namespace = true;
        if !is_package_segment(segment) {
            return false;
        }
    }
    has_namespace
}

fn is_package_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else { return false };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn object_package_from_fact(fact: &TypeFact) -> Option<String> {
    object_package_from_type(&fact.ty).or_else(|| match &fact.shape {
        Some(ShapeFact::Object(shape)) => Some(shape.package.clone()),
        _ => None,
    })
}

fn object_package_from_type(ty: &PerlType) -> Option<String> {
    match ty {
        PerlType::Object(package) => Some(package.clone()),
        PerlType::Reference(inner) => object_package_from_type(inner),
        PerlType::Union(types) => types.iter().find_map(object_package_from_type),
        _ => None,
    }
}

fn object_fields_from_bless_reference(
    reference_fact: TypeFact,
    package: &str,
) -> BTreeMap<String, TypeFact> {
    match reference_fact.shape {
        Some(ShapeFact::Hash(shape)) => shape
            .slots
            .into_iter()
            .map(|(name, fact)| (name, bless_field_fact(fact, package)))
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn bless_field_fact(mut fact: TypeFact, package: &str) -> TypeFact {
    if fact.confidence == Confidence::High {
        fact.confidence = Confidence::Medium;
    }
    fact.evidence.push(TypeEvidence::BlessLiteral { package: package.to_string() });
    fact
}

fn hash_shape_fact(slots: BTreeMap<String, TypeFact>, evidence: TypeEvidence) -> TypeFact {
    let confidence = if slots.is_empty() { Confidence::Low } else { Confidence::High };
    let mut fact =
        fact_with_evidence(hash_type_from_slot_facts(slots.values()), confidence, evidence);
    fact.shape = Some(ShapeFact::Hash(HashShape::new(slots, None)));
    fact
}

fn hash_type_from_slot_facts<'a, I>(facts: I) -> PerlType
where
    I: IntoIterator<Item = &'a TypeFact>,
{
    PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(hash_value_type(facts)),
    }
}

fn hash_value_type<'a, I>(facts: I) -> PerlType
where
    I: IntoIterator<Item = &'a TypeFact>,
{
    let mut values = facts.into_iter();
    let Some(first) = values.next() else {
        return PerlType::Any;
    };
    let first_ty = first.erased_type();
    if values.all(|fact| fact.erased_type() == first_ty) { first_ty } else { PerlType::Any }
}

fn static_package_node(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::String { value, .. } => normalize_literal(value),
        _ => None,
    }
}

fn static_constructor_package(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => static_constructor_package(expression),
        NodeKind::MethodCall { object, method, .. } if method == "new" => {
            static_package_node(object)
        }
        _ => None,
    }
}

fn variable_name(node: &Node) -> Option<&str> {
    match &node.kind {
        NodeKind::Variable { name, .. } => Some(name.as_str()),
        NodeKind::VariableWithAttributes { variable, .. } => variable_name(variable),
        _ => None,
    }
}

fn static_hash_slot_target(node: &Node) -> Option<(String, String, bool)> {
    let NodeKind::Binary { op, left, right } = &node.kind else {
        return None;
    };
    if op != "{}" && op != "->{}" {
        return None;
    }
    let name = variable_name(left)?;
    let key = static_hash_key(right)?;
    Some((name.to_string(), key, op == "->{}"))
}

fn static_hash_key(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, .. } => normalize_literal(value),
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}

fn static_array_index(node: &Node) -> Option<usize> {
    match &node.kind {
        NodeKind::Number { value } => value.parse().ok(),
        _ => None,
    }
}

fn receiver_base_label(node: &Node) -> String {
    match &node.kind {
        NodeKind::Variable { sigil, name } => format!("{sigil}{name}"),
        NodeKind::VariableWithAttributes { variable, .. } => receiver_base_label(variable),
        _ => node.kind.kind_name().to_string(),
    }
}

fn normalize_literal(value: &str) -> Option<String> {
    let normalized = value.trim().trim_matches('\'').trim_matches('"').trim();
    if normalized.is_empty() { None } else { Some(normalized.to_string()) }
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
