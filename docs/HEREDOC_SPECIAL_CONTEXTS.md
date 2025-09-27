# Heredoc Behavior in Special Perl Contexts

## Research Summary

This document summarizes the research on how Perl handles heredocs in special contexts like eval strings, regex substitutions with /e flag, and qx// operators.

## Key Findings

### 1. Heredocs in eval Strings

When a heredoc appears inside an eval string literal, it remains part of the string and is NOT evaluated during initial parsing:

```perl
eval 'my $x = <<EOF;
content
EOF
print $x;';
```

Perl's deparse shows this becomes:
```perl
eval "my \$x = <<EOF;\ncontent\nEOF\nprint \$x;";
```

The heredoc is evaluated when the eval executes, not when the eval string is parsed.

### 2. Heredocs in s///e Substitutions

The `/e` flag treats the replacement as Perl code to evaluate. Heredocs in the replacement are processed during substitution evaluation:

```perl
my $text = "X";
$text =~ s/X/<<EOF/e;
replacement
EOF
```

Deparse reveals Perl converts this to:
```perl
$text =~ s/X/"replacement\n";/e;
```

**Key insight**: Perl processes the heredoc during compilation and converts it to a string literal in the replacement code.

### 3. Complex s///e Cases

With code blocks in s///e:
```perl
$text =~ s/X/do { my $h = <<HD; chomp $h; $h }/e;
content
HD
```

Becomes:
```perl
$text =~ s/X/do { my $h = "content\n"; chomp $h; $h };/e;
```

### 4. Heredocs in qx// and Backticks

Heredocs used to build commands are evaluated before the command execution:
```perl
my $cmd = <<CMD;
echo "test"
CMD
my $result = qx($cmd);
```

The heredoc content is NOT passed literally to the shell.

### 5. Multiple Heredocs in Expressions

Multiple heredocs can appear in a single expression:
```perl
my ($a, $b) = (<<A, <<B);
First
A
Second
B
```

Perl handles these left-to-right, collecting content after the statement ends.

## Parser Implementation Challenges

### Current Issues

1. **Context Awareness**: The parser needs to recognize when it's in a context where heredocs will be evaluated differently (e.g., inside s///e replacements).

2. **Multi-phase Processing**: Heredocs in s///e require special handling because:
   - The replacement part is code when /e is present
   - Heredocs in that code need to be processed
   - But they're processed at compile time, not runtime

3. **Statement Boundaries**: The current implementation correctly identifies statement boundaries for normal heredocs but struggles with heredocs inside code contexts like s///e.

### Implementation Recommendations

1. **Enhanced Context Tracking**: The parser needs to track whether it's inside:
   - A string literal (heredocs remain literal)
   - An s///e replacement (heredocs get evaluated)
   - A code block within s///e (heredocs get evaluated)

2. **Special s///e Handling**: When parsing substitutions:
   - Check for /e flag
   - If present, parse replacement as code, not as a literal
   - Apply heredoc processing to that code

3. **Eval String Handling**: Heredocs inside eval strings should be preserved as-is, not processed during initial parsing.

## Test Cases

The following test files demonstrate these behaviors:
- `test_heredoc_eval.pl` - Heredocs in eval contexts
- `test_heredoc_substitution.pl` - Heredocs in s///e substitutions  
- `test_heredoc_qx.pl` - Heredocs with qx// and backticks
- `test_heredoc_complex.pl` - Complex nested cases
- `test_heredoc_context_analysis.pl` - Focused analysis of context-sensitive behavior

## Perl's Internal Handling

Based on deparse output, Perl appears to:
1. Identify heredocs during compilation
2. Collect their content after statement boundaries
3. Replace heredoc syntax with string literals in the compiled code
4. Special handling for s///e where replacement is treated as code

This explains why our parser sees transformed code rather than the original heredoc syntax in certain contexts.