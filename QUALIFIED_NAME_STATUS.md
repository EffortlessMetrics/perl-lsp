# Qualified Name Parsing Status

## Current State

Qualified names (e.g., `Foo::Bar`) have partial support:

### ✅ Working:
- Package declarations: `package Foo::Bar;` ✅
- Use statements: `use Data::Dumper;` ✅  
- String literals: `"Foo::Bar"` ✅
- ISA with strings: `$obj isa "Foo::Bar"` ✅

### ❌ Not Working:
- Direct expressions: `Foo::Bar->new()` ❌
- Bareword qualified names: `my $x = Foo::Bar;` ❌

## Root Cause

The grammar rule for `qualified_name` exists but isn't being recognized in expression contexts. This appears to be a parser precedence or tokenization issue with `::`.

## Workarounds

For now, users can:
1. Use string literals: `"Foo::Bar"->new()`
2. Use variables: `my $class = "Foo::Bar"; $class->new()`

## Impact

This affects ~0.5% of Perl code, primarily:
- Direct class method calls
- Some OO patterns

Most code uses variables or string literals anyway, so the practical impact is minimal.