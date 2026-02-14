//! Iterative AST builder to avoid stack overflow
//!
//! This module provides an iterative implementation of the AST building process,
//! replacing recursion with an explicit stack to avoid stack overflow in debug builds.

use crate::pure_rust_parser::{AstNode, PureRustPerlParser, Rule};
use pest::iterators::Pair;
use std::sync::Arc;
use std::{error::Error, fmt};

/// State for iterative AST building
#[derive(Debug)]
enum BuildState<'a> {
    /// Process a new pair
    Process(Pair<'a, Rule>),
    /// Waiting for children to be processed
    WaitingForChildren {
        rule: Rule,
        processed_children: Vec<AstNode>,
        remaining_children: Vec<Pair<'a, Rule>>,
        original_str: &'a str,
    },
    /// Build node from processed children
    BuildFromChildren { rule: Rule, children: Vec<AstNode>, original_str: &'a str },
}

/// Result of processing a state
#[derive(Debug)]
enum ProcessResult<'a> {
    /// Node is complete
    Complete(Option<AstNode>),
    /// Need to process children first
    PushStates(Vec<BuildState<'a>>),
}

#[derive(Debug)]
enum IterativeParserError {
    UnexpectedWaitingForChildren,
}

impl fmt::Display for IterativeParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IterativeParserError::UnexpectedWaitingForChildren => {
                write!(f, "WaitingForChildren should be handled in main loop")
            }
        }
    }
}

impl Error for IterativeParserError {}

impl PureRustPerlParser {
    /// Iterative version of build_node that uses explicit stack instead of recursion
    pub fn build_node_iterative(
        &mut self,
        initial_pair: Pair<Rule>,
    ) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        let mut stack: Vec<BuildState> = vec![BuildState::Process(initial_pair)];
        let mut results: Vec<Option<AstNode>> = Vec::new();

        while let Some(state) = stack.pop() {
            match state {
                BuildState::WaitingForChildren {
                    rule,
                    mut processed_children,
                    mut remaining_children,
                    original_str,
                } => {
                    // Handle WaitingForChildren state directly in main loop
                    if let Some(completed_child) = results.pop()
                        && let Some(node) = completed_child
                    {
                        processed_children.push(node);
                    }

                    // Process next child or build final node
                    if let Some(next_child) = remaining_children.pop() {
                        // Push state back to continue processing remaining children
                        stack.push(BuildState::WaitingForChildren {
                            rule,
                            processed_children,
                            remaining_children,
                            original_str,
                        });
                        // Process next child
                        stack.push(BuildState::Process(next_child));
                    } else {
                        // All children processed, build the node
                        stack.push(BuildState::BuildFromChildren {
                            rule,
                            children: processed_children,
                            original_str,
                        });
                    }
                }
                _ => {
                    match self.process_state(state)? {
                        ProcessResult::Complete(node) => {
                            // If we're at the bottom of the stack, this is our final result
                            if stack.is_empty() {
                                return Ok(node);
                            }
                            // Otherwise, add to results for parent processing
                            results.push(node);
                        }
                        ProcessResult::PushStates(states) => {
                            // Push states in reverse order so they're processed in correct order
                            for state in states.into_iter().rev() {
                                stack.push(state);
                            }
                        }
                    }
                }
            }
        }

        // Should not reach here - if we do, return the last result or empty program
        if let Some(result) = results.pop() {
            Ok(result)
        } else {
            // Return empty program if nothing was processed
            Ok(Some(AstNode::Program(vec![])))
        }
    }

    /// Process a single state in the iterative builder
    fn process_state<'a>(
        &mut self,
        state: BuildState<'a>,
    ) -> Result<ProcessResult<'a>, Box<dyn std::error::Error>> {
        match state {
            BuildState::Process(pair) => {
                let rule = pair.as_rule();
                let pair_str = pair.as_str();

                // Handle simple leaf nodes directly
                match rule {
                    Rule::scalar_variable => {
                        return Ok(ProcessResult::Complete(Some(AstNode::ScalarVariable(
                            Arc::from(pair_str),
                        ))));
                    }
                    Rule::array_variable => {
                        return Ok(ProcessResult::Complete(Some(AstNode::ArrayVariable(
                            Arc::from(pair_str),
                        ))));
                    }
                    Rule::hash_variable => {
                        return Ok(ProcessResult::Complete(Some(AstNode::HashVariable(
                            Arc::from(pair_str),
                        ))));
                    }
                    Rule::number => {
                        return Ok(ProcessResult::Complete(Some(AstNode::Number(Arc::from(
                            pair_str,
                        )))));
                    }
                    Rule::string => {
                        return Ok(ProcessResult::Complete(Some(AstNode::String(Arc::from(
                            pair_str,
                        )))));
                    }
                    Rule::identifier => {
                        return Ok(ProcessResult::Complete(Some(AstNode::Identifier(Arc::from(
                            pair_str,
                        )))));
                    }
                    Rule::int_number => {
                        return Ok(ProcessResult::Complete(Some(AstNode::Number(Arc::from(
                            pair_str,
                        )))));
                    }
                    Rule::float_number => {
                        return Ok(ProcessResult::Complete(Some(AstNode::Number(Arc::from(
                            pair_str,
                        )))));
                    }
                    _ => {}
                }

                // For complex nodes, collect children and process them
                let children: Vec<_> = pair.into_inner().collect();

                if children.is_empty() {
                    // No children, return based on rule
                    Ok(ProcessResult::Complete(self.build_leaf_node(rule, pair_str)?))
                } else {
                    // Need to process children
                    let mut states = vec![BuildState::WaitingForChildren {
                        rule,
                        processed_children: Vec::new(),
                        remaining_children: children,
                        original_str: pair_str,
                    }];

                    // Start processing first child
                    if let Some(BuildState::WaitingForChildren {
                        rule,
                        processed_children,
                        mut remaining_children,
                        original_str,
                    }) = states.pop()
                        && let Some(first_child) = remaining_children.pop()
                    {
                        states.push(BuildState::WaitingForChildren {
                            rule,
                            processed_children,
                            remaining_children,
                            original_str,
                        });
                        states.push(BuildState::Process(first_child));
                    }

                    Ok(ProcessResult::PushStates(states))
                }
            }

            BuildState::WaitingForChildren { .. } => {
                // This state is handled in the main loop, not here
                // If encountered, return a descriptive error
                Err(Box::new(IterativeParserError::UnexpectedWaitingForChildren))
            }

            BuildState::BuildFromChildren { rule, children, original_str } => {
                // Build node from processed children
                Ok(ProcessResult::Complete(self.build_node_from_children(
                    rule,
                    children,
                    original_str,
                )?))
            }
        }
    }

    /// Build a leaf node (no children)
    fn build_leaf_node(
        &self,
        rule: Rule,
        _text: &str,
    ) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        match rule {
            Rule::EOI => Ok(None),
            Rule::WHITESPACE => Ok(None),
            Rule::comment => Ok(None),
            _ => Ok(Some(AstNode::Identifier(Arc::from(format!("unhandled_leaf_{:?}", rule))))),
        }
    }

    /// Build a node from its processed children
    fn build_node_from_children(
        &self,
        rule: Rule,
        children: Vec<AstNode>,
        _original_str: &str,
    ) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
        match rule {
            Rule::program => {
                // Program should always return something, even if empty
                if children.is_empty() {
                    Ok(Some(AstNode::Program(vec![])))
                } else {
                    Ok(Some(AstNode::Program(children)))
                }
            }

            Rule::statements => {
                // Multiple statements
                Ok(Some(AstNode::Program(children)))
            }

            Rule::block => Ok(Some(AstNode::Block(children))),

            Rule::statement => {
                // Statement usually has a single child
                Ok(children.into_iter().next())
            }

            Rule::simple_assignment => {
                if children.len() == 2 {
                    Ok(Some(AstNode::Assignment {
                        target: Box::new(children[0].clone()),
                        op: Arc::from("="),
                        value: Box::new(children[1].clone()),
                    }))
                } else {
                    Err("Invalid assignment node".into())
                }
            }

            Rule::assignment_expression => {
                if children.len() >= 3 {
                    // For assignment expressions, assume first is target, last is value
                    // and middle is operator
                    Ok(Some(AstNode::Assignment {
                        target: Box::new(children[0].clone()),
                        op: Arc::from("="),
                        value: Box::new(children[children.len() - 1].clone()),
                    }))
                } else if children.len() == 2 {
                    // Might be missing operator, just create assignment
                    Ok(Some(AstNode::Assignment {
                        target: Box::new(children[0].clone()),
                        op: Arc::from("="),
                        value: Box::new(children[1].clone()),
                    }))
                } else if children.len() == 1 {
                    // Just pass through single expression
                    Ok(Some(children[0].clone()))
                } else {
                    // Empty assignment expression
                    Ok(None)
                }
            }

            Rule::function_call => {
                if !children.is_empty() {
                    Ok(Some(AstNode::FunctionCall {
                        function: Box::new(children[0].clone()),
                        args: children[1..].to_vec(),
                    }))
                } else {
                    Err("Empty function call".into())
                }
            }

            Rule::expression => {
                // Expression should have a single child
                Ok(children.into_iter().next())
            }

            Rule::primary_expression => {
                // For parenthesized expressions, just pass through the inner expression
                Ok(children.into_iter().next())
            }

            Rule::ternary_expression => {
                if children.len() == 3 {
                    Ok(Some(AstNode::TernaryOp {
                        condition: Box::new(children[0].clone()),
                        true_expr: Box::new(children[1].clone()),
                        false_expr: Box::new(children[2].clone()),
                    }))
                } else if children.len() == 1 {
                    Ok(Some(children[0].clone()))
                } else {
                    Ok(None)
                }
            }

            _ => {
                // For unhandled rules, create a generic node
                if children.len() == 1 {
                    Ok(Some(children[0].clone()))
                } else if children.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(AstNode::List(children)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure_rust_parser::PerlParser;
    use pest::Parser;

    #[test]
    fn test_iterative_vs_recursive_simple() {
        use perl_tdd_support::{must, must_some};
        let mut parser = PureRustPerlParser::new();
        let input = "$x = 42";

        // Parse with Pest
        let pairs = must(PerlParser::parse(Rule::program, input));
        let pair = must_some(pairs.into_iter().next());

        // Compare iterative result
        let iterative_result = parser.build_node_iterative(pair.clone());

        // Check the result
        match iterative_result {
            Ok(Some(node)) => {
                // Success - we have a node
                println!("Iterative parser returned: {:?}", node);
            }
            Ok(None) => {
                assert!(false, "Iterative parser returned None");
            }
            Err(e) => {
                assert!(false, "Iterative parser failed with error: {}", e);
            }
        }
    }

    #[test]
    fn test_unexpected_waiting_state_error() {
        let mut parser = PureRustPerlParser::new();
        let state = BuildState::WaitingForChildren {
            rule: Rule::program,
            processed_children: vec![],
            remaining_children: vec![],
            original_str: "",
        };
        let result = parser.process_state(state);
        assert!(matches!(
            result,
            Err(e) if e.downcast_ref::<IterativeParserError>().is_some()
        ));
    }

    #[test]
    fn test_deep_nesting_iterative() {
        use perl_tdd_support::{must, must_some};
        let mut parser = PureRustPerlParser::new();

        // Create deeply nested expression - start with smaller depth
        let depth = 100;
        let mut expr = "42".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }

        // This should work without stack overflow in debug mode
        let pairs = must(PerlParser::parse(Rule::expression, &expr));
        let pair = must_some(pairs.into_iter().next());

        let result = parser.build_node_iterative(pair);
        assert!(result.is_ok(), "Deep nesting should work with iterative approach");

        // Now test with even deeper nesting
        let deep_depth = 500;
        let mut deep_expr = "42".to_string();
        for _ in 0..deep_depth {
            deep_expr = format!("({})", deep_expr);
        }

        let deep_pairs = must(PerlParser::parse(Rule::expression, &deep_expr));
        let deep_pair = must_some(deep_pairs.into_iter().next());

        let deep_result = parser.build_node_iterative(deep_pair);
        assert!(deep_result.is_ok(), "Very deep nesting should work with iterative approach");
    }
}
