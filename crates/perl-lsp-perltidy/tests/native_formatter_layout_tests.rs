use perl_lsp_perltidy::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, KeywordSpacing, NativeFormatter,
    PerlFormatter, TextPosition, TextRange, TrailingComma,
};

#[test]
fn native_formatter_formats_simple_lexical_declarations() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\nour @y;\nstate %z;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1;\nour @y;\nstate %z;\n");
    assert_eq!(result.edits.len(), 1);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_lexical_declaration_lists() {
    let formatter = NativeFormatter::new();
    let source = "my($x,$y)=($a,$b);\nour($left,@right);\nstate($count,%seen)=seed();\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my ($x, $y) = ($a, $b);\nour ($left, @right);\nstate ($count, %seen) = seed();\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_binary_expressions() {
    let formatter = NativeFormatter::new();
    let source = "my$x=$y+1;\nreturn$x*2;\nif($x==2){return$y+1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my $x = $y + 1;\nreturn $x * 2;\nif ($x == 2) {\n    return $y + 1;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_assignments() {
    let formatter = NativeFormatter::new();
    let source = "$x=1;\n$y=$x+2;\nsub bump{$x=$x+1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "$x = 1;\n$y = $x + 2;\nsub bump {\n    $x = $x + 1;\n    return $x;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_call_expressions() {
    let formatter = NativeFormatter::new();
    let source = "my$x=foo($y,1);\n$z=bar();\nreturn baz($x,$z);\nfoo($x,bar());\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my $x = foo($y, 1);\n$z = bar();\nreturn baz($x, $z);\nfoo($x, bar());\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_list_literals() {
    let formatter = NativeFormatter::new();
    let source = "my@xs=(1,2,$y);\n$x=(foo(1),bar($y));\nreturn($x,3);\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my @xs = (1, 2, $y);\n$x = (foo(1), bar($y));\nreturn ($x, 3);\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_hash_constructors() {
    let formatter = NativeFormatter::new();
    let source = "my$h={foo=>1,bar=>$x};\n$x={nested=>{ok=>1},list=>(1,2)};\nreturn{answer=>42};\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my $h = {foo => 1, bar => $x};\n$x = {nested => {ok => 1}, list => (1, 2)};\nreturn {answer => 42};\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_module_declarations() {
    let formatter = NativeFormatter::new();
    let source =
        "package Foo::Bar ;\nuse strict ;\nno warnings ;\nrequire Foo::Bar ;\nuse lib 'lib';\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "package Foo::Bar;\nuse strict;\nno warnings;\nrequire Foo::Bar;\nuse lib 'lib';\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_method_calls() {
    let formatter = NativeFormatter::new();
    let source = "$x=$obj->build();\n$z=$obj->empty();\nreturn $obj->wrap(foo(1),{ok=>1});\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "$x = $obj->build();\n$z = $obj->empty();\nreturn $obj->wrap(foo(1), {ok => 1});\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_method_call_chains() {
    let formatter = NativeFormatter::new();
    let source = "$x=$obj->build()->name();\nreturn $obj->find($id)->wrap(foo(1),{ok=>1});\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "$x = $obj->build()->name();\nreturn $obj->find($id)->wrap(foo(1), {ok => 1});\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_wraps_simple_calls_lists_and_hashes_by_line_width() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { line_width: 18, indent_width: 2, ..FormatConfig::default() };
    let source = concat!(
        "my$result=foo($alpha,$beta,$gamma);\n",
        "my@items=($alpha,$beta,$gamma);\n",
        "my$hash={alpha=>$alpha,beta=>$beta};\n",
        "return $object->wrap($alpha,$beta,$gamma);\n",
    );

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "my $result = foo(\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma\n",
            ");\n",
            "my @items = (\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma\n",
            ");\n",
            "my $hash = {\n",
            "  alpha => $alpha,\n",
            "  beta => $beta\n",
            "};\n",
            "return $object->wrap(\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma\n",
            ");\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_adds_trailing_commas_only_when_wrapped_and_configured() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig {
        line_width: 18,
        indent_width: 2,
        trailing_comma: TrailingComma::AddWhenWrapped,
        ..FormatConfig::default()
    };
    let source = concat!(
        "my$result=foo($alpha,$beta,$gamma);\n",
        "my@items=($alpha,$beta,$gamma);\n",
        "my$hash={alpha=>$alpha,beta=>$beta};\n",
        "return foo($a,$b);\n",
    );

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "my $result = foo(\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma,\n",
            ");\n",
            "my @items = (\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma,\n",
            ");\n",
            "my $hash = {\n",
            "  alpha => $alpha,\n",
            "  beta => $beta,\n",
            "};\n",
            "return foo($a, $b);\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_wraps_delimited_expression_when_statement_prefix_exceeds_width() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { line_width: 30, indent_width: 2, ..FormatConfig::default() };
    let source = "my$long_variable_name=foo($a,$b);\nreturn foo($a,$b);\n";

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "my $long_variable_name = foo(\n",
            "  $a,\n",
            "  $b\n",
            ");\n",
            "return foo($a, $b);\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_preserves_indent_and_line_endings_for_simple_declarations() {
    let formatter = NativeFormatter::new();
    let source = "  my $x=1;\r\n\tour @y;\r\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert_eq!(result.formatted, "  my $x = 1;\r\n\tour @y;\r\n");
}

#[test]
fn native_formatter_is_idempotent_for_simple_lexical_layout() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\n";

    let once = formatter.format_document(source, &FormatConfig::default());
    let twice = formatter.format_document(&once.formatted, &FormatConfig::default());

    assert_eq!(once.formatted, twice.formatted);
    assert!(!twice.changed);
}

#[test]
fn native_formatter_keeps_unsupported_lines_unchanged() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\nprint$x;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert_eq!(result.formatted, "my $x = 1;\nprint$x;\n");
}

#[test]
fn native_formatter_preserves_trailing_comment_while_formatting_simple_statement() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1; # keep this exact comment\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1; # keep this exact comment\n");
}

#[test]
fn native_formatter_formats_simple_trailing_comment_matrix() {
    let formatter = NativeFormatter::new();
    let source = concat!(
        "# leading file comment\n",
        "my$x=1; # trailing assignment comment\n",
        "if($x){ # trailing block opener comment\n",
        "    return$x; # trailing return comment\n",
        "}\n",
    );

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "# leading file comment\n",
            "my $x = 1; # trailing assignment comment\n",
            "if($x){ # trailing block opener comment\n",
            "    return $x; # trailing return comment\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_does_not_treat_hash_inside_strings_as_trailing_comment() {
    let formatter = NativeFormatter::new();
    let source = "my$msg=\"#not a comment\";\nreturn\"#value\";\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $msg = \"#not a comment\";\nreturn \"#value\";\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_combines_simple_layout_with_final_newline_policy() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { final_newline: FinalNewline::Insert, ..FormatConfig::default() };

    let result = formatter.format_document("my $x=1;", &config);

    assert_eq!(result.formatted, "my $x = 1;\n");
}

#[test]
fn native_range_formatter_formats_only_selected_simple_declaration_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nmy$y=2;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 7));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nmy $y = 2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(
        result.edits[0].range,
        TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 7))
    );
    assert_eq!(result.edits[0].new_text, "my $y = 2;");
}

#[test]
fn native_range_formatter_preserves_trailing_comment_on_selected_simple_statement_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1; # keep\nmy$y=2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 14));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1; # keep\nmy$y=2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my $x = 1; # keep");
}

#[test]
fn native_range_formatter_formats_selected_simple_lexical_declaration_list_line() {
    let formatter = NativeFormatter::new();
    let source = "my($x,$y)=($a,$b);\nmy$z=1;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 18));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my ($x, $y) = ($a, $b);\nmy$z=1;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my ($x, $y) = ($a, $b);");
}

#[test]
fn native_range_formatter_formats_selected_simple_binary_expression_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=$y+1;\nreturn$x*2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 10));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = $y + 1;\nreturn$x*2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my $x = $y + 1;");
}

#[test]
fn native_range_formatter_formats_selected_simple_assignment_line() {
    let formatter = NativeFormatter::new();
    let source = "$x=1;\n$y=$x+2;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 8));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "$x=1;\n$y = $x + 2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "$y = $x + 2;");
}

#[test]
fn native_range_formatter_formats_selected_simple_call_expression_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=foo($y,1);\nreturn baz($x,$y);\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 15));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = foo($y, 1);\nreturn baz($x,$y);\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my $x = foo($y, 1);");
}

#[test]
fn native_range_formatter_formats_selected_simple_list_literal_line() {
    let formatter = NativeFormatter::new();
    let source = "my@xs=(1,2,$y);\nreturn($x,3);\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 13));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my@xs=(1,2,$y);\nreturn ($x, 3);\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "return ($x, 3);");
}

#[test]
fn native_range_formatter_formats_selected_simple_hash_constructor_line() {
    let formatter = NativeFormatter::new();
    let source = "my$h={foo=>1,bar=>$x};\nreturn{answer=>42};\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 19));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$h={foo=>1,bar=>$x};\nreturn {answer => 42};\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "return {answer => 42};");
}

#[test]
fn native_range_formatter_formats_selected_simple_module_declaration_line() {
    let formatter = NativeFormatter::new();
    let source = "package Foo::Bar ;\nuse strict ;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 12));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "package Foo::Bar ;\nuse strict;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "use strict;");
}

#[test]
fn native_range_formatter_formats_selected_simple_method_call_line() {
    let formatter = NativeFormatter::new();
    let source = "$x=$obj->empty();\nreturn $obj->build(1,$y);\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 25));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "$x=$obj->empty();\nreturn $obj->build(1, $y);\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "return $obj->build(1, $y);");
}

#[test]
fn native_range_formatter_formats_selected_simple_method_chain_line() {
    let formatter = NativeFormatter::new();
    let source = "$x=$obj->build()->name();\nreturn $obj->find($id)->wrap({ok=>1});\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 38));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "$x=$obj->build()->name();\nreturn $obj->find($id)->wrap({ok => 1});\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "return $obj->find($id)->wrap({ok => 1});");
}

#[test]
fn native_range_formatter_wraps_selected_simple_call_line_by_width() {
    let formatter = NativeFormatter::new();
    let source = "$x=1;\nreturn $object->wrap($alpha,$beta,$gamma);\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 42));
    let config = FormatConfig { line_width: 18, indent_width: 2, ..FormatConfig::default() };

    let result = formatter.format_range(source, range, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "$x=1;\n",
            "return $object->wrap(\n",
            "  $alpha,\n",
            "  $beta,\n",
            "  $gamma\n",
            ");\n",
        )
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(
        result.edits[0].new_text,
        "return $object->wrap(\n  $alpha,\n  $beta,\n  $gamma\n);"
    );
}

#[test]
fn native_range_formatter_treats_end_line_at_character_zero_as_exclusive() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nmy$y=2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(1, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert_eq!(result.formatted, "my $x = 1;\nmy$y=2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].new_text, "my $x = 1;");
}

#[test]
fn native_formatter_formats_compact_keyword_variable_boundary_when_tokenized_safely() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1;\n");
}

#[test]
fn native_formatter_expands_simple_subroutine_blocks() {
    let formatter = NativeFormatter::new();
    let source = "sub answer{my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "sub answer {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_places_opening_braces_on_next_line_when_configured() {
    let formatter = NativeFormatter::new();
    let config =
        FormatConfig { brace_placement: BracePlacement::NextLine, ..FormatConfig::default() };
    let source = "sub answer{return 1;}\nif($ok){return 1;}else{return 0;}\nwhile($ok){next;}continue{last;}\n";

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "sub answer\n",
            "{\n",
            "    return 1;\n",
            "}\n",
            "if ($ok)\n",
            "{\n",
            "    return 1;\n",
            "} else\n",
            "{\n",
            "    return 0;\n",
            "}\n",
            "while ($ok)\n",
            "{\n",
            "    next;\n",
            "} continue\n",
            "{\n",
            "    last;\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_places_else_tails_on_separate_lines_when_configured() {
    let formatter = NativeFormatter::new();
    let config =
        FormatConfig { else_placement: ElsePlacement::SeparateLine, ..FormatConfig::default() };
    let source = "if($a){return 1;}elsif($b){return 2;}else{return 3;}\nunless($ok){return 0;}else{return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "if ($a) {\n",
            "    return 1;\n",
            "}\n",
            "elsif ($b) {\n",
            "    return 2;\n",
            "}\n",
            "else {\n",
            "    return 3;\n",
            "}\n",
            "unless ($ok) {\n",
            "    return 0;\n",
            "}\n",
            "else {\n",
            "    return 1;\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_compacts_keyword_condition_spacing_when_configured() {
    let formatter = NativeFormatter::new();
    let config =
        FormatConfig { keyword_spacing: KeywordSpacing::Compact, ..FormatConfig::default() };
    let source =
        "if($a){return 1;}elsif($b){return 2;}else{return 3;}\nwhile($ok){next;}continue{last;}\n";

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "if($a) {\n",
            "    return 1;\n",
            "} elsif($b) {\n",
            "    return 2;\n",
            "} else {\n",
            "    return 3;\n",
            "}\n",
            "while($ok) {\n",
            "    next;\n",
            "} continue {\n",
            "    last;\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_subroutine_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "sub answer{return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "sub answer {\n  return 1;\n}\n");
}

#[test]
fn native_formatter_expands_simple_if_blocks() {
    let formatter = NativeFormatter::new();
    let source = "if($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "if ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_if_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "  if($ok){return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "  if ($ok) {\n    return 1;\n  }\n");
}

#[test]
fn native_formatter_expands_simple_while_blocks() {
    let formatter = NativeFormatter::new();
    let source = "while($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "while ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_while_continue_blocks() {
    let formatter = NativeFormatter::new();
    let source = "while($ok){next;}continue{last;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "while ($ok) {\n    next;\n} continue {\n    last;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_while_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "  while($ok){return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "  while ($ok) {\n    return 1;\n  }\n");
}

#[test]
fn native_formatter_expands_simple_unless_blocks() {
    let formatter = NativeFormatter::new();
    let source = "unless($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "unless ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_until_blocks() {
    let formatter = NativeFormatter::new();
    let source = "until($done){return 1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "until ($done) {\n    return 1;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_foreach_blocks() {
    let formatter = NativeFormatter::new();
    let source = "foreach my$item(@items){return$item;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "foreach my $item (@items) {\n    return $item;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_loop_control_statements() {
    let formatter = NativeFormatter::new();
    let source = "foreach my$item(@items){next;last LOOP;redo;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "foreach my $item (@items) {\n    next;\n    last LOOP;\n    redo;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_foreach_continue_blocks() {
    let formatter = NativeFormatter::new();
    let source = "foreach my$item(@items){next;}continue{redo;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "foreach my $item (@items) {\n    next;\n} continue {\n    redo;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_for_foreach_alias_blocks() {
    let formatter = NativeFormatter::new();
    let source = "for$item(@items){return$item;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "for $item (@items) {\n    return $item;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_c_style_for_blocks() {
    let formatter = NativeFormatter::new();
    let source = "for(my$i=0;$i<3;$i++){next;}\nfor(;$ok;--$remaining){last;}\nfor(;;){redo;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "for (my $i = 0; $i < 3; $i++) {\n",
            "    next;\n",
            "}\n",
            "for (; $ok; --$remaining) {\n",
            "    last;\n",
            "}\n",
            "for (;;) {\n",
            "    redo;\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_c_style_for_continue_blocks() {
    let formatter = NativeFormatter::new();
    let source = "for(my$i=0;$i<3;$i++){next;}continue{tick($i);}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        concat!(
            "for (my $i = 0; $i < 3; $i++) {\n",
            "    next;\n",
            "} continue {\n",
            "    tick($i);\n",
            "}\n",
        )
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_if_else_blocks() {
    let formatter = NativeFormatter::new();
    let source = "if($ok){return 1;}else{return 0;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "if ($ok) {\n    return 1;\n} else {\n    return 0;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_if_elsif_else_blocks() {
    let formatter = NativeFormatter::new();
    let source = "if($ok){return 1;}elsif($maybe){return 2;}else{return 0;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "if ($ok) {\n    return 1;\n} elsif ($maybe) {\n    return 2;\n} else {\n    return 0;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_unless_else_blocks() {
    let formatter = NativeFormatter::new();
    let source = "unless($ok){return 0;}else{return 1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "unless ($ok) {\n    return 0;\n} else {\n    return 1;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_range_formatter_formats_selected_simple_subroutine_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nsub answer{our@y;return@y;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 27));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nsub answer {\n    our @y;\n    return @y;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "sub answer {\n    our @y;\n    return @y;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_if_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nif($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 18));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nif ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "if ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_while_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nwhile($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 21));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nwhile ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "while ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_while_continue_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nwhile($ok){next;}continue{last;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 32));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nwhile ($ok) {\n    next;\n} continue {\n    last;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "while ($ok) {\n    next;\n} continue {\n    last;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_unless_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nunless($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 22));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nunless ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "unless ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_foreach_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nforeach my$item(@items){return$item;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 37));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nforeach my $item (@items) {\n    return $item;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "foreach my $item (@items) {\n    return $item;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_loop_control_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nforeach my$item(@items){next;last LOOP;redo;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 45));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my$x=1;\nforeach my $item (@items) {\n    next;\n    last LOOP;\n    redo;\n}\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(
        result.edits[0].new_text,
        "foreach my $item (@items) {\n    next;\n    last LOOP;\n    redo;\n}"
    );
}

#[test]
fn native_range_formatter_formats_selected_simple_c_style_for_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nfor(my$i=0;$i<3;$i++){next;}\n";
    let range = TextRange {
        start: TextPosition { line: 1, character: 0 },
        end: TextPosition { line: 1, character: 28 },
    };

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nfor (my $i = 0; $i < 3; $i++) {\n    next;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].new_text, "for (my $i = 0; $i < 3; $i++) {\n    next;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_c_style_for_continue_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nfor(my$i=0;$i<3;$i++){next;}continue{tick($i);}\n";
    let range = TextRange {
        start: TextPosition { line: 1, character: 0 },
        end: TextPosition { line: 1, character: 47 },
    };

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my$x=1;\nfor (my $i = 0; $i < 3; $i++) {\n    next;\n} continue {\n    tick($i);\n}\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(
        result.edits[0].new_text,
        "for (my $i = 0; $i < 3; $i++) {\n    next;\n} continue {\n    tick($i);\n}"
    );
}

#[test]
fn native_range_formatter_formats_selected_simple_if_else_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nif($ok){return 1;}else{return 0;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 33));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my$x=1;\nif ($ok) {\n    return 1;\n} else {\n    return 0;\n}\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "if ($ok) {\n    return 1;\n} else {\n    return 0;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_if_elsif_else_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nif($ok){return 1;}elsif($maybe){return 2;}else{return 0;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 57));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my$x=1;\nif ($ok) {\n    return 1;\n} elsif ($maybe) {\n    return 2;\n} else {\n    return 0;\n}\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(
        result.edits[0].new_text,
        "if ($ok) {\n    return 1;\n} elsif ($maybe) {\n    return 2;\n} else {\n    return 0;\n}"
    );
}
