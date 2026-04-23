use perl_parser_core::Parser;

fn parse_clean(src: &str) -> Result<(), String> {
    let mut parser = Parser::new(src);
    let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;
    let sexp = ast.to_sexp();
    if sexp.contains("ERROR") {
        return Err(format!("expected clean parse, got ERROR nodes in: {sexp}\nsource: {src}"));
    }
    Ok(())
}

#[test]
fn push_postfix_array_deref() -> Result<(), String> {
    parse_clean("push $aref->@*, $x;")
}

#[test]
fn push_deep_postfix_array_deref() -> Result<(), String> {
    parse_clean("push $h->{key}->@*, $val;")
}

#[test]
fn unshift_postfix_array_deref() -> Result<(), String> {
    parse_clean("unshift $aref->@*, $x;")
}

#[test]
fn push_normal_array_regression() -> Result<(), String> {
    parse_clean("push @array, $val;")
}

#[test]
fn push_brace_array_regression() -> Result<(), String> {
    parse_clean("push @{$ref}, $val;")
}

#[test]
fn last_index_postfix_deref_simple() -> Result<(), String> {
    parse_clean("my $n = $aref->$#*;")
}

#[test]
fn last_index_postfix_deref_in_for() -> Result<(), String> {
    parse_clean("for (my $i = 0; $i <= $aref->$#*; $i++) { }")
}

#[test]
fn last_index_postfix_deref_arithmetic() -> Result<(), String> {
    parse_clean("my $len = $aref->$#* + 1;")
}

#[test]
fn last_index_postfix_deref_on_hash_slot() -> Result<(), String> {
    parse_clean("my $n = $self->{data}->$#*;")
}
