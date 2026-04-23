//! Transform perl-parser S-expressions to standard Tree-sitter format
//!
//! This example shows how to transform our concise S-expression format
//! to the more verbose standard Tree-sitter format with field names.

use perl_parser::Parser;
use regex::Regex;
use std::sync::LazyLock;

static VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(variable (\S) (\w+)\)").unwrap_or_else(|_| unreachable!()));
static BINARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(binary_(\S+) ([^()]+|\([^)]*\)) ([^()]+|\([^)]*\))\)")
        .unwrap_or_else(|_| unreachable!())
});
static DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((\w+)_declaration ([^()]+|\([^)]*\))([^()]+|\([^)]*\))?\)")
        .unwrap_or_else(|_| unreachable!())
});
static CALL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(call (\w+) \(([^)]*)\)\)").unwrap_or_else(|_| unreachable!()));
static METHOD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(method_call ([^()]+|\([^)]*\)) (\w+) \(([^)]*)\)\)")
        .unwrap_or_else(|_| unreachable!())
});
static ASSIGN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(assignment_(\w+) ([^()]+|\([^)]*\)) ([^()]+|\([^)]*\))\)")
        .unwrap_or_else(|_| unreachable!())
});
static IF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(if ([^()]+|\([^)]*\)) ([^()]+|\([^)]*\))\)").unwrap_or_else(|_| unreachable!())
});

fn main() {
    let examples = vec![
        "my $x = 42;",
        "$hash->{key} = $value;",
        "print $foo, $bar;",
        "if ($x > 0) { print 'positive'; }",
        "$obj->method($arg1, $arg2);",
    ];

    for code in examples {
        println!("\n--- Perl Code ---");
        println!("{}", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let our_format = ast.to_sexp();
                println!("\nOur Format:");
                println!("{}", our_format);

                let standard_format = transform_to_standard(&our_format);
                println!("\nStandard Tree-sitter Format:");
                println!("{}", standard_format);
            }
            Err(e) => {
                println!("Parse error: {:?}", e);
            }
        }
    }
}

/// Transform our S-expression format to standard Tree-sitter format
fn transform_to_standard(sexp: &str) -> String {
    let mut result = sexp.to_string();

    // Transform variables: (variable $ x) -> (variable name: "$x")
    result = VAR_RE.replace_all(&result, "(variable name: \"$1$2\")").to_string();

    // Transform binary operators: (binary_+ ...) -> (binary_expression operator: "+" ...)
    result = BINARY_RE
        .replace_all(&result, "(binary_expression left: $2 operator: \"$1\" right: $3)")
        .to_string();

    // Transform declarations: (my_declaration ...) -> (variable_declaration kind: "my" ...)
    result = DECL_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let kind = &caps[1];
            let var = &caps[2];
            if let Some(init) = caps.get(3) {
                format!(
                    "(variable_declaration kind: \"{}\" name: {} value: {})",
                    kind,
                    var,
                    init.as_str()
                )
            } else {
                format!("(variable_declaration kind: \"{}\" name: {})", kind, var)
            }
        })
        .to_string();

    // Transform function calls: (call func (...)) -> (call_expression function: func arguments: (...))
    result = CALL_RE
        .replace_all(
            &result,
            "(call_expression function: (identifier \"$1\") arguments: (argument_list $2))",
        )
        .to_string();

    // Transform method calls: (method_call obj method (...)) -> (method_call_expression object: obj method: method arguments: (...))
    result = METHOD_RE.replace_all(&result, "(method_call_expression object: $1 method: (identifier \"$2\") arguments: (argument_list $3))").to_string();

    // Transform assignments: (assignment_assign ...) -> (assignment_expression operator: "=" ...)
    result = ASSIGN_RE
        .replace_all(&result, "(assignment_expression left: $2 operator: \"$1\" right: $3)")
        .to_string();

    // Transform conditionals: (if cond then) -> (if_statement condition: cond consequence: then)
    result = IF_RE.replace_all(&result, "(if_statement condition: $1 consequence: $2)").to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_transform() {
        assert_eq!(transform_to_standard("(variable $ x)"), "(variable name: \"$x\")");
    }

    #[test]
    fn test_binary_transform() {
        assert_eq!(
            transform_to_standard("(binary_+ (variable $ x) (number 42))"),
            "(binary_expression left: (variable name: \"$x\") operator: \"+\" right: (number 42))"
        );
    }

    #[test]
    fn test_declaration_transform() {
        assert_eq!(
            transform_to_standard("(my_declaration (variable $ x)(number 42))"),
            "(variable_declaration kind: \"my\" name: (variable name: \"$x\") value: (number 42))"
        );
    }
}
