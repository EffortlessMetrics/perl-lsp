#!/usr/bin/env python3
"""
Example of analyzing perl-parser S-expressions using Python tools.

This demonstrates how standard S-expression libraries can work with our output.
"""

import re
from typing import List, Tuple, Any, Dict

class SExpParser:
    """Simple S-expression parser for perl-parser output."""
    
    def __init__(self):
        self.pos = 0
        self.text = ""
    
    def parse(self, text: str) -> Any:
        """Parse an S-expression string."""
        self.text = text.strip()
        self.pos = 0
        return self._parse_expr()
    
    def _parse_expr(self) -> Any:
        """Parse a single expression."""
        self._skip_whitespace()
        
        if self.pos >= len(self.text):
            return None
            
        if self.text[self.pos] == '(':
            return self._parse_list()
        else:
            return self._parse_atom()
    
    def _parse_list(self) -> List[Any]:
        """Parse a list expression."""
        self.pos += 1  # Skip '('
        elements = []
        
        while True:
            self._skip_whitespace()
            if self.pos >= len(self.text):
                raise ValueError("Unexpected end of input")
            if self.text[self.pos] == ')':
                self.pos += 1  # Skip ')'
                break
            elements.append(self._parse_expr())
        
        return elements
    
    def _parse_atom(self) -> str:
        """Parse an atom (identifier, number, string)."""
        self._skip_whitespace()
        
        # Handle quoted strings
        if self.text[self.pos] == '"':
            return self._parse_string()
        
        # Handle other atoms
        start = self.pos
        while self.pos < len(self.text) and self.text[self.pos] not in ' \t\n()':
            self.pos += 1
        
        return self.text[start:self.pos]
    
    def _parse_string(self) -> str:
        """Parse a quoted string."""
        self.pos += 1  # Skip opening quote
        start = self.pos
        
        while self.pos < len(self.text) and self.text[self.pos] != '"':
            if self.text[self.pos] == '\\':
                self.pos += 2  # Skip escape sequence
            else:
                self.pos += 1
        
        if self.pos >= len(self.text):
            raise ValueError("Unterminated string")
        
        result = self.text[start:self.pos]
        self.pos += 1  # Skip closing quote
        return result
    
    def _skip_whitespace(self):
        """Skip whitespace characters."""
        while self.pos < len(self.text) and self.text[self.pos] in ' \t\n':
            self.pos += 1


class PerlASTAnalyzer:
    """Analyze perl-parser AST in S-expression format."""
    
    def __init__(self):
        self.parser = SExpParser()
    
    def analyze(self, sexp: str) -> Dict[str, Any]:
        """Analyze an S-expression and extract information."""
        ast = self.parser.parse(sexp)
        
        return {
            'variables': self._find_variables(ast),
            'functions': self._find_functions(ast),
            'operators': self._find_operators(ast),
            'complexity': self._calculate_complexity(ast),
            'structure': self._analyze_structure(ast)
        }
    
    def _find_variables(self, node: Any) -> List[Tuple[str, str]]:
        """Find all variables in the AST."""
        variables = []
        
        if isinstance(node, list):
            if len(node) > 0 and node[0] == 'variable':
                if len(node) >= 3:
                    sigil = node[1]
                    name = node[2]
                    variables.append((sigil, name))
            
            # Recurse into children
            for child in node:
                variables.extend(self._find_variables(child))
        
        return variables
    
    def _find_functions(self, node: Any) -> List[Dict[str, Any]]:
        """Find all function declarations and calls."""
        functions = []
        
        if isinstance(node, list) and len(node) > 0:
            # Function declarations
            if node[0] == 'sub':
                name = node[1] if len(node) > 1 else 'anonymous'
                functions.append({
                    'type': 'declaration',
                    'name': name,
                    'node': node
                })
            
            # Function calls
            elif node[0] == 'call':
                name = node[1] if len(node) > 1 else 'unknown'
                functions.append({
                    'type': 'call',
                    'name': name,
                    'node': node
                })
            
            # Method calls
            elif node[0] == 'method_call':
                method = node[2] if len(node) > 2 else 'unknown'
                functions.append({
                    'type': 'method_call',
                    'name': method,
                    'node': node
                })
            
            # Recurse
            for child in node:
                functions.extend(self._find_functions(child))
        
        return functions
    
    def _find_operators(self, node: Any) -> List[str]:
        """Find all operators used in the code."""
        operators = []
        
        if isinstance(node, list) and len(node) > 0:
            node_type = str(node[0])
            
            # Binary operators
            if node_type.startswith('binary_'):
                op = node_type[7:]  # Remove 'binary_' prefix
                operators.append(op)
            
            # Unary operators
            elif node_type.startswith('unary_'):
                op = node_type[6:]  # Remove 'unary_' prefix
                operators.append(op)
            
            # Assignment operators
            elif node_type.startswith('assignment_'):
                op = node_type[11:]  # Remove 'assignment_' prefix
                operators.append(op)
            
            # Recurse
            for child in node:
                operators.extend(self._find_operators(child))
        
        return operators
    
    def _calculate_complexity(self, node: Any, depth: int = 0) -> int:
        """Calculate cyclomatic complexity."""
        complexity = 0
        
        if isinstance(node, list) and len(node) > 0:
            node_type = node[0]
            
            # Control flow increases complexity
            if node_type in ['if', 'elsif', 'while', 'for', 'foreach', 'given', 'when']:
                complexity += 1
            
            # Logical operators increase complexity
            elif node_type in ['binary_&&', 'binary_||', 'binary_and', 'binary_or']:
                complexity += 1
            
            # Recurse
            for child in node:
                complexity += self._calculate_complexity(child, depth + 1)
        
        return complexity
    
    def _analyze_structure(self, node: Any) -> Dict[str, int]:
        """Analyze code structure metrics."""
        metrics = {
            'max_depth': 0,
            'node_count': 0,
            'leaf_count': 0
        }
        
        self._analyze_structure_recursive(node, 0, metrics)
        return metrics
    
    def _analyze_structure_recursive(self, node: Any, depth: int, metrics: Dict[str, int]):
        """Recursive helper for structure analysis."""
        metrics['node_count'] += 1
        metrics['max_depth'] = max(metrics['max_depth'], depth)
        
        if isinstance(node, list):
            if len(node) == 1:  # Leaf node
                metrics['leaf_count'] += 1
            else:
                for child in node[1:]:  # Skip node type
                    self._analyze_structure_recursive(child, depth + 1, metrics)
        else:
            metrics['leaf_count'] += 1


def pretty_print_ast(node: Any, indent: int = 0) -> None:
    """Pretty print an AST node."""
    if isinstance(node, list):
        if len(node) > 0:
            print(' ' * indent + f'({node[0]}')
            for child in node[1:]:
                pretty_print_ast(child, indent + 2)
            print(' ' * indent + ')')
    else:
        print(' ' * indent + str(node))


# Example usage
if __name__ == '__main__':
    # Example S-expressions from perl-parser
    examples = [
        '(program (my_declaration (variable $ x)(number 42)))',
        '(program (call print ((variable $ x) (string "\\n"))))',
        '(program (if (binary_> (variable $ x) (number 0)) (block (call print ((string "positive"))))))',
        '(program (sub factorial ((variable $ n))(block (if (binary_<= (variable $ n) (number 1)) (block (return (number 1))) (else (block (return (binary_* (variable $ n) (call factorial ((binary_- (variable $ n) (number 1))))))))))))',
    ]
    
    analyzer = PerlASTAnalyzer()
    
    for i, sexp in enumerate(examples):
        print(f"\n{'='*60}")
        print(f"Example {i+1}: {sexp[:50]}...")
        print('='*60)
        
        # Parse and analyze
        ast = analyzer.parser.parse(sexp)
        analysis = analyzer.analyze(sexp)
        
        # Print results
        print("\nParsed AST:")
        pretty_print_ast(ast)
        
        print("\nAnalysis Results:")
        print(f"Variables found: {analysis['variables']}")
        print(f"Functions found: {[f['name'] for f in analysis['functions']]}")
        print(f"Operators used: {list(set(analysis['operators']))}")
        print(f"Complexity: {analysis['complexity']}")
        print(f"Structure: {analysis['structure']}")
        
        # Demonstrate transformations
        if analysis['variables']:
            print("\nVariable Transformations:")
            for sigil, name in analysis['variables']:
                print(f"  {sigil} {name} -> ${sigil}{name}")