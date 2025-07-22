# Perl Parser Status

This document tracks the current capabilities of the perl-parser implementation.

## ✅ Completed Features

### Variables
- Variable declarations: `my`, `our`, `local`, `state`
- All variable types: scalars (`$`), arrays (`@`), hashes (`%`), subroutines (`&`), globs (`*`)
- Special variables: `$_`, `$!`, `$?`, `$@`, `$$`, `$^O`, `@_`, `@ARGV`, etc.
- Variable dereferencing: `$array[0]`, `$hash{key}`, `$ref->[0]`, `$ref->{key}`
- Method calls: `$obj->method()`, `$obj->method($arg1, $arg2)`
- Complex dereferencing chains: `$data->{users}[$i]{name}`

### Literals and Expressions
- Numbers: integers, floats, scientific notation
- Strings: single-quoted (literal), double-quoted (interpolated)
- String interpolation detection (marks double-quoted strings as interpolated)
- Array literals: `[1, 2, 3]`, `[]`
- Hash literals: `{}` (empty hash ref)
- Operators: arithmetic, comparison, logical, assignment
- Binary operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
- Regex match operators: `=~`, `!~`
- Method call operator: `->`
- Array/hash access operators: `[]`, `{}`

### Control Flow
- Conditionals: `if`, `elsif`, `else`, `unless`
- Loops: `while`, `until`, `for`, `foreach`
- Loop control: `last`, `next`, `redo`
- Return statements: `return`, `return $value`

### Functions and OOP
- Subroutine declarations: `sub name { ... }`
- Anonymous subroutines: `sub { ... }`
- Function calls: `func()`, `func($arg1, $arg2)`
- Method calls: `$obj->method()`, `Class->new()`
- Object construction: `bless($ref, 'Class')`, `bless([], 'Class')`

### Package System
- Package declarations: `package Foo;`, `package Foo { ... }`
- Module loading: `use Module;`, `use Module qw(import list);`
- Pragma control: `no strict;`, `no warnings;`

### Regular Expressions
- Regex literals: `/pattern/`, `/pattern/modifiers`
- Match operators: `$str =~ /pattern/`, `$str !~ /pattern/`

### Other Features
- Word lists: `qw(word1 word2 word3)`
- Phase blocks: `BEGIN { }`, `END { }`, `CHECK { }`, `INIT { }`, `UNITCHECK { }`
- Comments: `# single line comments`

## 🚧 Partially Implemented

### Expressions
- Comma operator works but creates array literals
- Hash literals with pairs not fully implemented

## ❌ Not Yet Implemented

### Language Features
- `qw()` operator for word lists
- BEGIN/END/CHECK/INIT blocks
- Substitution operators: `s///`, `tr///`, `y///`
- Bare regex in conditionals: `if (/pattern/) { }`
- Function calls without parentheses: `print "hello"`
- Prototypes and signatures
- Heredocs
- Format strings
- POD documentation
- Labels and goto
- eval blocks
- do blocks
- given/when (smart match)

### Advanced Features
- Typeglobs and symbol table manipulation
- Tied variables
- Operator overloading
- Attributes
- Source filters

## Current Limitations

1. **Parser Ambiguities**: Some Perl constructs are ambiguous and require semantic analysis:
   - `{}` can be either a hash ref or a block
   - Function calls without parentheses vs separate statements
   
2. **Context Sensitivity**: Perl's context-sensitive features are not fully handled:
   - List vs scalar context
   - Void context
   
3. **String Interpolation**: While interpolated strings are detected, the actual interpolation parsing is not implemented

4. **Regex Parsing**: Regex patterns are stored as strings, not parsed into their components

## Usage Examples

```perl
# Variable declarations
my $scalar = 42;
our @array = (1, 2, 3);
local %hash = (key => 'value');

# Special variables
print $_;
die $! if $?;

# Dereferencing
$array[0] = $hash{key};
$obj->method($arg);
$data->{users}[$i]{name};

# Control flow
if ($x > 0) {
    print "positive";
} elsif ($x < 0) {
    print "negative";
} else {
    print "zero";
}

# Loops
for (my $i = 0; $i < 10; $i++) {
    print $i;
}

foreach my $item (@list) {
    process($item);
}

# Pattern matching
if ($str =~ /pattern/i) {
    print "match!";
}

# OOP
my $obj = bless({}, 'MyClass');
$obj->method();
```