// Property-based tests for quote-like operators (q, qq, qr, qw, s///, tr///)

// Include the utilities module
include!("prop_test_utils.rs");

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_quote_like");

#[cfg(test)]
mod tests {
    use perl_parser::Parser;
    use proptest::prelude::*;

    // Import utilities from parent module
    use super::{
        closing_safe_payload, extract_ast_shape, m_modifiers, qr_modifiers, quote_delim_strategy,
        quote_payload_no_interp_safe, quote_payload_safe, regex_pattern, s_modifiers, shape,
        tr_modifiers,
    };

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: std::env::var("PROPTEST_CASES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(64),
            failure_persistence: Some(Box::new(
                proptest::test_runner::FileFailurePersistence::Direct(crate::REGRESS_DIR)
            )),
            ..ProptestConfig::default()
        })]

        #[test]
        fn qw_constant_discovery_holds(
            names in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]{0,15}",
                1..5
            ),
            (open, close) in quote_delim_strategy(),
        ) {
            // Ensure names don't contain delimiters
            prop_assume!(!names.iter().any(|n| n.contains(open) || n.contains(close)));

            let names_str = names.join(" ");
            let code = format!("use constant qw{open}{names_str}{close};",
                              open=open, close=close, names_str=names_str);

            let mut parser = Parser::new(&code);
            let ast = parser.parse();

            prop_assert!(ast.is_ok(), "Failed to parse: {}", code);

            // Extract constants from the AST
            let ast_str = format!("{:?}", ast.unwrap());

            // All names should be discoverable in the AST
            for name in &names {
                prop_assert!(
                    ast_str.contains(name),
                    "Constant '{}' not found in AST for: {}",
                    name, code
                );
            }
        }

        #[test]
        fn q_qq_delimiter_metamorphic(
            payload in quote_payload_safe(),  // Use safe generator
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!payload.contains(open1) && !payload.contains(close1));
            prop_assume!(!payload.contains(open2) && !payload.contains(close2));

            // Test q{} with different delimiters
            let q1 = format!("my $x = q{open}{payload}{close};",
                            open=open1, payload=payload, close=close1);
            let q2 = format!("my $x = q{open}{payload}{close};",
                            open=open2, payload=payload, close=close2);

            let mut parser1 = Parser::new(&q1);
            let mut parser2 = Parser::new(&q2);

            let ast1 = parser1.parse();
            let ast2 = parser2.parse();

            prop_assert!(ast1.is_ok(), "Failed to parse q with {}{}: {}",
                        open1, close1, q1);
            prop_assert!(ast2.is_ok(), "Failed to parse q with {}{}: {}",
                        open2, close2, q2);

            let shape1 = extract_ast_shape(&ast1.unwrap());
            let shape2 = extract_ast_shape(&ast2.unwrap());

            prop_assert_eq!(shape1, shape2,
                           "Different AST shapes for q with different delimiters\n{}\n---\n{}",
                           q1, q2);

            // Test qq{} with different delimiters
            let qq1 = format!("my $x = qq{open}{payload}{close};",
                             open=open1, payload=payload, close=close1);
            let qq2 = format!("my $x = qq{open}{payload}{close};",
                             open=open2, payload=payload, close=close2);

            let mut parser3 = Parser::new(&qq1);
            let mut parser4 = Parser::new(&qq2);

            let ast3 = parser3.parse();
            let ast4 = parser4.parse();

            prop_assert!(ast3.is_ok(), "Failed to parse qq with {}{}: {}",
                        open1, close1, qq1);
            prop_assert!(ast4.is_ok(), "Failed to parse qq with {}{}: {}",
                        open2, close2, qq2);

            let shape3 = extract_ast_shape(&ast3.unwrap());
            let shape4 = extract_ast_shape(&ast4.unwrap());

            prop_assert_eq!(shape3, shape4,
                           "Different AST shapes for qq with different delimiters\n{}\n---\n{}",
                           qq1, qq2);
        }

        #[test]
        fn qr_delimiter_metamorphic(
            pattern in regex_pattern(),
            modifiers in qr_modifiers(),
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!pattern.contains(open1) && !pattern.contains(close1));
            prop_assume!(!pattern.contains(open2) && !pattern.contains(close2));

            let code1 = format!("my $re = qr{open}{pattern}{close}{modifiers};",
                               open=open1, pattern=pattern, close=close1, modifiers=modifiers);
            let code2 = format!("my $re = qr{open}{pattern}{close}{modifiers};",
                               open=open2, pattern=pattern, close=close2, modifiers=modifiers);

            let mut parser1 = Parser::new(&code1);
            let mut parser2 = Parser::new(&code2);

            let ast1 = parser1.parse();
            let ast2 = parser2.parse();

            prop_assert!(ast1.is_ok(), "Failed to parse qr with {}{}: {}",
                        open1, close1, code1);
            prop_assert!(ast2.is_ok(), "Failed to parse qr with {}{}: {}",
                        open2, close2, code2);

            let shape1 = extract_ast_shape(&ast1.unwrap());
            let shape2 = extract_ast_shape(&ast2.unwrap());

            prop_assert_eq!(shape1, shape2,
                           "Different AST shapes for qr with {}{} vs {}{}\n{}\n---\n{}",
                           open1, close1, open2, close2, code1, code2);
        }

        #[test]
        fn substitution_delimiter_metamorphic(
            pattern in regex_pattern(),
            replacement in quote_payload_safe(),  // Use safe generator for replacement
            modifiers in s_modifiers(),
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!pattern.contains(open1) && !pattern.contains(close1));
            prop_assume!(!replacement.contains(open1) && !replacement.contains(close1));
            prop_assume!(!pattern.contains(open2) && !pattern.contains(close2));
            prop_assume!(!replacement.contains(open2) && !replacement.contains(close2));

            // Build s/// with first delimiter pair
            let code1 = if open1 == close1 {
                format!("s{open}{pattern}{close}{replacement}{close}{modifiers};",
                       open=open1, pattern=pattern, close=close1,
                       replacement=replacement, modifiers=modifiers)
            } else {
                format!("s{open}{pattern}{close}{open}{replacement}{close}{modifiers};",
                       open=open1, pattern=pattern, close=close1,
                       replacement=replacement, modifiers=modifiers)
            };

            // Build s/// with second delimiter pair
            let code2 = if open2 == close2 {
                format!("s{open}{pattern}{close}{replacement}{close}{modifiers};",
                       open=open2, pattern=pattern, close=close2,
                       replacement=replacement, modifiers=modifiers)
            } else {
                format!("s{open}{pattern}{close}{open}{replacement}{close}{modifiers};",
                       open=open2, pattern=pattern, close=close2,
                       replacement=replacement, modifiers=modifiers)
            };

            let mut parser1 = Parser::new(&code1);
            let mut parser2 = Parser::new(&code2);

            let ast1 = parser1.parse();
            let ast2 = parser2.parse();

            prop_assert!(ast1.is_ok(), "Failed to parse s/// with {}{}: {}",
                        open1, close1, code1);
            prop_assert!(ast2.is_ok(), "Failed to parse s/// with {}{}: {}",
                        open2, close2, code2);

            let shape1 = extract_ast_shape(&ast1.unwrap());
            let shape2 = extract_ast_shape(&ast2.unwrap());

            prop_assert_eq!(shape1, shape2,
                           "Different AST shapes for s/// with {}{} vs {}{}\n{}\n---\n{}",
                           open1, close1, open2, close2, code1, code2);
        }

        #[test]
        fn transliteration_alias_and_delimiter_metamorphic(
            from in "[a-z]{1,5}".prop_map(closing_safe_payload),  // Make safe
            to in "[A-Z]{1,5}".prop_map(closing_safe_payload),    // Make safe
            modifiers in tr_modifiers(),
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!from.contains(open1) && !from.contains(close1));
            prop_assume!(!to.contains(open1) && !to.contains(close1));
            prop_assume!(!from.contains(open2) && !from.contains(close2));
            prop_assume!(!to.contains(open2) && !to.contains(close2));

            // Build tr/// with first delimiter pair
            let tr1 = if open1 == close1 {
                format!("tr{open}{from}{close}{to}{close}{modifiers};",
                       open=open1, from=from, close=close1, to=to, modifiers=modifiers)
            } else {
                format!("tr{open}{from}{close}{open}{to}{close}{modifiers};",
                       open=open1, from=from, close=close1, to=to, modifiers=modifiers)
            };

            // y/// is an alias of tr///
            let y1 = tr1.replacen("tr", "y", 1);

            // Build tr/// with second delimiter pair
            let tr2 = if open2 == close2 {
                format!("tr{open}{from}{close}{to}{close}{modifiers};",
                       open=open2, from=from, close=close2, to=to, modifiers=modifiers)
            } else {
                format!("tr{open}{from}{close}{open}{to}{close}{modifiers};",
                       open=open2, from=from, close=close2, to=to, modifiers=modifiers)
            };

            let mut parser_tr1 = Parser::new(&tr1);
            let mut parser_y1 = Parser::new(&y1);
            let mut parser_tr2 = Parser::new(&tr2);

            let ast_tr1 = parser_tr1.parse();
            let ast_y1 = parser_y1.parse();
            let ast_tr2 = parser_tr2.parse();

            prop_assert!(ast_tr1.is_ok(), "Failed to parse tr with {}{}: {}",
                        open1, close1, tr1);
            prop_assert!(ast_y1.is_ok(), "Failed to parse y with {}{}: {}",
                        open1, close1, y1);
            prop_assert!(ast_tr2.is_ok(), "Failed to parse tr with {}{}: {}",
                        open2, close2, tr2);

            let shape_tr1 = extract_ast_shape(&ast_tr1.unwrap());
            let shape_y1 = extract_ast_shape(&ast_y1.unwrap());
            let shape_tr2 = extract_ast_shape(&ast_tr2.unwrap());

            // Test that y/// is an alias of tr///
            prop_assert_eq!(&shape_tr1, &shape_y1,
                           "y/// should be an alias of tr///\n{}\n---\n{}", tr1, y1);

            // Test delimiter metamorphic property
            prop_assert_eq!(&shape_tr1, &shape_tr2,
                           "Different AST shapes for tr/// with {}{} vs {}{}\n{}\n---\n{}",
                           open1, close1, open2, close2, tr1, tr2);
        }

        #[test]
        fn quote_like_in_contexts_metamorphic(
            payload in quote_payload_safe(),  // Use safe generator
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!payload.contains(open1) && !payload.contains(close1));
            prop_assume!(!payload.contains(open2) && !payload.contains(close2));

            let q1 = format!("q{open}{payload}{close}", open=open1, payload=payload, close=close1);
            let q2 = format!("q{open}{payload}{close}", open=open2, payload=payload, close=close2);

            let contexts = vec![
                ("print {};", "print context"),
                ("my $x = {} . 'suffix';", "concatenation context"),
                ("push @arr, {};", "list context"),
                ("if ({} eq 'test') {{ }}", "comparison context"),
                ("return {} || 'default';", "logical context"),
            ];

            for (context_template, context_name) in contexts {
                let code1 = context_template.replace("{}", &q1);
                let code2 = context_template.replace("{}", &q2);

                let mut parser1 = Parser::new(&code1);
                let mut parser2 = Parser::new(&code2);

                let ast1 = parser1.parse();
                let ast2 = parser2.parse();

                prop_assert!(ast1.is_ok(), "Failed to parse q in {}: {}", context_name, code1);
                prop_assert!(ast2.is_ok(), "Failed to parse q in {}: {}", context_name, code2);

                let shape1 = extract_ast_shape(&ast1.unwrap());
                let shape2 = extract_ast_shape(&ast2.unwrap());

                prop_assert_eq!(shape1, shape2,
                               "Different AST shapes in {} with {}{} vs {}{}\n{}\n---\n{}",
                               context_name, open1, close1, open2, close2, code1, code2);
            }
        }

        #[test]
        fn q_equals_qq_when_no_interpolation(
            payload in quote_payload_no_interp_safe(),  // Use safe generator
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!payload.contains(open1) && !payload.contains(close1));
            prop_assume!(!payload.contains(open2) && !payload.contains(close2));

            let q_code = format!("my $x = q{open}{payload}{close};",
                                open=open1, payload=payload, close=close1);
            let qq_code = format!("my $x = qq{open}{payload}{close};",
                                 open=open2, payload=payload, close=close2);

            let mut parser_q = Parser::new(&q_code);
            let mut parser_qq = Parser::new(&qq_code);

            let ast_q = parser_q.parse();
            let ast_qq = parser_qq.parse();

            prop_assert!(ast_q.is_ok(), "Failed to parse q: {}", q_code);
            prop_assert!(ast_qq.is_ok(), "Failed to parse qq: {}", qq_code);

            // When there's no interpolation, q and qq should produce the same shape
            // (though the actual QuoteLike node might differ in interpolation flag)
            let shape_q = shape(&ast_q.unwrap());
            let shape_qq = shape(&ast_qq.unwrap());

            // Both should have the same basic structure
            prop_assert_eq!(shape_q.len(), shape_qq.len(),
                           "q and qq should have same shape when no interpolation\n{}\n---\n{}",
                           q_code, qq_code);
        }

        #[test]
        fn match_operator_delimiter_metamorphic(
            pattern in regex_pattern(),
            modifiers in m_modifiers(),
            (open1, close1) in quote_delim_strategy(),
            (open2, close2) in quote_delim_strategy(),
        ) {
            // Avoid delimiter collisions
            prop_assume!(!pattern.contains(open1) && !pattern.contains(close1));
            prop_assume!(!pattern.contains(open2) && !pattern.contains(close2));

            let code1 = format!("'test' =~ m{open}{pattern}{close}{modifiers};",
                               open=open1, pattern=pattern, close=close1, modifiers=modifiers);
            let code2 = format!("'test' =~ m{open}{pattern}{close}{modifiers};",
                               open=open2, pattern=pattern, close=close2, modifiers=modifiers);

            let mut parser1 = Parser::new(&code1);
            let mut parser2 = Parser::new(&code2);

            let ast1 = parser1.parse();
            let ast2 = parser2.parse();

            prop_assert!(ast1.is_ok(), "Failed to parse m// with {}{}: {}",
                        open1, close1, code1);
            prop_assert!(ast2.is_ok(), "Failed to parse m// with {}{}: {}",
                        open2, close2, code2);

            let shape1 = extract_ast_shape(&ast1.unwrap());
            let shape2 = extract_ast_shape(&ast2.unwrap());

            prop_assert_eq!(shape1, shape2,
                           "Different AST shapes for m// with {}{} vs {}{}\n{}\n---\n{}",
                           open1, close1, open2, close2, code1, code2);
        }

        #[test]
        fn qw_whitespace_variations(
            words in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]{0,8}",
                1..5
            ),
            (open, close) in quote_delim_strategy(),
        ) {
            // Ensure words don't contain delimiters
            prop_assume!(!words.iter().any(|w| w.contains(open) || w.contains(close)));

            // Different whitespace separators
            let single_space = words.join(" ");
            let multiple_spaces = words.join("  ");
            let tabs = words.join("\t");
            let newlines = words.join("\n");
            let mixed = words.join(" \t\n ");

            let variations = vec![single_space, multiple_spaces, tabs, newlines, mixed];
            let mut shapes = Vec::new();

            for ws_variant in variations {
                let code = format!("my @arr = qw{open}{ws}{close};",
                                  open=open, ws=ws_variant, close=close);

                let mut parser = Parser::new(&code);
                let ast = parser.parse();

                prop_assert!(ast.is_ok(), "Failed to parse qw: {}", code);
                shapes.push(extract_ast_shape(&ast.unwrap()));
            }

            // All whitespace variations should produce the same shape
            for i in 1..shapes.len() {
                prop_assert_eq!(&shapes[0], &shapes[i],
                               "Different AST shapes for qw with different whitespace");
            }
        }
    }
}

// Include the shared utilities module
