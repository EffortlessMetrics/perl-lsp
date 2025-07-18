# AST Comparison: Tree-sitter vs Pure Rust Parser

## Missing Constructs in Pure Rust Parser

### 1. **Use Statements**
- Tree-sitter: `(use_statement use (package strict) ;)`
- Pure Rust: `(identifier strict)` - incorrectly parsed as just an identifier

### 2. **Package Statements**
- Tree-sitter: `(package_statement package (package MyClass) ;)`
- Pure Rust: Missing entirely - parsed as part of comment

### 3. **Our/My Variable Declarations**
- Tree-sitter: `(variable_declaration our (array @ (varname EXPORT_OK)))`
- Pure Rust: `(variable_declaration (@EXPORT_OK )` - missing 'our' keyword

### 4. **Subroutine Declarations**
- Tree-sitter: `(subroutine_declaration_statement sub (bareword new) (block {...}))`
- Pure Rust: Missing - not parsing full subroutine structure

### 5. **Hash References and Dereferencing**
- Tree-sitter: `(hash_element_expression (container_variable $ (varname args)) { (autoquoted_bareword data) })`
- Pure Rust: Missing - not parsing hash element access like `$args{data}`

### 6. **Binary Expressions**
- Tree-sitter: `(binary_expression ... || ...)`
- Pure Rust: Missing - not parsing || operator and precedence

### 7. **Anonymous Hash/Array Constructors**
- Tree-sitter: `(anonymous_hash_expression { ... })`
- Pure Rust: Missing - not parsing anonymous constructors

### 8. **For/Foreach Loops**
- Tree-sitter: `(for_statement foreach my (scalar $ (varname item)) ...)`
- Pure Rust: Missing entirely

### 9. **Eval Blocks**
- Tree-sitter: `(eval_expression eval (block { ... }))`
- Pure Rust: Missing entirely

### 10. **If Statements**
- Tree-sitter: `(conditional_statement if ( ... ) (block { ... }))`
- Pure Rust: Missing entirely

### 11. **Method Calls with Arrow Operator**
- Tree-sitter: `(method_call_expression (bareword MyClass) -> (method new) ( ... ))`
- Pure Rust: `(method_call (identifier MyClass) new )` - simplified, missing proper structure

### 12. **Anonymous Subroutines**
- Tree-sitter: `(anonymous_subroutine_expression sub (block { ... }))`
- Pure Rust: Missing entirely

### 13. **Special Variables**
- Tree-sitter: `(scalar $ (varname @))` for `$@`
- Pure Rust: Missing - not recognizing special error variable

### 14. **Return Statements**
- Tree-sitter: `(return_expression return ...)`
- Pure Rust: Missing entirely

### 15. **Reference Operator**
- Tree-sitter: `(refgen_expression \ (array @ (varname results)))`
- Pure Rust: `(array_reference \@results)` - simplified

### 16. **Fat Comma Operator**
- Tree-sitter: `(autoquoted_bareword data) => ...`
- Pure Rust: Missing - not parsing => properly

### 17. **Range Operator**
- Tree-sitter: `(binary_expression (number 1.) . (number 10))`
- Pure Rust: Parsing as separate numbers

### 18. **Postfix Conditionals**
- Tree-sitter: `(postfix_conditional_expression ... if ...)`
- Pure Rust: Missing entirely

### 19. **String Interpolation**
- Tree-sitter: `(interpolated_string_literal " (string_content ...) ")`
- Pure Rust: Missing - not handling variable interpolation in strings

### 20. **END Marker and POD**
- Tree-sitter: `(eof_marker __END__) (pod ...)`
- Pure Rust: Missing entirely

## Priority Implementation Order

1. **use/require statements** - Essential for module loading
2. **package declarations** - Core namespace feature
3. **sub declarations** - Function definitions
4. **if/unless/for/foreach/while** - Control flow
5. **Method calls with ->** - OOP support
6. **Hash/array dereferencing** - Data structure access
7. **Binary operators** - Expression evaluation
8. **eval blocks** - Exception handling
9. **Anonymous subs** - Callbacks and closures
10. **String interpolation** - Common string feature