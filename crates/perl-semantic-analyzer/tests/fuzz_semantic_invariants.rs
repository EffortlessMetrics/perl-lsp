use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::ScopeAnalyzer;
use perl_semantic_analyzer::symbol::SymbolExtractor;
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

fn identifier_strategy() -> impl Strategy<Value = String> {
    let first = prop_oneof![
        Just('_'),
        (b'a'..=b'z').prop_map(char::from),
        (b'A'..=b'Z').prop_map(char::from)
    ];
    let rest_char = prop_oneof![
        Just('_'),
        (b'a'..=b'z').prop_map(char::from),
        (b'A'..=b'Z').prop_map(char::from),
        (b'0'..=b'9').prop_map(char::from),
    ];

    (first, prop::collection::vec(rest_char, 0..8)).prop_map(|(first, rest)| {
        let mut ident = String::with_capacity(rest.len() + 1);
        ident.push(first);
        ident.extend(rest);
        ident
    })
}

fn package_name_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(identifier_strategy(), 1..=3).prop_map(|parts| parts.join("::"))
}

fn perl_fragment_strategy() -> impl Strategy<Value = String> {
    let scalar_decl = (identifier_strategy(), any::<i16>())
        .prop_map(|(name, value)| format!("my ${name} = {value};"));
    let array_decl = (identifier_strategy(), any::<u8>(), any::<u8>())
        .prop_map(|(name, a, b)| format!("my @{name} = ({a}, {b});"));
    let hash_decl = (identifier_strategy(), any::<u8>(), any::<u8>())
        .prop_map(|(name, a, b)| format!("my %{name} = (left => {a}, right => {b});"));
    let package_decl = package_name_strategy().prop_map(|pkg| format!("package {pkg};"));
    let sub_decl = (identifier_strategy(), identifier_strategy(), any::<i8>()).prop_map(
        |(sub_name, local_name, value)| {
            format!("sub {sub_name} {{ my ${local_name} = {value}; return ${local_name}; }}")
        },
    );
    let conditional_block = (identifier_strategy(), any::<i8>())
        .prop_map(|(name, value)| format!("if ({value} > 0) {{ my ${name} = {value}; }}"));
    let regex_match =
        identifier_strategy().prop_map(|name| format!("my ${name} = 'abc'; ${name} =~ /a/;"));
    let constant_decl = (identifier_strategy(), any::<u16>())
        .prop_map(|(name, value)| format!("use constant {name} => {value};"));

    prop_oneof![
        scalar_decl,
        array_decl,
        hash_decl,
        package_decl,
        sub_decl,
        conditional_block,
        regex_match,
        constant_decl,
    ]
}

fn perl_program_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(perl_fragment_strategy(), 1..25).prop_map(|parts| parts.join("\n"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(250))]

    #[test]
    fn fuzz_semantic_symbol_and_scope_invariants(source in perl_program_strategy()) {
        let mut parser = Parser::new(&source);
        if let Ok(ast) = parser.parse() {
            let source_len = source.len();

            let symbol_table = SymbolExtractor::new_with_source(&source).extract(&ast);
            for symbols in symbol_table.symbols.values() {
                for symbol in symbols {
                    prop_assert!(symbol.location.start <= symbol.location.end);
                    prop_assert!(symbol.location.end <= source_len);
                }
            }

            for references in symbol_table.references.values() {
                for reference in references {
                    prop_assert!(reference.location.start <= reference.location.end);
                    prop_assert!(reference.location.end <= source_len);
                }
            }

            for scope in symbol_table.scopes.values() {
                prop_assert!(scope.location.start <= scope.location.end);
                prop_assert!(scope.location.end <= source_len);
            }

            let issues = ScopeAnalyzer::new().analyze(&ast, &source, &[]);
            for issue in issues {
                prop_assert!(issue.range.0 <= issue.range.1);
                prop_assert!(issue.range.1 <= source_len);
                prop_assert!(!issue.description.is_empty());
            }
        }
    }
}
