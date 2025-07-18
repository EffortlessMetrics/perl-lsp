#!/usr/bin/env python3
"""
Perl code fuzzer - generates variations of Perl code to stress-test parsers
"""

import random
import os
import re
import sys
from pathlib import Path

# Perl code fragments for injection
FRAGMENTS = {
    'operators': [
        ' && ', ' || ', ' // ', ' and ', ' or ', ' xor ',
        ' + ', ' - ', ' * ', ' / ', ' % ', ' ** ',
        ' . ', ' x ', ' =~ ', ' !~ ',
        ' < ', ' > ', ' <= ', ' >= ', ' == ', ' != ',
        ' <=> ', ' cmp ', ' eq ', ' ne ', ' lt ', ' gt ',
        ' << ', ' >> ', ' & ', ' | ', ' ^ ', ' ~ ',
        '++', '--', '+=', '-=', '*=', '/=', '.=', '&&=', '||=', '//='
    ],
    
    'quotes': [
        'q{}', 'qq{}', 'qr{}', 'qw{}', 'qx{}',
        'q[]', 'qq[]', 'qr[]', 'qw[]', 'qx[]',
        'q()', 'qq()', 'qr()', 'qw()', 'qx()',
        'q<>', 'qq<>', 'qr<>', 'qw<>', 'qx<>',
        'q||', 'qq||', 'qr||', 'qw||', 'qx||',
        'q""', "qq''", 'qr//', 'qw//', 'qx``',
        'm{}', 'm[]', 'm()', 'm<>', 'm||', 'm//',
        's{}{};', 's[][];', 's()();', 's<><>;', 's|||;', 's///',
        'y{}{};', 'y[][];', 'y()();', 'y<><>;', 'y|||;', 'y///',
        'tr{}{};', 'tr[][];', 'tr()();', 'tr<><>;', 'tr|||;', 'tr///',
    ],
    
    'variables': [
        '$foo', '@foo', '%foo', '*foo', '&foo',
        '${foo}', '@{foo}', '%{foo}', '*{foo}', '&{foo}',
        '$foo->{bar}', '$foo->[0]', '$foo{bar}', '$foo[0]',
        '$$foo', '@$foo', '%$foo', '*$foo', '&$foo',
        '${"foo"}', '@{"foo"}', '%{"foo"}', '*{"foo"}',
        '$foo::', '$Foo::bar', '$::foo', '$main::foo',
        '$_', '@_', '%_', '$!', '$@', '$#', '$$', '$0',
        '$1', '$2', '$^A', '$^W', '$]', '$"', '$;',
    ],
    
    'blocks': [
        'BEGIN { }', 'END { }', 'CHECK { }', 'INIT { }',
        'do { }', 'eval { }', 'sub { }',
        'if (1) { }', 'unless (0) { }', 
        'while (0) { }', 'until (1) { }',
        'for (;;) { }', 'foreach (@_) { }',
        'given ($x) { }', 'when (1) { }',
        'try { } catch { }', 'finally { }',
        '{ }', '{ ; }', '{ 42 }',
    ],
    
    'statements': [
        'use strict;', 'use warnings;', 'no warnings;',
        'use feature qw(say state);', 'use 5.010;',
        'package Foo;', 'package Foo::Bar;', 'package main;',
        'sub foo { }', 'sub foo ($) { }', 'sub foo : lvalue { }',
        'my $x;', 'our $x;', 'local $x;', 'state $x;',
        'print "hello";', 'say "world";', 'warn "warning";',
        'die "error";', 'exit 0;', 'return;',
        'next;', 'last;', 'redo;', 'goto LABEL;',
        'LABEL:', '__END__', '__DATA__',
    ],
    
    'heredocs': [
        '<<EOF', '<<"EOF"', "<<'EOF'", '<<`EOF`',
        '<<\\EOF', '<<~EOF', '<<~"EOF"', "<<~'EOF'",
        '<< EOF', '<< "EOF"', "<< 'EOF'", '<< `EOF`',
        '<<""', "<<''", '<<``', '<< ""', "<< ''",
        '<<END', '<<HEREDOC', '<<HTML', '<<SQL',
    ],
    
    'special': [
        '...', '..', '\\', '//', '{}', '[]', '()',
        '=>', '->', '::',
        '__PACKAGE__', '__FILE__', '__LINE__', '__SUB__',
        'STDIN', 'STDOUT', 'STDERR', 'ARGV', 'DATA',
        'format =\n.\n', 'format STDOUT =\n.\n',
        '=pod\n\n=cut', '=head1 NAME\n\n=cut',
        '#line 42 "file.pl"', '# -*- mode: perl -*-',
    ],
    
    'complex_exprs': [
        '$$$$ref', '@{[@{[@array]}]}', '%{\\%hash}', 
        '*{*STDOUT{IO}}', '&{\\&sub}',
        '$x->$y->$z', '$x->[$y]->{$z}', '$x->{$y}->[$z]',
        '@$x[1..10]', '@{$x}[1..10]', '@x[1..10]',
        '%$x{keys %hash}', '%{$x}{keys %hash}',
        'map { $_ * 2 } grep { $_ > 0 } @array',
        'sort { $a <=> $b } map { chomp; $_ } <FILE>',
        'do { local $/; <FILE> }',
        'sub { @_ }->(1,2,3)',
    ],
    
    'edge_patterns': [
        # Nested delimiters
        'q{{}', 'qq{{}}', 'qr{{}}',
        'q{{{}}}', 'qq{{{{}}}}', 'qr{{{{{}}}}}',
        'm{\\{}', 's{\\{}{\\}}', 'y{\\{}{\\}}',
        
        # Empty constructs
        'sub{}', 'do{}', 'eval{}', 'BEGIN{}',
        'if(){}', 'while(){}', 'for(;;){}',
        '()', '[]', '{}',
        
        # Weird spacing
        'sub  foo  {  }', 'my  $x  =  42  ;',
        'print\n"hello"\n,\n"world"\n;',
        
        # Unicode
        'my $café = "☕";', 'sub 你好 { "hello" }',
        'package 日本語;', 'my $π = 3.14;',
    ]
}

def inject_fragment(code, fragment_type='random'):
    """Inject a random fragment into the code at a random position"""
    if fragment_type == 'random':
        fragment_type = random.choice(list(FRAGMENTS.keys()))
    
    if fragment_type not in FRAGMENTS:
        return code
    
    fragment = random.choice(FRAGMENTS[fragment_type])
    
    # Find valid injection points (not inside strings or comments)
    lines = code.split('\n')
    if not lines:
        return code
    
    # Simple heuristic: inject at line boundaries
    insert_line = random.randint(0, len(lines))
    
    # Add proper statement termination if needed
    if fragment_type in ['statements', 'heredocs']:
        if insert_line < len(lines):
            lines.insert(insert_line, fragment)
        else:
            lines.append(fragment)
    else:
        # For expressions, try to inject within a line
        if lines and insert_line < len(lines):
            line = lines[insert_line]
            # Avoid strings and comments (simple check)
            if '#' not in line and '"' not in line and "'" not in line:
                insert_pos = random.randint(0, len(line))
                lines[insert_line] = line[:insert_pos] + fragment + line[insert_pos:]
    
    return '\n'.join(lines)

def mutate_whitespace(code):
    """Randomly add/remove whitespace"""
    mutations = []
    for char in code:
        if char == ' ' and random.random() < 0.1:
            # Sometimes remove spaces
            if random.random() < 0.5:
                continue
            # Sometimes add extra spaces
            mutations.append(' ' * random.randint(1, 4))
        elif char == '\n' and random.random() < 0.1:
            # Sometimes add extra newlines
            mutations.append('\n' * random.randint(1, 3))
        else:
            mutations.append(char)
    
    return ''.join(mutations)

def mutate_operators(code):
    """Replace operators with similar ones"""
    replacements = [
        ('&&', random.choice(['and', '&&'])),
        ('||', random.choice(['or', '||'])),
        ('!', random.choice(['not', '!'])),
        ('==', random.choice(['eq', '=='])),
        ('!=', random.choice(['ne', '!='])),
        ('<', random.choice(['lt', '<'])),
        ('>', random.choice(['gt', '>'])),
        ('<=', random.choice(['le', '<='])),
        ('>=', random.choice(['ge', '>='])),
    ]
    
    for old, new in replacements:
        if random.random() < 0.3:
            code = code.replace(old, new)
    
    return code

def nest_constructs(code):
    """Add nested constructs"""
    nesting = [
        'do { ' + code + ' }',
        'eval { ' + code + ' }',
        'sub { ' + code + ' }->',
        'do { do { ' + code + ' } }',
        'if (1) { ' + code + ' }',
        'BEGIN { ' + code + ' }',
    ]
    
    if random.random() < 0.2:
        return random.choice(nesting)
    return code

def generate_fuzzed_file(seed_file, output_dir, num_variations=10):
    """Generate variations of a seed file"""
    with open(seed_file, 'r') as f:
        original = f.read()
    
    base_name = Path(seed_file).stem
    
    for i in range(num_variations):
        mutated = original
        
        # Apply multiple mutations
        num_mutations = random.randint(3, 10)
        
        for _ in range(num_mutations):
            mutation_type = random.choice([
                'inject', 'inject', 'inject',  # More injections
                'whitespace', 'operators', 'nest'
            ])
            
            if mutation_type == 'inject':
                mutated = inject_fragment(mutated)
            elif mutation_type == 'whitespace':
                mutated = mutate_whitespace(mutated)
            elif mutation_type == 'operators':
                mutated = mutate_operators(mutated)
            elif mutation_type == 'nest':
                # Only nest small sections
                lines = mutated.split('\n')
                if len(lines) > 5:
                    start = random.randint(0, len(lines) - 3)
                    end = start + random.randint(1, 3)
                    section = '\n'.join(lines[start:end])
                    nested = nest_constructs(section)
                    lines[start:end] = [nested]
                    mutated = '\n'.join(lines)
        
        # Ensure it still looks like Perl
        if not mutated.strip():
            mutated = original
        
        # Save the mutated file
        output_file = output_dir / f"fuzz_{base_name}_{i:03d}.pl"
        with open(output_file, 'w') as f:
            f.write("#!/usr/bin/perl\n")
            f.write("# Fuzzed from: " + str(seed_file) + "\n")
            f.write("# Mutation: " + str(i) + "\n")
            f.write("use strict;\n")
            f.write("use warnings;\n\n")
            f.write(mutated)
            if not mutated.endswith('\n'):
                f.write('\n')
            f.write("\n1;\n")

def generate_stress_tests(output_dir):
    """Generate specific stress test files"""
    
    # Deep nesting stress test
    nested = "my $x = "
    for i in range(50):
        nested += "{ a => "
    nested += "42"
    for i in range(50):
        nested += " }"
    nested += ";\n"
    
    with open(output_dir / "stress_deep_nesting.pl", 'w') as f:
        f.write("#!/usr/bin/perl\n# Deep nesting stress test\n")
        f.write(nested)
    
    # Operator precedence stress test
    ops = "my $x = 1"
    operators = [' + ', ' - ', ' * ', ' / ', ' % ', ' ** ', ' && ', ' || ', ' // ']
    for i in range(100):
        ops += random.choice(operators) + str(random.randint(1, 10))
    ops += ";\n"
    
    with open(output_dir / "stress_operators.pl", 'w') as f:
        f.write("#!/usr/bin/perl\n# Operator precedence stress test\n")
        f.write(ops)
    
    # Quote nesting stress test
    quotes = """
# Quote nesting stress test
my $x = qq{outer qq{middle qq{inner qq{deep}}}};
my $y = q{outer q{middle q{inner q{deep}}}};
my $z = qr{outer (?:qr{middle (?:qr{inner (?:qr{deep})})})};
my $w = s{q{a}q{b}q{c}}{qq{x}qq{y}qq{z}};
"""
    
    with open(output_dir / "stress_quotes.pl", 'w') as f:
        f.write("#!/usr/bin/perl\n")
        f.write(quotes)
    
    # Regex complexity stress test
    regex = """
# Regex complexity stress test
my $complex = qr{
    (?<name> \\w+ )
    \\s*
    (?:
        (?<num> \\d+ )
        |
        (?<word> [a-zA-Z]+ )
    )
    (?:
        (?{ print "code block\\n" })
        (?(?{ $1 eq 'test' }) yes | no )
        (?= lookahead )
        (?! negative )
        (?<= lookbehind )
        (?<! negative )
    )*
}x;
"""
    
    with open(output_dir / "stress_regex.pl", 'w') as f:
        f.write("#!/usr/bin/perl\n")
        f.write(regex)

def main():
    # Set up paths
    benchmark_dir = Path(__file__).parent
    output_dir = benchmark_dir / "fuzzed"
    output_dir.mkdir(exist_ok=True)
    
    # Find seed files
    seed_files = list(benchmark_dir.glob("*.pl"))
    seed_files = [f for f in seed_files if not f.stem.startswith("fuzz")]
    
    print(f"Found {len(seed_files)} seed files")
    
    # Generate fuzzed variations
    for seed_file in seed_files:
        print(f"Fuzzing {seed_file.name}...")
        generate_fuzzed_file(seed_file, output_dir, num_variations=10)
    
    # Generate stress tests
    print("Generating stress tests...")
    generate_stress_tests(output_dir)
    
    # Summary
    fuzzed_files = list(output_dir.glob("*.pl"))
    print(f"\nGenerated {len(fuzzed_files)} fuzzed files in {output_dir}")
    
    # Generate a manifest
    with open(output_dir / "manifest.txt", 'w') as f:
        f.write("# Fuzzed test files manifest\n")
        f.write(f"# Generated from {len(seed_files)} seed files\n")
        f.write(f"# Total files: {len(fuzzed_files)}\n\n")
        for file in sorted(fuzzed_files):
            f.write(f"{file.name}\n")

if __name__ == "__main__":
    main()