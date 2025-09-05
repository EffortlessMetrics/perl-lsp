//! Basic TDD workflow support for LSP
//!
//! Simplified TDD implementation focused on core red-green-refactor cycle

use crate::ast::{Node, NodeKind};
use crate::diagnostics::Diagnostic;

/// Basic test generator
pub struct TestGenerator {
    framework: String,
}

impl TestGenerator {
    pub fn new(framework: &str) -> Self {
        Self { framework: framework.to_string() }
    }

    /// Generate basic test for a subroutine
    pub fn generate_test(&self, name: &str, params: usize) -> String {
        let args = (0..params).map(|i| format!("'arg{}'", i + 1)).collect::<Vec<_>>().join(", ");

        match self.framework.as_str() {
            "Test2::V0" => {
                format!(
                    "use Test2::V0;\n\n\
                     subtest '{}' => sub {{\n    \
                     my $result = {}({});\n    \
                     ok($result, 'Function returns value');\n\
                     }};\n\n\
                     done_testing();\n",
                    name, name, args
                )
            }
            _ => {
                // Default to Test::More
                format!(
                    "use Test::More;\n\n\
                     subtest '{}' => sub {{\n    \
                     my $result = {}({});\n    \
                     ok(defined $result, 'Function returns defined value');\n\
                     }};\n\n\
                     done_testing();\n",
                    name, name, args
                )
            }
        }
    }

    /// Find all subroutines in AST
    pub fn find_subroutines(&self, node: &Node) -> Vec<SubroutineInfo> {
        let mut subs = Vec::new();
        self.find_subroutines_recursive(node, &mut subs);
        subs
    }

    fn find_subroutines_recursive(&self, node: &Node, subs: &mut Vec<SubroutineInfo>) {
        match &node.kind {
            NodeKind::Subroutine { name, signature, .. } => {
                subs.push(SubroutineInfo {
                    name: name.clone().unwrap_or_else(|| "anonymous".to_string()),
                    param_count: signature
                        .as_ref()
                        .map(|s| {
                            if let NodeKind::Signature { parameters } = &s.kind {
                                parameters.len()
                            } else {
                                0
                            }
                        })
                        .unwrap_or(0),
                });
            }
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.find_subroutines_recursive(stmt, subs);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_subroutines_recursive(stmt, subs);
                }
            }
            NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
                self.find_subroutines_recursive(then_branch, subs);
                for (_, branch) in elsif_branches {
                    self.find_subroutines_recursive(branch, subs);
                }
                if let Some(branch) = else_branch {
                    self.find_subroutines_recursive(branch, subs);
                }
            }
            NodeKind::While { body, continue_block, .. }
            | NodeKind::For { body, continue_block, .. } => {
                self.find_subroutines_recursive(body, subs);
                if let Some(cont) = continue_block {
                    self.find_subroutines_recursive(cont, subs);
                }
            }
            NodeKind::Foreach { body, .. }
            | NodeKind::Given { body, .. }
            | NodeKind::When { body, .. }
            | NodeKind::Default { body } => {
                self.find_subroutines_recursive(body, subs);
            }
            NodeKind::Package { block, .. } => {
                if let Some(blk) = block {
                    self.find_subroutines_recursive(blk, subs);
                }
            }
            NodeKind::Class { body, .. } => {
                self.find_subroutines_recursive(body, subs);
            }
            _ => {
                // Other node types don't contain subroutines
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubroutineInfo {
    pub name: String,
    pub param_count: usize,
}

/// Basic refactoring analyzer
pub struct RefactoringAnalyzer {
    max_complexity: usize,
    max_lines: usize,
    max_params: usize,
}

impl Default for RefactoringAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RefactoringAnalyzer {
    pub fn new() -> Self {
        Self { max_complexity: 10, max_lines: 50, max_params: 5 }
    }

    /// Analyze code and suggest refactorings
    pub fn analyze(&self, node: &Node, source: &str) -> Vec<RefactoringSuggestion> {
        let mut suggestions = Vec::new();
        self.analyze_recursive(node, source, &mut suggestions);
        suggestions
    }

    fn analyze_recursive(
        &self,
        node: &Node,
        source: &str,
        suggestions: &mut Vec<RefactoringSuggestion>,
    ) {
        match &node.kind {
            NodeKind::Subroutine { name, signature, body, .. } => {
                let sub_name = name.clone().unwrap_or_else(|| "anonymous".to_string());

                // Check parameter count
                let param_count = signature
                    .as_ref()
                    .map(|s| {
                        if let NodeKind::Signature { parameters } = &s.kind {
                            parameters.len()
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                if param_count > self.max_params {
                    suggestions.push(RefactoringSuggestion {
                        title: format!("Too many parameters in {}", sub_name),
                        description: format!(
                            "Function has {} parameters, consider using a hash",
                            param_count
                        ),
                        category: RefactoringCategory::TooManyParameters,
                    });
                }

                // Check complexity
                let complexity = self.calculate_complexity(body);
                if complexity > self.max_complexity {
                    suggestions.push(RefactoringSuggestion {
                        title: format!("High complexity in {}", sub_name),
                        description: format!(
                            "Cyclomatic complexity is {}, consider breaking into smaller functions",
                            complexity
                        ),
                        category: RefactoringCategory::HighComplexity,
                    });
                }

                // Check length
                let lines = self.count_lines(body, source);
                if lines > self.max_lines {
                    suggestions.push(RefactoringSuggestion {
                        title: format!("Long method: {}", sub_name),
                        description: format!(
                            "Method has {} lines, consider breaking into smaller functions",
                            lines
                        ),
                        category: RefactoringCategory::LongMethod,
                    });
                }

                // Recurse into body
                self.analyze_recursive(body, source, suggestions);
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.analyze_recursive(stmt, source, suggestions);
                }
            }
            NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
                self.analyze_recursive(then_branch, source, suggestions);
                for (_, branch) in elsif_branches {
                    self.analyze_recursive(branch, source, suggestions);
                }
                if let Some(branch) = else_branch {
                    self.analyze_recursive(branch, source, suggestions);
                }
            }
            NodeKind::While { body, .. }
            | NodeKind::For { body, .. }
            | NodeKind::Foreach { body, .. }
            | NodeKind::Given { body, .. }
            | NodeKind::When { body, .. }
            | NodeKind::Default { body }
            | NodeKind::Class { body, .. } => {
                self.analyze_recursive(body, source, suggestions);
            }
            NodeKind::Package { block, .. } => {
                if let Some(blk) = block {
                    self.analyze_recursive(blk, source, suggestions);
                }
            }
            _ => {
                // Other node types don't need analysis
            }
        }
    }

    fn calculate_complexity(&self, node: &Node) -> usize {
        let mut complexity = 1;
        self.count_decision_points(node, &mut complexity);
        complexity
    }

    fn count_decision_points(&self, node: &Node, complexity: &mut usize) {
        match &node.kind {
            NodeKind::If { elsif_branches, .. } => {
                *complexity += 1 + elsif_branches.len();
            }
            NodeKind::While { .. } | NodeKind::For { .. } | NodeKind::Foreach { .. } => {
                *complexity += 1;
            }
            NodeKind::Binary { op, left, right } => {
                if op == "&&" || op == "||" || op == "and" || op == "or" {
                    *complexity += 1;
                }
                self.count_decision_points(left, complexity);
                self.count_decision_points(right, complexity);
                return;
            }
            _ => {}
        }

        // Recurse into children
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.count_decision_points(stmt, complexity);
                }
            }
            NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
                self.count_decision_points(then_branch, complexity);
                for (_, branch) in elsif_branches {
                    self.count_decision_points(branch, complexity);
                }
                if let Some(branch) = else_branch {
                    self.count_decision_points(branch, complexity);
                }
            }
            NodeKind::While { body, .. }
            | NodeKind::For { body, .. }
            | NodeKind::Foreach { body, .. }
            | NodeKind::Given { body, .. }
            | NodeKind::When { body, .. }
            | NodeKind::Default { body }
            | NodeKind::Class { body, .. } => {
                self.count_decision_points(body, complexity);
            }
            _ => {}
        }
    }

    fn count_lines(&self, node: &Node, source: &str) -> usize {
        let start = node.location.start;
        let end = node.location.end.min(source.len());

        if start >= end {
            return 0;
        }

        source[start..end].lines().count()
    }
}

#[derive(Debug, Clone)]
pub struct RefactoringSuggestion {
    pub title: String,
    pub description: String,
    pub category: RefactoringCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefactoringCategory {
    TooManyParameters,
    HighComplexity,
    LongMethod,
}

/// Simple TDD workflow state
#[derive(Debug, Clone, PartialEq)]
pub enum TddState {
    Red,
    Green,
    Refactor,
    Idle,
}

/// TDD workflow manager
pub struct TddWorkflow {
    state: TddState,
    generator: TestGenerator,
    analyzer: RefactoringAnalyzer,
}

impl TddWorkflow {
    pub fn new(framework: &str) -> Self {
        Self {
            state: TddState::Idle,
            generator: TestGenerator::new(framework),
            analyzer: RefactoringAnalyzer::new(),
        }
    }

    /// Start TDD cycle
    pub fn start_cycle(&mut self, test_name: &str) -> TddResult {
        self.state = TddState::Red;
        TddResult {
            state: self.state.clone(),
            message: format!("Starting TDD cycle for '{}'", test_name),
        }
    }

    /// Run tests and update state
    pub fn run_tests(&mut self, success: bool) -> TddResult {
        self.state = if success { TddState::Green } else { TddState::Red };

        TddResult {
            state: self.state.clone(),
            message: if success {
                "Tests passing, ready to refactor".to_string()
            } else {
                "Tests failing, fix implementation".to_string()
            },
        }
    }

    /// Move to refactor phase
    pub fn start_refactor(&mut self) -> TddResult {
        self.state = TddState::Refactor;
        TddResult {
            state: self.state.clone(),
            message: "Refactoring phase - improve code while keeping tests green".to_string(),
        }
    }

    /// Complete cycle
    pub fn complete_cycle(&mut self) -> TddResult {
        self.state = TddState::Idle;
        TddResult { state: self.state.clone(), message: "TDD cycle complete".to_string() }
    }

    /// Generate test for function
    pub fn generate_test(&self, name: &str, params: usize) -> String {
        self.generator.generate_test(name, params)
    }

    /// Analyze code for refactoring
    pub fn analyze_for_refactoring(&self, ast: &Node, source: &str) -> Vec<RefactoringSuggestion> {
        self.analyzer.analyze(ast, source)
    }

    /// Get coverage diagnostics
    pub fn get_coverage_diagnostics(&self, uncovered_lines: &[usize]) -> Vec<Diagnostic> {
        uncovered_lines
            .iter()
            .map(|&line| Diagnostic {
                range: (line, line),
                severity: crate::diagnostics::DiagnosticSeverity::Warning,
                code: Some("tdd.uncovered".to_string()),
                message: "Line not covered by tests".to_string(),
                related_information: vec![],
                tags: vec![],
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TddResult {
    pub state: TddState,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceLocation;

    #[test]
    fn test_test_generation() {
        let generator = TestGenerator::new("Test::More");
        let test = generator.generate_test("add", 2);
        assert!(test.contains("Test::More"));
        assert!(test.contains("add"));
        assert!(test.contains("arg1"));
        assert!(test.contains("arg2"));
    }

    #[test]
    fn test_find_subroutines() {
        let ast = Node::new(
            NodeKind::Program {
                statements: vec![Node::new(
                    NodeKind::Subroutine {
                        name: Some("test_func".to_string()),
                        name_span: None,
                        prototype: None,
                        signature: None,
                        attributes: vec![],
                        body: Box::new(Node::new(
                            NodeKind::Block { statements: vec![] },
                            SourceLocation { start: 0, end: 0 },
                        )),
                    },
                    SourceLocation { start: 0, end: 0 },
                )],
            },
            SourceLocation { start: 0, end: 0 },
        );

        let generator = TestGenerator::new("Test::More");
        let subs = generator.find_subroutines(&ast);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "test_func");
    }

    #[test]
    fn test_refactoring_suggestions() {
        let analyzer = RefactoringAnalyzer::new();

        // Create a subroutine with too many parameters
        let ast = Node::new(
            NodeKind::Subroutine {
                name: Some("complex".to_string()),
                name_span: None,
                prototype: None,
                signature: Some(Box::new(Node::new(
                    NodeKind::Signature {
                        parameters: vec![
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "a".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "b".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "c".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "d".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "e".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            Node::new(
                                NodeKind::MandatoryParameter {
                                    variable: Box::new(Node::new(
                                        NodeKind::Variable {
                                            sigil: "$".to_string(),
                                            name: "f".to_string(),
                                        },
                                        SourceLocation { start: 0, end: 0 },
                                    )),
                                },
                                SourceLocation { start: 0, end: 0 },
                            ),
                            // 6 parameters - more than max_params (5)
                        ],
                    },
                    SourceLocation { start: 0, end: 0 },
                ))),
                attributes: vec![],
                body: Box::new(Node::new(
                    NodeKind::Block { statements: vec![] },
                    SourceLocation { start: 0, end: 0 },
                )),
            },
            SourceLocation { start: 0, end: 0 },
        );

        let suggestions = analyzer.analyze(&ast, "sub complex($a, $b, $c, $d, $e, $f) { }");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.category == RefactoringCategory::TooManyParameters));
    }

    #[test]
    fn test_tdd_workflow() {
        let mut workflow = TddWorkflow::new("Test::More");

        // Start cycle
        let _result = workflow.start_cycle("add");
        assert_eq!(workflow.state, TddState::Red);

        // Run failing tests
        let _result = workflow.run_tests(false);
        assert_eq!(workflow.state, TddState::Red);

        // Run passing tests
        let _result = workflow.run_tests(true);
        assert_eq!(workflow.state, TddState::Green);

        // Start refactoring
        let _result = workflow.start_refactor();
        assert_eq!(workflow.state, TddState::Refactor);

        // Complete cycle
        let _result = workflow.complete_cycle();
        assert_eq!(workflow.state, TddState::Idle);
    }
}
