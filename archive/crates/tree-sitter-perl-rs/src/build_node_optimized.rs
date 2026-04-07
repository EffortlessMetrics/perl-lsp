/// Example optimized build_node implementation
/// Shows key optimization techniques to close the performance gap

use std::sync::Arc;
use pest::iterators::Pair;

// Example of optimized build_node with reduced allocations
impl PureRustPerlParser {
    #[inline]
    pub(crate) fn build_node_optimized(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        match pair.as_rule() {
            Rule::program => {
                // Pre-allocate with typical program size
                let mut statements = Vec::with_capacity(32);
                for inner in pair.into_inner() {
                    if let Some(node) = self.build_node_optimized(inner)? {
                        statements.push(node);
                    }
                }
                Ok(Some(AstNode::Program(statements)))
            }
            
            // Fast path for simple variable assignment
            Rule::simple_assignment => {
                let mut inner = pair.into_inner();
                let var = inner.next().ok_or("missing assignment target")?;
                let value = inner.next().ok_or("missing assignment value")?;
                
                // Build without intermediate allocations
                let var_node = self.build_node_optimized(var)?.ok_or("invalid assignment target")?;
                let value_node = self.build_node_optimized(value)?.ok_or("invalid assignment value")?;
                
                Ok(Some(AstNode::BinaryOp {
                    op: "=".into(), // Use static string
                    left: Box::new(var_node),
                    right: Box::new(value_node),
                }))
            }
            
            // Variables - avoid string allocation
            Rule::scalar_variable => {
                // Option 1: Use Arc<str> for shared ownership
                let name: Arc<str> = Arc::from(pair.as_str());
                Ok(Some(AstNode::ScalarVariable(name)))
                
                // Option 2: Store as &'static str if possible
                // Ok(Some(AstNode::ScalarVariableRef(pair.as_str())))
            }
            
            Rule::array_variable => {
                let name: Arc<str> = Arc::from(pair.as_str());
                Ok(Some(AstNode::ArrayVariable(name)))
            }
            
            Rule::hash_variable => {
                let name: Arc<str> = Arc::from(pair.as_str());
                Ok(Some(AstNode::HashVariable(name)))
            }
            
            // Numbers - consider parsing immediately
            Rule::number => {
                // Option 1: Store as string
                let num: Arc<str> = Arc::from(pair.as_str());
                Ok(Some(AstNode::Number(num)))
                
                // Option 2: Parse and store as numeric type
                // let value: f64 = pair.as_str().parse()?;
                // Ok(Some(AstNode::NumericLiteral(value)))
            }
            
            // Binary operations - build chain efficiently
            Rule::additive_expression => {
                self.build_binary_chain_optimized(pair)
            }
            
            Rule::multiplicative_expression => {
                self.build_binary_chain_optimized(pair)
            }
            
            // Control flow - minimize allocations
            Rule::if_statement => {
                let mut inner = pair.into_inner();
                
                // Process without cloning pairs
                let condition = {
                    let cond_pair = inner.next().ok_or("missing if condition")?;
                    Box::new(self.build_node_optimized(cond_pair)?.ok_or("invalid if condition")?)
                };
                
                let then_block = {
                    let block_pair = inner.next().ok_or("missing then block")?;
                    Box::new(self.build_node_optimized(block_pair)?.ok_or("invalid then block")?)
                };
                
                // Pre-allocate for elsif clauses
                let mut elsif_clauses = Vec::with_capacity(2);
                let mut else_block = None;
                
                for remaining in inner {
                    match remaining.as_rule() {
                        Rule::elsif_clause => {
                            let mut elsif_inner = remaining.into_inner();
                            let cond = self.build_node_optimized(elsif_inner.next().ok_or("missing elsif condition")?)?.ok_or("invalid elsif condition")?;
                            let block = self.build_node_optimized(elsif_inner.next().ok_or("missing elsif block")?)?.ok_or("invalid elsif block")?;
                            elsif_clauses.push((cond, block));
                        }
                        Rule::else_clause => {
                            let mut else_inner = remaining.into_inner();
                            else_block = Some(Box::new(
                                self.build_node_optimized(else_inner.next().ok_or("missing else block")?)?.ok_or("invalid else block")?
                            ));
                        }
                        _ => {}
                    }
                }
                
                Ok(Some(AstNode::IfStatement {
                    condition,
                    then_block,
                    elsif_clauses,
                    else_block,
                }))
            }
            
            // Handle other common cases with similar optimizations
            _ => {
                // Fallback to original implementation
                // ...
                Ok(None)
            }
        }
    }
    
    #[inline]
    fn build_binary_chain_optimized(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        let mut inner = pair.into_inner();
        
        // Build first operand
        let first = inner.next().ok_or("missing binary chain operand")?;
        let mut result = self.build_node_optimized(first)?.ok_or("invalid binary chain operand")?;
        
        // Process remaining operator-operand pairs without collecting
        while let Some(op_pair) = inner.next() {
            if let Some(operand_pair) = inner.next() {
                // Use static strings for common operators
                let op = match op_pair.as_str() {
                    "+" => "+",
                    "-" => "-", 
                    "*" => "*",
                    "/" => "/",
                    "." => ".",
                    other => other,
                };
                
                let right = self.build_node_optimized(operand_pair)?.ok_or("invalid binary chain right operand")?;
                result = AstNode::BinaryOp {
                    op: op.into(),
                    left: Box::new(result),
                    right: Box::new(right),
                };
            }
        }
        
        Ok(Some(result))
    }
}

/// Additional optimizations for the AST enum
#[derive(Debug, Clone)]
pub enum AstNodeOptimized {
    // Use Arc<str> instead of String
    ScalarVariable(Arc<str>),
    ArrayVariable(Arc<str>),
    HashVariable(Arc<str>),
    TypeglobVariable(Arc<str>),
    
    // Store numbers as parsed values when possible
    Number(Arc<str>),  // Or use: NumericLiteral(f64),
    
    // Use &'static str for operators
    BinaryOp {
        op: &'static str,  // Most operators are known at compile time
        left: Box<AstNodeOptimized>,
        right: Box<AstNodeOptimized>,
    },
    
    // Other variants...
}