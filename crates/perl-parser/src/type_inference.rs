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
    Hash { key: Box<PerlType>, value: Box<PerlType> },
    /// Reference to another type
    Reference(Box<PerlType>),
    /// Subroutine type with parameter and return types
    Subroutine {
        params: Vec<PerlType>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarType {
    String,
    Integer,
    Float,
    Boolean,
    Undef,
    Mixed, // Can be any scalar type
}

/// Type constraint for type checking
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub expected: PerlType,
    pub actual: PerlType,
    pub location: TypeLocation,
    pub reason: String,
}

/// Location information for type errors
#[derive(Debug, Clone)]
pub struct TypeLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
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
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            subroutines: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: TypeEnvironment) -> Self {
        Self {
            variables: HashMap::new(),
            subroutines: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn set_variable(&mut self, name: String, ty: PerlType) {
        self.variables.insert(name, ty);
    }

    pub fn get_variable(&self, name: &str) -> Option<&PerlType> {
        self.variables.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.get_variable(name))
        })
    }

    pub fn set_subroutine(&mut self, name: String, ty: PerlType) {
        self.subroutines.insert(name, ty);
    }

    pub fn get_subroutine(&self, name: &str) -> Option<&PerlType> {
        self.subroutines.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.get_subroutine(name))
        })
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
        self.builtins.insert("length".to_string(), 
            Subroutine {
                params: vec![Scalar(String)],
                returns: vec![Scalar(Integer)],
            }
        );
        
        self.builtins.insert("substr".to_string(),
            Subroutine {
                params: vec![Scalar(String), Scalar(Integer), Scalar(Integer)],
                returns: vec![Scalar(String)],
            }
        );
        
        // Array functions
        self.builtins.insert("push".to_string(),
            Subroutine {
                params: vec![Array(Box::new(Any)), Any],
                returns: vec![Scalar(Integer)],
            }
        );
        
        self.builtins.insert("pop".to_string(),
            Subroutine {
                params: vec![Array(Box::new(Any))],
                returns: vec![Any],
            }
        );
        
        // Hash functions
        self.builtins.insert("keys".to_string(),
            Subroutine {
                params: vec![Hash { 
                    key: Box::new(Scalar(String)), 
                    value: Box::new(Any) 
                }],
                returns: vec![Array(Box::new(Scalar(String)))],
            }
        );
        
        // I/O functions
        self.builtins.insert("print".to_string(),
            Subroutine {
                params: vec![Any],
                returns: vec![Scalar(Boolean)],
            }
        );
        
        self.builtins.insert("open".to_string(),
            Subroutine {
                params: vec![Glob, Scalar(String), Scalar(String)],
                returns: vec![Scalar(Boolean)],
            }
        );
        
        // Reference functions
        self.builtins.insert("ref".to_string(),
            Subroutine {
                params: vec![Any],
                returns: vec![Scalar(String)],
            }
        );
        
        // Type checking functions
        self.builtins.insert("defined".to_string(),
            Subroutine {
                params: vec![Any],
                returns: vec![Scalar(Boolean)],
            }
        );
    }

    /// Infer types for an AST
    pub fn infer(&mut self, ast: &Node) -> Result<PerlType, Vec<TypeConstraint>> {
        let ty = self.infer_node(ast, &mut self.global_env.clone())?;
        
        // Check constraints
        if !self.constraints.is_empty() {
            let violations: Vec<_> = self.constraints.iter()
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
    fn infer_node(&mut self, node: &Node, env: &mut TypeEnvironment) -> Result<PerlType, Vec<TypeConstraint>> {
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
            
            NodeKind::Number { value } => {
                if value.contains('.') || value.contains('e') || value.contains('E') {
                    Ok(Scalar(Float))
                } else {
                    Ok(Scalar(Integer))
                }
            }
            
            NodeKind::String { .. } => Ok(Scalar(String)),
            
            NodeKind::Undef => Ok(Scalar(Undef)),
            
            NodeKind::Variable { name, .. } => {
                let base_name = name.trim_start_matches(['$', '@', '%', '*']);
                
                if name.starts_with('$') {
                    // Scalar variable
                    if let Some(ty) = env.get_variable(base_name) {
                        Ok(ty.clone())
                    } else {
                        // Infer as Any for unknown variables
                        Ok(Scalar(Mixed))
                    }
                } else if name.starts_with('@') {
                    // Array variable
                    Ok(Array(Box::new(Any)))
                } else if name.starts_with('%') {
                    // Hash variable
                    Ok(Hash {
                        key: Box::new(Scalar(String)),
                        value: Box::new(Any),
                    })
                } else if name.starts_with('*') {
                    // Glob variable
                    Ok(Glob)
                } else {
                    Ok(Any)
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
                let mut key_type = Scalar(String);
                let mut value_type = Any;
                
                // Infer types from pairs
                for (key, val) in pairs {
                    let key_ty = self.infer_node(key, env)?;
                    if let Scalar(_) = key_ty {
                        key_type = key_ty;
                    }
                    let val_ty = self.infer_node(val, env)?;
                    if value_type == Any {
                        value_type = val_ty;
                    } else if !self.types_compatible(&value_type, &val_ty) {
                        value_type = Any;
                    }
                }
                
                Ok(Hash {
                    key: Box::new(key_type),
                    value: Box::new(value_type),
                })
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
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | 
                    "eq" | "ne" | "lt" | "gt" | "le" | "ge" |
                    "<=>" | "cmp" => {
                        Ok(Scalar(Boolean))
                    }
                    
                    // Logical operators
                    "&&" | "||" | "and" | "or" | "xor" => {
                        Ok(Scalar(Boolean))
                    }
                    
                    // Assignment operators
                    "=" | "+=" | "-=" | "*=" | "/=" | ".=" => {
                        env.set_variable(self.extract_var_name(left), right_ty.clone());
                        Ok(right_ty)
                    }
                    
                    _ => Ok(Any)
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
                    _ => Ok(Any)
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
                let return_type = self.infer_node(body, &mut sub_env)?;
                
                let sub_type = Subroutine {
                    params: param_types,
                    returns: vec![return_type],
                };
                
                // Register subroutine in environment
                if let Some(sub_name) = name {
                    env.set_subroutine(sub_name.clone(), sub_type.clone());
                }
                
                Ok(sub_type)
            }
            
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let init_type = if let Some(init) = initializer {
                    self.infer_node(init, env)?
                } else {
                    Scalar(Undef)
                };
                
                // Register variable with inferred type
                if let NodeKind::Variable { name, .. } = &variable.kind {
                    let base_name = name.trim_start_matches(['$', '@', '%']);
                    env.set_variable(base_name.to_string(), init_type.clone());
                }
                
                Ok(init_type)
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
            
            _ => Ok(Any) // Default for unhandled nodes
        }
    }

    /// Parse a subroutine signature
    #[allow(dead_code)]
    fn parse_signature(&mut self, _sig: &Node, _env: &mut TypeEnvironment) -> Result<Vec<PerlType>, Vec<TypeConstraint>> {
        // Simplified signature parsing
        // In a full implementation, this would parse Perl 5.20+ signatures
        Ok(vec![PerlType::Any])
    }

    /// Extract variable name from a node
    fn extract_var_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                name.trim_start_matches(['$', '@', '%']).to_string()
            }
            _ => String::new()
        }
    }

    /// Extract function name from a node
    #[allow(dead_code)]
    fn extract_func_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Identifier { name } => name.clone(),
            _ => String::new()
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
            _ => t1 == t2
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
            _ => s1 == s2
        }
    }
    
    /// Get type information for a variable at a specific location
    pub fn get_type_at(&self, name: &str) -> Option<PerlType> {
        self.global_env.get_variable(name).cloned()
    }
    
    /// Get all type errors/warnings
    pub fn get_type_errors(&self) -> Vec<TypeConstraint> {
        self.constraints.iter()
            .filter(|c| !self.types_compatible(&c.expected, &c.actual))
            .cloned()
            .collect()
    }
}

/// Type-based code completion suggestions
pub struct TypeBasedCompletion {
    engine: Arc<TypeInferenceEngine>,
}

impl TypeBasedCompletion {
    pub fn new(engine: Arc<TypeInferenceEngine>) -> Self {
        Self { engine }
    }
    
    /// Get completions based on variable type
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

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub documentation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_scalar_type_inference() {
        let mut engine = TypeInferenceEngine::new();
        
        let code = r#"
            my $x = 42;
            my $y = "hello";
            my $z = 3.14;
        "#;
        
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
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
        let ast = parser.parse().unwrap();
        let _result = engine.infer(&ast);
        
        assert!(matches!(
            engine.get_type_at("numbers"),
            Some(PerlType::Array(_))
        ));
        assert!(matches!(
            engine.get_type_at("strings"),
            Some(PerlType::Array(_))
        ));
        assert!(matches!(
            engine.get_type_at("mixed"),
            Some(PerlType::Array(_))
        ));
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
        let ast = parser.parse().unwrap();
        let _result = engine.infer(&ast);
        
        // Check that get_length returns an integer
        if let Some(PerlType::Subroutine { returns, .. }) = engine.global_env.get_subroutine("get_length") {
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
        let ast = parser.parse().unwrap();
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