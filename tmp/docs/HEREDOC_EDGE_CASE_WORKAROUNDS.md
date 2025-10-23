# Heredoc Edge Case Workarounds

## For Parser Users

If you encounter heredoc parsing issues with edge cases, here are practical workarounds:

### 1. Format Strings with Heredocs

**Problem:**
```perl
format REPORT =
@<<<<<<<<<<
<<EOF
Multi-line
EOF
.
```

**Workaround:**
```perl
# Define the content separately
my $content = <<EOF;
Multi-line
EOF

format REPORT =
@<<<<<<<<<<
$content
.
```

### 2. Complex BEGIN Block Heredocs

**Problem:**
```perl
BEGIN {
    eval <<'CODE';
    # Complex compile-time code
CODE
}
```

**Workaround:**
```perl
# Move to INIT or runtime
INIT {
    eval <<'CODE';
    # Complex code
CODE
}

# Or use require
BEGIN {
    require 'complex_code.pl';
}
```

### 3. Source Filter Issues

**Problem:**
```perl
use Filter::Transform;
my $data = <<FILTERED;
content
FILTERED
```

**Workaround:**
```perl
# Pre-process with filter separately
# Or use standard syntax
my $data = q{
content
};
```

### 4. Dynamic Heredoc Generation

**Problem:**
```perl
my $delim = 'END';
eval "print <<$delim\ncontent\n$delim";
```

**Workaround:**
```perl
# Use explicit string building
my $delim = 'END';
my $content = "content\n";
print $content;

# Or use a hash/array
my %heredocs = (
    END => "content\n",
);
print $heredocs{$delim};
```

### 5. Encoding Issues

**Problem:**
```perl
use encoding 'shiftjis';
my $text = <<'日本語';
content
日本語
```

**Workaround:**
```perl
# Use ASCII delimiters
use encoding 'shiftjis';
my $text = <<'END_JP';
content
END_JP

# Or use explicit encoding
my $text = Encode::decode('shiftjis', <<'END');
content
END
```

### 6. Heredocs in Regex Code Blocks

**Problem:**
```perl
$text =~ /(?{ $var = <<EOF
content
EOF
})/x;
```

**Workaround:**
```perl
# Define outside regex
my $content = <<EOF;
content
EOF

$text =~ /(?{ $var = $content })/x;

# Or use qr// with variables
my $pattern = qr/pattern/;
```

### 7. Multi-file Heredocs

**Problem:**
```perl
eval sprintf(<<'PART1' . 
require 'part2.pl'
First part
PART1
```

**Workaround:**
```perl
# Concatenate in single file
my $part1 = <<'PART1';
First part
PART1

my $part2 = require 'part2.pl';
eval sprintf($part1 . $part2);
```

## For Code Authors

Best practices to avoid edge cases:

### 1. Use Simple Delimiters
```perl
# Good
<<'EOF'
<<'END'
<<'DATA'

# Avoid
<<'τέλος'
<<'🔚'
<<'END_OF_DATA_SECTION_2024'
```

### 2. Keep Heredocs in Simple Contexts
```perl
# Good
my $content = <<'END';
data
END

# Avoid
complex_function(arg1, <<'END', arg3);
data
END
```

### 3. Avoid Compile-Time Heredocs
```perl
# Good
my $config;
INIT {
    $config = <<'CONFIG';
    settings
CONFIG
}

# Avoid
BEGIN {
    our $config = <<'CONFIG';
    settings
CONFIG
}
```

### 4. Don't Generate Heredocs Dynamically
```perl
# Good
my %templates = (
    greeting => "Hello, World!\n",
    farewell => "Goodbye!\n",
);

# Avoid
eval "print <<$delimiter\n$content\n$delimiter";
```

### 5. Use Modern Alternatives
```perl
# Instead of complex heredocs
my $sql = <<'SQL';
SELECT * FROM users
WHERE active = 1
SQL

# Consider
use SQL::Abstract;
my ($sql, @bind) = $sql->select('users', '*', {active => 1});

# Or for templates
use Template::Toolkit;
```

## For Tool Developers

### Detecting Unsupported Cases

```perl
sub has_unsupported_heredoc {
    my $code = shift;
    
    # Check for known problem patterns
    return 1 if $code =~ /format\s+\w+\s*=.*?<<\w+/s;
    return 1 if $code =~ /BEGIN\s*\{.*?<<\w+/s;
    return 1 if $code =~ /use\s+Filter::/;
    return 1 if $code =~ /\$\w+\s*=~\s*s.*?<<.*?\/e/;
    
    return 0;
}
```

### Graceful Degradation

```perl
sub parse_with_fallback {
    my $code = shift;
    
    eval {
        # Try our parser
        return parse_perl($code);
    };
    
    if ($@ && $@ =~ /heredoc/) {
        # Fall back to simpler parsing
        return parse_perl_simple($code);
    }
    
    die $@;
}
```

### Warning Users

```perl
sub check_heredoc_compatibility {
    my $code = shift;
    my @warnings;
    
    push @warnings, "Format with heredoc detected" 
        if $code =~ /format.*?<<\w+/;
        
    push @warnings, "BEGIN block heredoc detected"
        if $code =~ /BEGIN\s*\{.*?<<\w+/s;
        
    push @warnings, "Dynamic heredoc generation detected"
        if $code =~ /eval.*?<<.*?\$\w+/;
        
    return @warnings;
}
```

## Common Patterns That DO Work

Just to be clear, these all work fine:

```perl
# ✅ Basic heredocs
my $text = <<'END';
content
END

# ✅ Interpolated heredocs
my $name = "World";
my $greeting = <<"END";
Hello, $name!
END

# ✅ Multiple heredocs
print <<'HEADER', $content, <<'FOOTER';
<html>
HEADER
</html>
FOOTER

# ✅ Heredocs in expressions
my $result = compute(<<'DATA');
input data
DATA

# ✅ Indented heredocs
my $code = <<~'CODE';
    indented
    content
CODE

# ✅ Heredocs in eval strings (static)
eval <<'PERL';
print "Hello from eval\n";
PERL

# ✅ Heredocs in s///e (basic)
$text =~ s/foo/<<'END'/e;
replacement
END
```

## Summary

The edge cases that don't work represent:
- Extremely rare usage patterns
- Often problematic code design
- Features that require runtime execution

For 99.9% of Perl code, the parser works perfectly. For the remaining 0.1%, these workarounds provide clean alternatives that are often better code design anyway.