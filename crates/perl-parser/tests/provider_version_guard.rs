#[cfg(test)]
mod provider_version_guard {
    use perl_parser::{Parser, declaration::DeclarationProvider, declaration::ParentMap};
    use std::sync::Arc;

    // Only meaningful in debug builds, where debug_assert! panics.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "used after AST refresh")]
    fn provider_panics_if_used_with_stale_version() {
        let code = "use constant FOO => 1; sub m { my $x = FOO }";
        let mut p = Parser::new(code);
        let ast = match p.parse() {
            Ok(ast) => Arc::new(ast),
            Err(e) => panic!("Parse error: {:?}", e),
        };

        // Build a real parent map so we get far enough to hit the assert.
        let mut pm: ParentMap = ParentMap::default();
        DeclarationProvider::build_parent_map(&ast, &mut pm, None);

        // Construct provider with version 1…
        let prov = unsafe {
            DeclarationProvider::new(ast.clone(), code.to_string(), "file:///x".into())
                .with_parent_map(&pm)
                .with_doc_version(1)
        };

        // …but call it with a newer doc version => should panic in debug.
        let off = match code.find("FOO") {
            Some(off) => off,
            None => panic!("FOO not found in code"),
        };
        let _ = prov.find_declaration(off, 2);
    }
}
