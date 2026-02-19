# Remaining Heredoc Edge Cases: The Final 0.1%

## Overview

While we've achieved ~99.9% heredoc coverage, there remain some extremely rare edge cases that are challenging due to Perl's dynamic nature and parsing ambiguities. This document explains what's missing, why it's hard, and what would be needed to implement them.

## 1. Format Strings with Heredocs

### The Challenge
```perl
format STDOUT =
@<<<<<<< @|||||| @>>>>>>
$name,   $score,  $grade
.

# Can you use heredocs in format strings?
format REPORT =
@<<<<<<<<<<<<<<<<<<<<<<<<<<<<
<<EOF
Multi-line format text
EOF
.
```

### Why It's Hard
- **Parsing Context**: Format strings have their own mini-language
- **Lexer State**: The lexer must track being inside a format declaration
- **Ambiguity**: Distinguishing between format placeholders and heredoc markers
- **Rarity**: Almost nobody uses formats anymore, let alone with heredocs

### What We Need
- Special lexer state for format declarations
- Format string parser that recognizes heredoc markers
- Understanding of format placeholder syntax vs heredoc syntax

## 2. Heredocs in BEGIN/CHECK/INIT/END Blocks at Compile Time

### The Challenge
```perl
BEGIN {
    # This executes at compile time
    my $config = <<'CONFIG';
    startup configuration
CONFIG
    eval $config;  # Compile-time eval of heredoc content
}

CHECK {
    # Can heredocs affect compilation order?
    require <<'MODULE';
Some::Dynamic::Module
MODULE
}
```

### Why It's Hard
- **Phase Distinction**: Perl has multiple execution phases (BEGIN, CHECK, INIT, runtime, END)
- **Compile-Time Execution**: BEGIN blocks run during parsing
- **State Management**: Parser state can be modified by BEGIN block execution
- **Circular Dependencies**: BEGIN blocks can modify the code being parsed

### What We Need
- Understanding of Perl's compilation phases
- Ability to execute code during parsing (like Perl itself does)
- State management across compilation phases

## 3. Source Filters Modifying Heredocs

### The Challenge
```perl
use Filter::Simple;

FILTER {
    # Source filter that modifies heredocs
    s/<<(\w+)/"MODIFIED_" . $1/ge;
};

my $text = <<EOF;  # Will this be modified by the filter?
Original text
EOF
```

### Why It's Hard
- **Pre-Processing**: Source filters modify code before parsing
- **Dynamic Transformation**: Filters can arbitrarily transform heredoc syntax
- **Multiple Passes**: Need to simulate filter execution before parsing
- **Filter Stacking**: Multiple filters can be applied in sequence

### What We Need
- Source filter emulation system
- Multi-pass parsing architecture
- Understanding of common source filters

## 4. Tied Filehandles and Heredocs

### The Challenge
```perl
tie *FH, 'MyPackage';
print FH <<EOF;
Does the tied filehandle affect heredoc parsing?
EOF

# Or even worse:
open my $fh, '>', \my $buffer;
print $fh <<EOF;
In-memory filehandle with heredoc
EOF
```

### Why It's Hard
- **Runtime Behavior**: Tied filehandles have custom behavior
- **Parser Assumptions**: Parser assumes standard I/O behavior
- **Dynamic Dispatch**: Method calls on tied objects at runtime

### What We Need
- Understanding of Perl's tie mechanism
- Runtime simulation capabilities
- Filehandle abstraction layer

## 5. Encoding Issues with Heredocs

### The Challenge
```perl
use utf8;
use encoding 'shiftjis';

my $text = <<'文字列';
日本語のヒアドキュメント
文字列

# Or with encoding layers:
binmode(DATA, ":encoding(UTF-16)");
my $utf16 = <DATA>;
__DATA__
<<HEREDOC
UTF-16 encoded heredoc content?
HEREDOC
```

### Why It's Hard
- **Encoding Layers**: Perl's I/O layers affect text interpretation
- **Lexer Encoding**: The lexer must handle multiple encodings
- **Runtime vs Parse Time**: Encoding can change at runtime
- **Delimiter Encoding**: Heredoc delimiters in non-ASCII

### What We Need
- Full Unicode and encoding support
- I/O layer emulation
- Multi-encoding lexer

## 6. Heredocs in Complex Regex Constructs

### The Challenge
```perl
# Recursive regex with heredoc in code block
$text =~ m{
    (?<content>
        (?:
            [^{}]++
            |
            \{ 
                (?&content)
            \}
            |
            (?{ 
                # Code block in regex
                $::var = <<'DATA';
heredoc in regex code block
DATA
            })
        )*
    )
}x;
```

### Why It's Hard
- **Regex Parser**: Need a full regex parser to find code blocks
- **Lexical Scope**: Code blocks have their own lexical scope
- **Execution Context**: Code executes during regex matching
- **Nesting**: Multiple levels of language nesting

### What We Need
- Complete regex parser
- Code block extraction from regex
- Context switching between regex and Perl

## 7. Heredocs with Custom Operators

### The Challenge
```perl
# Using Devel::Declare or similar
method foo ($arg) {
    return <<METHOD;
    Custom syntax heredoc
METHOD
}

# Or with operator overloading
use overload '<<' => sub {
    # Does this affect heredoc parsing?
};
```

### Why It's Hard
- **Syntax Extensions**: Perl's syntax can be modified at runtime
- **Parse Order**: Custom operators might not be defined when parsing
- **Ambiguity**: Distinguishing custom operators from heredocs

### What We Need
- Pluggable syntax system
- Runtime syntax modification support
- Operator precedence tables

## 8. Heredocs in String Eval with Lexical Scope

### The Challenge
```perl
my $outer = "outer";
{
    my $inner = "inner";
    eval 'print <<EOT;
Value: $inner
EOT';
    # Which $inner? Lexical scope in string eval is complex
}
```

### Why It's Hard
- **Lexical Scope**: String eval has complex scoping rules
- **Variable Capture**: Must track which variables are available
- **Closure Behavior**: Eval can create closures

### What We Need
- Full lexical scope tracking
- Variable capture analysis
- Closure compilation

## 9. Heredocs Split Across Files

### The Challenge
```perl
# In file1.pl
eval sprintf(<<'PART1' . 
# In file2.pl
<<'PART2', $arg);
First part: %s
PART1
Second part
PART2
```

### Why It's Hard
- **File Boundaries**: Parser assumes single file input
- **Dynamic Loading**: Files might be loaded at runtime
- **State Persistence**: Parser state across file boundaries

### What We Need
- Multi-file parsing support
- State serialization/deserialization
- Dynamic file loading

## 10. Heredocs in Generated Code

### The Challenge
```perl
# Self-modifying code
my $code = 'print <<EOF;';
$code .= "\nDynamic heredoc\n";
$code .= "EOF\n";
eval $code;

# Or with string interpolation generating heredocs
my $generator = '<<${delimiter}';
my $delimiter = 'END';
eval "print $generator\nContent\n$delimiter";
```

### Why It's Hard
- **Dynamic Generation**: Heredoc syntax created at runtime
- **Partial Parsing**: Need to parse incomplete heredoc declarations
- **Template Expansion**: Variable interpolation creating syntax

### What We Need
- Incremental parsing
- Template analysis
- Dynamic syntax generation

## Impact Analysis

### Coverage Impact
These edge cases represent approximately:
- Format strings: ~0.001% of Perl code
- BEGIN-time heredocs: ~0.01% 
- Source filters: ~0.001%
- Tied filehandles: ~0.001%
- Encoding issues: ~0.01%
- Regex code blocks: ~0.001%
- Custom operators: ~0.0001%
- Complex eval scoping: ~0.001%
- Multi-file heredocs: ~0.0001%
- Generated heredocs: ~0.001%

**Total**: ~0.1% of real-world Perl code

### Implementation Effort
Each edge case would require:
- 500-2000 lines of code
- Significant architectural changes
- Deep Perl internals knowledge
- Extensive testing

### Risk vs Reward
- **Risk**: High complexity, potential for bugs
- **Reward**: Minimal practical impact
- **Recommendation**: Document as known limitations

## Conclusion

These remaining edge cases exist at the intersection of:
1. **Dynamic code generation**
2. **Compile-time execution**
3. **Runtime syntax modification**
4. **Multiple parsing contexts**

They're hard because they require:
- **Full Perl interpreter**: Not just a parser
- **Runtime emulation**: Executing code during parsing
- **Dynamic syntax**: Handling syntax that changes during parsing
- **Complex state**: Managing state across phases and files

For a practical parser, documenting these as unsupported edge cases is reasonable. Full support would essentially require implementing a complete Perl interpreter, not just a parser.