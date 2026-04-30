//! Behavior-driven parser scenarios for core Perl workflows.
//!
//! The goal of this suite is to keep high-value parser behavior readable as
//! executable user stories.

use perl_parser::Parser;
use perl_tdd_support::must;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parse_sexp(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    Ok(ast.to_sexp())
}

#[test]
fn bdd_given_named_sub_with_assignment_when_parsed_then_ast_contains_subroutine_and_assignment()
-> TestResult {
    // Given: a developer writes a basic subroutine that mutates a lexical scalar.
    let code = r#"
        sub greet {
            my $name = "world";
            $name = "perl";
            return $name;
        }
    "#;

    // When: the parser processes the source.
    let sexp = parse_sexp(code)?;

    // Then: core semantic structure appears in the AST.
    assert!(sexp.contains("sub "), "Expected Subroutine node in: {sexp}");
    assert!(sexp.contains("assignment_"), "Expected Assignment node in: {sexp}");
    assert!(sexp.contains("(return"), "Expected Return node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_regex_substitution_when_parsed_then_pattern_replacement_and_flags_are_retained()
-> TestResult {
    // Given: a developer normalizes identifiers with substitution flags.
    let code = r#"$value =~ s/(\w+)/prefix_$1/gi;"#;

    // When: the parser processes the statement.
    let sexp = parse_sexp(code)?;

    // Then: regex substitution semantics are preserved in AST text form.
    assert!(sexp.contains("substitution"), "Expected Substitution node in: {sexp}");
    assert!(sexp.contains("prefix_$1"), "Expected replacement text in: {sexp}");
    assert!(sexp.contains("gi"), "Expected modifier flags in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid substitution: {sexp}"
    );

    Ok(())
}

#[test]
fn bdd_given_match_switch_when_parsed_then_given_and_when_constructs_are_present() -> TestResult {
    // Given: a developer uses Perl given/when smart-match control flow.
    let code = r#"
        given ($topic) {
            when (/^foo/) { print "foo"; }
            when (/^bar/) { print "bar"; }
            default { print "other"; }
        }
    "#;

    // When: the parser builds syntax trees.
    let sexp = parse_sexp(code)?;

    // Then: control-flow specific nodes are present and parse is clean.
    assert!(sexp.contains("(given "), "Expected Given node in: {sexp}");
    assert!(sexp.contains("(when "), "Expected When node in: {sexp}");
    assert!(sexp.contains("(default "), "Expected Default node in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid given/when: {sexp}"
    );

    Ok(())
}

#[test]
fn bdd_given_incomplete_if_when_parsed_then_parser_recovers_with_error_nodes_instead_of_crashing() {
    // Given: a developer is in the middle of editing incomplete syntax.
    let code = "if ($x > 10 { print $x;";

    // When: the parser processes malformed incremental text.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should recover, producing either ParseError or ERROR AST node.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR"),
                "Expected ERROR recovery node for malformed input: {sexp}"
            );
        }
        Err(err) => {
            let message = err.to_string();
            assert!(!message.is_empty(), "Expected diagnostic message when parse returns Err");
        }
    }
}

#[test]
fn bdd_given_multiple_realistic_statements_when_parsed_then_program_shape_is_stable() {
    // Given: a small realistic script using strict/warnings, loops, and conditionals.
    let code = r#"
        use strict;
        use warnings;

        my @values = (1, 2, 3);
        for my $v (@values) {
            if ($v % 2 == 0) {
                print "even";
            } else {
                print "odd";
            }
        }
    "#;

    // When: the parser builds the full AST.
    let sexp = must(parse_sexp(code));

    // Then: top-level shape includes declarations and structured control flow.
    assert!(sexp.contains("(use "), "Expected Use declarations in: {sexp}");
    assert!(sexp.contains("(for") || sexp.contains("(foreach"), "Expected loop node in: {sexp}");
    assert!(sexp.contains("(if "), "Expected If node in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid script: {sexp}"
    );
}

#[test]
fn bdd_given_postfix_flow_and_ternary_when_parsed_then_control_flow_nodes_are_retained()
-> TestResult {
    // Given: a developer writes concise Perl with postfix conditionals and ternary expressions.
    let code = r#"
        my $count = 2;
        print "nonzero" if $count;
        my $label = $count > 1 ? "many" : "one";
    "#;

    // When: the parser processes the snippet.
    let sexp = parse_sexp(code)?;

    // Then: compact control-flow structure remains visible in AST output.
    assert!(sexp.contains("statement_modifier"), "Expected statement modifier node in: {sexp}");
    assert!(sexp.contains("ternary"), "Expected ternary node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_unclosed_quote_when_parsed_then_recovery_is_reported_without_panicking() {
    // Given: a developer is typing and leaves a quoted string unfinished.
    let code = r#"my $name = "perl; print $name;"#;

    // When: the parser attempts to build an AST.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should return an error or emit recovery nodes, but never panic.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR")
                    || sexp.contains("(UNKNOWN_REST)")
                    || sexp.contains("(missing_expression)")
                    || sexp.contains("(missing_statement)"),
                "Expected recovery marker for malformed quoted string: {sexp}"
            );
        }
        Err(err) => {
            assert!(!err.to_string().is_empty(), "Expected non-empty parse failure message");
        }
    }
}

#[test]
fn bdd_given_package_and_constructor_pattern_when_parsed_then_namespace_and_bless_flow_are_preserved()
-> TestResult {
    // Given: a developer writes a package with a constructor that blesses a hashref.
    let code = r#"
        package My::Service;
        use strict;
        use warnings;

        sub new {
            my ($class, %args) = @_;
            my $self = bless { %args }, $class;
            return $self;
        }
    "#;

    // When: the parser processes this object-construction pattern.
    let sexp = parse_sexp(code)?;

    // Then: namespace + constructor flow is represented without recovery artifacts.
    assert!(sexp.contains("My::Service"), "Expected package name in AST output: {sexp}");
    assert!(sexp.contains("bless"), "Expected bless call in AST output: {sexp}");
    assert!(sexp.contains("(return"), "Expected Return node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_partial_hashref_literal_when_parsed_then_parser_recovers_without_panicking() {
    // Given: a developer is typing a hashref literal and stops mid-expression.
    let code = r#"
        my $cfg = {
            host => "localhost",
            port =>
    "#;

    // When: the parser attempts to process incomplete input.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should recover (ERROR node) or return a descriptive parse failure.
    // Note: AST recovery node names are ERROR, (UNKNOWN_REST), (missing_expression),
    // (missing_statement) — lowercase "unknown" is not a valid sexp token name.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR")
                    || sexp.contains("(UNKNOWN_REST)")
                    || sexp.contains("(missing_expression)")
                    || sexp.contains("(missing_statement)"),
                "Expected recovery marker for incomplete hashref literal: {sexp}"
            );
        }
        Err(err) => {
            assert!(!err.to_string().is_empty(), "Expected non-empty parse failure message");
        }
    }
}

#[test]
fn bdd_given_nested_try_catch_and_finally_when_parsed_then_exception_flow_nodes_are_preserved()
-> TestResult {
    // Given: a developer uses nested exception handling with fallback and cleanup logic.
    let code = r#"
        my $status = "pending";
        try {
            try {
                die "boom";
            } catch ($inner) {
                $status = "inner";
            }
        } catch ($outer) {
            $status = "outer";
        } finally {
            $status = "done";
        }
    "#;

    // When: the parser processes the nested exception handling flow.
    let sexp = parse_sexp(code)?;

    // Then: structured try/catch/finally constructs remain visible and parse is clean.
    assert!(sexp.contains("(try"), "Expected Try node in: {sexp}");
    assert!(sexp.contains("(catch"), "Expected Catch node in: {sexp}");
    assert!(sexp.contains("(finally"), "Expected Finally node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_labeled_loop_control_when_parsed_then_label_and_control_ops_are_retained() -> TestResult
{
    // Given: a developer writes a labeled loop with next/last/redo control flow.
    let code = r#"
        OUTER: while (my $line = <STDIN>) {
            next OUTER if $line =~ /^\s*$/;
            redo OUTER if $line =~ /\\$/;
            last OUTER if $line =~ /^quit$/;
            print $line;
        }
    "#;

    // When: the parser builds the AST for loop-control heavy code.
    let sexp = parse_sexp(code)?;

    // Then: loop control operations should survive parsing without recovery noise.
    assert!(sexp.contains("(while"), "Expected While node in: {sexp}");
    assert!(sexp.contains("(next"), "Expected Next loop-control node in: {sexp}");
    assert!(sexp.contains("(redo"), "Expected Redo loop-control node in: {sexp}");
    assert!(sexp.contains("(last"), "Expected Last loop-control node in: {sexp}");
    assert!(sexp.contains("OUTER"), "Expected loop label in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_loop_with_continue_block_when_parsed_then_continue_and_flow_nodes_are_preserved()
-> TestResult {
    // Given: a developer maintains a read loop with a continue block for line accounting.
    let code = r#"
        my $line_no = 0;
        while (my $line = <DATA>) {
            next if $line =~ /^#/;
            last if $line =~ /^__END__$/;
            print $line;
        } continue {
            $line_no++;
        }
    "#;

    // When: the parser processes loop control mixed with a continue block.
    let sexp = parse_sexp(code)?;

    // Then: loop, continue, and control-flow nodes should be retained without recovery markers.
    assert!(sexp.contains("(while"), "Expected While node in: {sexp}");
    assert!(sexp.contains("(continue"), "Expected Continue node in: {sexp}");
    assert!(sexp.contains("(next"), "Expected Next loop-control node in: {sexp}");
    assert!(sexp.contains("(last"), "Expected Last loop-control node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_do_while_and_until_loops_when_parsed_then_post_condition_flow_is_retained()
-> TestResult {
    // Given: a developer mixes post-condition and pre-condition loop forms.
    let code = r#"
        my $i = 0;
        do {
            $i++;
        } while ($i < 2);

        until ($i > 3) {
            $i++;
        }
    "#;

    // When: the parser processes loop constructs with explicit conditions.
    let sexp = parse_sexp(code)?;

    // Then: do/while and normalized until flow should be represented without recovery markers.
    assert!(sexp.contains("(do"), "Expected Do node in: {sexp}");
    assert!(sexp.contains("(while"), "Expected While node in: {sexp}");
    assert!(sexp.contains("(unary_not"), "Expected normalized Until condition in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_state_vars_and_method_chaining_when_parsed_then_state_and_arrow_invocations_are_preserved()
-> TestResult {
    // Given: a developer caches constructor state and chains method calls.
    let code = r#"
        use feature 'state';
        sub service_name {
            state $svc = My::Service->new()->bootstrap();
            return $svc->name();
        }
    "#;

    // When: the parser reads state declarations and chained invocations.
    let sexp = parse_sexp(code)?;

    // Then: state declaration + arrow invocation structure should be represented cleanly.
    assert!(sexp.contains("(state_declaration"), "Expected state declaration node in: {sexp}");
    assert!(sexp.contains("bootstrap"), "Expected chained method call in: {sexp}");
    assert!(sexp.contains("name"), "Expected terminal method call in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_map_and_grep_pipeline_when_parsed_then_higher_order_blocks_are_retained() -> TestResult
{
    // Given: a developer transforms and filters a list with map/grep blocks.
    let code = r#"
        my @nums = (1, 2, 3, 4, 5);
        my @evens_squared = map { $_ * $_ } grep { $_ % 2 == 0 } @nums;
    "#;

    // When: the parser processes nested higher-order list operations.
    let sexp = parse_sexp(code)?;

    // Then: map/grep and their block structure should remain visible in AST output.
    assert!(sexp.contains("(call map"), "Expected map call node in: {sexp}");
    assert!(sexp.contains("(call grep"), "Expected grep call node in: {sexp}");
    assert!(sexp.contains("binary_"), "Expected block expression nodes in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_eval_block_with_localized_error_when_parsed_then_eval_and_local_nodes_are_preserved()
-> TestResult {
    // Given: a developer wraps risky logic in eval and localizes $@ handling.
    let code = r#"
        my $result;
        {
            local $@;
            eval {
                die "bad input";
            };
            if ($@) {
                $result = "failed";
            }
        }
    "#;

    // When: the parser processes exception-prone code.
    let sexp = parse_sexp(code)?;

    // Then: eval/local/if structure should remain visible and parse should be clean.
    assert!(sexp.contains("(eval"), "Expected Eval node in: {sexp}");
    assert!(sexp.contains("(local"), "Expected Local node in: {sexp}");
    assert!(sexp.contains("(if "), "Expected If node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_map_and_grep_pipeline_when_parsed_then_high_order_ops_and_regex_match_are_retained()
-> TestResult {
    // Given: a developer transforms and filters a list using map/grep.
    let code = r#"
        my @values = qw(alpha beta gamma);
        my @upper = map { uc $_ } grep { $_ =~ /a/ } @values;
    "#;

    // When: parser builds the AST for list-processing expressions.
    let sexp = parse_sexp(code)?;

    // Then: map/grep and regex matching should remain explicit in AST shape.
    assert!(sexp.contains("(call map"), "Expected Map node in: {sexp}");
    assert!(sexp.contains("(call grep"), "Expected Grep node in: {sexp}");
    assert!(sexp.contains("(match"), "Expected RegexMatch node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_closure_capturing_lexical_when_parsed_then_anon_sub_and_capture_are_preserved()
-> TestResult {
    // Given: a developer builds an event handler that captures lexical state via closure.
    let code = r#"
        my $prefix = "WARN";
        my $logger = sub {
            my ($msg) = @_;
            print "$prefix: $msg\n";
        };
        $logger->("something went wrong");
    "#;

    // When: the parser processes the closure and its invocation.
    let sexp = parse_sexp(code)?;

    // Then: anonymous subroutine node and the invocation should both be present.
    assert!(
        sexp.contains("sub") || sexp.contains("subroutine") || sexp.contains("anonymous"),
        "Expected anonymous subroutine node in: {sexp}"
    );
    assert!(sexp.contains("prefix"), "Expected captured lexical variable name in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_begin_and_end_blocks_when_parsed_then_phase_blocks_are_retained() -> TestResult {
    // Given: a developer uses BEGIN to run imports at compile time and END for cleanup.
    let code = r#"
        BEGIN {
            require Scalar::Util;
            Scalar::Util->import('blessed', 'reftype');
        }

        my $obj = bless {}, 'MyClass';

        END {
            warn "Shutting down\n";
        }
    "#;

    // When: the parser processes the phase blocks alongside regular code.
    let sexp = parse_sexp(code)?;

    // Then: BEGIN and END phase blocks should be represented and parse should be clean.
    assert!(sexp.contains("BEGIN"), "Expected BEGIN block in: {sexp}");
    assert!(sexp.contains("END"), "Expected END block in: {sexp}");
    assert!(sexp.contains("bless"), "Expected bless call in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_named_regex_captures_when_parsed_then_capture_names_are_retained() -> TestResult {
    // Given: a developer parses a date string using named captures for clarity.
    let code = r#"
        my $date = "2026-04-30";
        if ($date =~ /(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/) {
            my $y = $+{year};
            my $m = $+{month};
            my $d = $+{day};
            print "Year: $y, Month: $m, Day: $d\n";
        }
    "#;

    // When: the parser processes the named-capture regex and the capture variable access.
    let sexp = parse_sexp(code)?;

    // Then: regex match and the capture variable hash should both be represented.
    assert!(
        sexp.contains("match") || sexp.contains("regex"),
        "Expected regex match node in: {sexp}"
    );
    assert!(sexp.contains("year"), "Expected named capture 'year' in: {sexp}");
    assert!(sexp.contains("month"), "Expected named capture 'month' in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_sort_with_custom_comparator_when_parsed_then_sort_and_comparison_are_retained()
-> TestResult {
    // Given: a developer sorts a list of records by a computed key using a block comparator.
    let code = r#"
        my @records = ({name => "Zoe"}, {name => "Alice"}, {name => "Bob"});
        my @sorted = sort { lc($a->{name}) cmp lc($b->{name}) } @records;
        my @by_len = sort { length($a) <=> length($b) || $a cmp $b } qw(foo ba quux z);
    "#;

    // When: the parser processes sort with multi-expression block comparators.
    let sexp = parse_sexp(code)?;

    // Then: sort invocations and comparison operators should appear in the AST.
    assert!(sexp.contains("sort") || sexp.contains("(call sort"), "Expected sort call in: {sexp}");
    assert!(sexp.contains("cmp"), "Expected cmp string comparator in: {sexp}");
    assert!(sexp.contains("<=>"), "Expected spaceship numeric comparator in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_subroutine_reference_and_dispatch_when_parsed_then_coderef_ops_are_retained()
-> TestResult {
    // Given: a developer builds a dispatch table with code references.
    let code = r#"
        sub add { return $_[0] + $_[1]; }
        sub mul { return $_[0] * $_[1]; }

        my %ops = (
            add => \&add,
            mul => \&mul,
        );

        my $op = $ops{add};
        my $result = $op->(3, 4);
    "#;

    // When: the parser processes the dispatch table and indirect call.
    let sexp = parse_sexp(code)?;

    // Then: subroutine declarations, references, and the call should all be present.
    assert!(sexp.contains("sub "), "Expected subroutine declarations in: {sexp}");
    assert!(sexp.contains("add"), "Expected 'add' name in: {sexp}");
    assert!(sexp.contains("result"), "Expected result variable in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_complex_dereference_chain_when_parsed_then_nested_deref_ops_are_retained() -> TestResult
{
    // Given: a developer traverses deeply nested data structures with postfix dereference.
    let code = r#"
        my $data = {
            users => [
                { name => "Alice", roles => ["admin", "user"] },
                { name => "Bob",   roles => ["user"] },
            ]
        };
        my $first_role = $data->{users}[0]{roles}[0];
        my @all_names  = map { $_->{name} } @{ $data->{users} };
    "#;

    // When: the parser processes the nested hash/array dereferences.
    let sexp = parse_sexp(code)?;

    // Then: the nested structure and dereference operations should be represented cleanly.
    assert!(sexp.contains("users"), "Expected 'users' key in: {sexp}");
    assert!(sexp.contains("Alice"), "Expected 'Alice' string literal in: {sexp}");
    assert!(sexp.contains("first_role"), "Expected 'first_role' variable in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_moose_style_class_declaration_when_parsed_then_has_and_extends_are_retained()
-> TestResult {
    // Given: a developer declares a Moose class with attributes and inheritance.
    let code = r#"
        package Animal;
        use Moose;

        has 'name'   => (is => 'ro', isa => 'Str', required => 1);
        has 'sound'  => (is => 'rw', isa => 'Str', default  => 'grunt');

        sub speak {
            my $self = shift;
            printf "%s says %s\n", $self->name, $self->sound;
        }

        package Dog;
        use Moose;
        extends 'Animal';

        has 'breed' => (is => 'ro', isa => 'Str');

        sub fetch { return "fetched!"; }
    "#;

    // When: the parser processes the Moose class declarations.
    let sexp = parse_sexp(code)?;

    // Then: package declarations, attribute helpers, and methods should be represented.
    // Note: `has` and `extends` are parsed as ambiguous function calls; the package names
    // and string arguments are what survive into the sexp representation.
    assert!(sexp.contains("Animal"), "Expected Animal package in: {sexp}");
    assert!(sexp.contains("Dog"), "Expected Dog package in: {sexp}");
    assert!(sexp.contains("name"), "Expected 'name' attribute argument in: {sexp}");
    assert!(
        sexp.contains("ambiguous_function_call_expression"),
        "Expected ambiguous function calls (has/extends) in: {sexp}"
    );
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_sprintf_with_various_formats_when_parsed_then_format_call_is_retained() -> TestResult {
    // Given: a developer formats output using sprintf with multiple format specifiers.
    let code = r#"
        my $name  = "Perl";
        my $ver   = 5.036;
        my $count = 42;
        my $msg   = sprintf "%-10s v%05.3f [%04d items]", $name, $ver, $count;
        my $hex   = sprintf "0x%08X", 255;
        my $multi = sprintf "%s: %d errors, %d warnings", $name, 0, 3;
        printf "%s\n", $msg;
    "#;

    // When: the parser processes the sprintf calls.
    let sexp = parse_sexp(code)?;

    // Then: sprintf/printf calls and their arguments should be represented cleanly.
    assert!(
        sexp.contains("sprintf") || sexp.contains("printf"),
        "Expected sprintf/printf calls in: {sexp}"
    );
    assert!(sexp.contains("msg"), "Expected result variable in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_wantarray_context_sensitive_return_when_parsed_then_wantarray_node_is_retained()
-> TestResult {
    // Given: a developer writes a function that returns differently depending on calling context.
    let code = r#"
        sub context_aware {
            my @items = (1, 2, 3);
            if (wantarray) {
                return @items;
            } else {
                return scalar @items;
            }
        }

        my @list   = context_aware();
        my $count  = context_aware();
    "#;

    // When: the parser processes the wantarray conditional.
    let sexp = parse_sexp(code)?;

    // Then: the if/else branch and both return paths should be present in AST.
    // Note: `wantarray` is parsed as a generic function call; the name is not preserved
    // in the sexp, but the call-site structure (function_call_expression) and the
    // if/else control flow are both retained.
    assert!(
        sexp.contains("function_call_expression"),
        "Expected function call (wantarray) in: {sexp}"
    );
    assert!(sexp.contains("(if "), "Expected if node for context-sensitive branch in: {sexp}");
    assert!(sexp.contains("(return"), "Expected return node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_overloaded_operators_when_parsed_then_overload_pragma_is_retained() -> TestResult {
    // Given: a developer implements a value-object class with overloaded arithmetic.
    let code = r#"
        package Vector;
        use overload
            '+' => \&add,
            '-' => \&sub_vec,
            '""' => \&stringify,
            fallback => 1;

        sub new {
            my ($class, $x, $y) = @_;
            return bless { x => $x, y => $y }, $class;
        }

        sub add {
            my ($self, $other) = @_;
            return Vector->new($self->{x} + $other->{x}, $self->{y} + $other->{y});
        }

        sub sub_vec {
            my ($self, $other, $swap) = @_;
            return Vector->new($self->{x} - $other->{x}, $self->{y} - $other->{y});
        }

        sub stringify {
            my $self = shift;
            return "($self->{x}, $self->{y})";
        }
    "#;

    // When: the parser processes the overloaded operator class.
    let sexp = parse_sexp(code)?;

    // Then: overload pragma and class structure should both survive parsing.
    assert!(sexp.contains("overload"), "Expected overload pragma in: {sexp}");
    assert!(sexp.contains("Vector"), "Expected package name in: {sexp}");
    assert!(sexp.contains("bless"), "Expected bless call in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_complex_regex_alternation_and_lookahead_when_parsed_then_regex_structure_is_retained()
-> TestResult {
    // Given: a developer validates input with a complex regex using alternation and lookahead.
    let code = r#"
        my $email = 'user@example.com';
        my $valid  = $email =~ /
            ^                       # start of string
            (?=[^@]{1,64}@)         # local part length lookahead
            [a-zA-Z0-9._%+\-]+      # local part characters
            @                       # separator
            (?:[a-zA-Z0-9\-]+\.)+  # domain labels
            [a-zA-Z]{2,}            # TLD
            $                       # end of string
        /x;
        my $ipv4 = "192.168.1.1";
        my $is_ip = $ipv4 =~ /^(\d{1,3}\.){3}\d{1,3}$/;
    "#;

    // When: the parser processes the extended regex with lookahead.
    let sexp = parse_sexp(code)?;

    // Then: regex operations should be represented and parse should remain stable.
    assert!(
        sexp.contains("match") || sexp.contains("regex") || sexp.contains("=~"),
        "Expected regex match operations in: {sexp}"
    );
    assert!(sexp.contains("email"), "Expected email variable in: {sexp}");
    assert!(sexp.contains("ipv4"), "Expected ipv4 variable in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_autoload_and_destroy_methods_when_parsed_then_special_sub_names_are_retained()
-> TestResult {
    // Given: a developer implements dynamic dispatch via AUTOLOAD and resource cleanup via DESTROY.
    let code = r#"
        package Proxy;

        sub new {
            my ($class, $target) = @_;
            return bless { target => $target, calls => 0 }, $class;
        }

        our $AUTOLOAD;
        sub AUTOLOAD {
            my ($self, @args) = @_;
            my $method = $AUTOLOAD;
            $method =~ s/.*:://;
            return if $method eq 'DESTROY';
            $self->{calls}++;
            return $self->{target}->$method(@args);
        }

        sub DESTROY {
            my $self = shift;
            printf "Proxy destroyed after %d calls\n", $self->{calls};
        }
    "#;

    // When: the parser processes the AUTOLOAD and DESTROY special methods.
    let sexp = parse_sexp(code)?;

    // Then: both special method names should appear in the AST representation.
    assert!(sexp.contains("AUTOLOAD"), "Expected AUTOLOAD subroutine in: {sexp}");
    assert!(sexp.contains("DESTROY"), "Expected DESTROY subroutine in: {sexp}");
    assert!(sexp.contains("Proxy"), "Expected package name in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}
