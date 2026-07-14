use super::ImportMap;
use perl_parser_core::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

mod runtime_imports;
mod symbols;
mod used_modules;

use runtime_imports::collect_runtime_imports;
use symbols::collect_import_symbols;
use used_modules::is_importable_module;

/// Walk the top-level AST and build an `ImportMap` from `use` statements.
///
/// Only uppercase-starting module names are included (skips pragmas like
/// `strict`, `warnings`, `feature`, `constant`, `utf8`, `lib`, `parent`, `base`).
pub(super) fn extract_import_map(ast: &Node) -> ImportMap {
    let mut map: ImportMap = HashMap::new();
    collect(ast, &mut map);
    map
}

fn collect(node: &Node, map: &mut ImportMap) {
    match &node.kind {
        NodeKind::Use { module, args, .. } => collect_use_import(module, args, map),
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            collect_runtime_imports(statements, map);
            for stmt in statements {
                collect(stmt, map);
            }
        }
        _ => {}
    }
}

fn collect_use_import(module: &str, args: &[String], map: &mut ImportMap) {
    if !is_importable_module(module) || args.is_empty() {
        return;
    }

    let mut symbols: HashSet<String> = HashSet::new();
    let mut has_symbol_args = false;
    let mut has_unresolved_tag = false;

    for arg in args.iter().filter(|arg| is_symbol_arg_candidate(arg)) {
        // The second tuple element signals an unresolvable export tag.  We used
        // to bail out on any unresolved tag, silently discarding all symbols
        // collected so far (#1700).  Now we treat it as a partial miss: the
        // tag-expanded symbols are lost, but explicit symbol names survive.  If
        // the import only contains unknown tags, leave the module unfiltered.
        let (has_symbols_in_arg, unresolved_tag) =
            collect_import_symbols(module, arg, &mut symbols);
        has_symbol_args |= has_symbols_in_arg;
        has_unresolved_tag |= unresolved_tag;
    }

    if has_symbol_args && has_unresolved_tag && symbols.is_empty() {
        return;
    }

    if has_symbol_args {
        map.entry(module.to_string()).or_default().extend(symbols);
    } else {
        map.entry(module.to_string()).or_default();
    }
}

fn is_symbol_arg_candidate(arg: &str) -> bool {
    let first_byte = arg.as_bytes().first().copied().unwrap_or(0);
    !first_byte.is_ascii_digit() && !arg.starts_with('-')
}

pub(super) use used_modules::collect_used_module_names;

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    /// Regression test for #1700: an unresolvable export tag must not silently
    /// discard the explicit symbols collected alongside it.
    ///
    /// Before the fix, `collect_use_import` returned early on `has_unresolved_tag`,
    /// leaving `Module::Thing` with no entry in the ImportMap even though `known_sub`
    /// had already been added to `symbols`.
    #[test]
    fn unresolved_export_tag_keeps_explicit_symbols() {
        let code = "use Module::Thing qw(:unknown_tag known_sub);\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        let symbols = must_some(map.get("Module::Thing"));
        assert!(
            symbols.contains("known_sub"),
            "explicit symbol `known_sub` must survive an unresolved export tag; got: {symbols:?}"
        );
    }

    /// Verify that a purely resolved import still works correctly after the change.
    #[test]
    fn resolved_only_import_unaffected() {
        let code = "use Foo::Bar qw(alpha beta);\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        let symbols = must_some(map.get("Foo::Bar"));
        assert!(symbols.contains("alpha"), "alpha must be present; got: {symbols:?}");
        assert!(symbols.contains("beta"), "beta must be present; got: {symbols:?}");
    }

    #[test]
    fn non_importable_module_with_args_is_skipped() {
        let code = "use strict qw(vars refs);\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        assert!(
            !map.contains_key("strict"),
            "pragma imports should stay out of ImportMap even with args; got: {map:?}"
        );
    }

    #[test]
    fn importable_module_without_args_is_skipped() {
        let code = "use Module::Thing;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        assert!(
            !map.contains_key("Module::Thing"),
            "use statements without import args should not create ImportMap entries; got: {map:?}"
        );
    }

    #[test]
    fn unresolved_export_tag_without_explicit_symbols_remains_unknown() {
        let code = "use Module::Thing qw(:unknown_tag);\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        assert!(
            !map.contains_key("Module::Thing"),
            "unresolved-only tag imports should remain unknown instead of becoming an empty-set import; got: {map:?}"
        );
    }

    #[test]
    fn empty_qw_import_records_empty_entry() {
        let code = "use Module::Thing qw();\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        let symbols = must_some(map.get("Module::Thing"));
        assert!(
            symbols.is_empty(),
            "actually empty import lists should still record an empty entry; got: {symbols:?}"
        );
    }

    /// Multiple explicit symbols alongside an unresolved tag — all explicit symbols
    /// must survive; unresolvable tag symbols are silently omitted (acceptable partial miss).
    #[test]
    fn multiple_explicit_symbols_with_unresolved_tag() {
        let code = "use My::Util qw(:nonexistent_tag foo bar baz);\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let map = extract_import_map(&ast);

        let symbols = must_some(map.get("My::Util"));
        assert!(symbols.contains("foo"), "foo must survive; got: {symbols:?}");
        assert!(symbols.contains("bar"), "bar must survive; got: {symbols:?}");
        assert!(symbols.contains("baz"), "baz must survive; got: {symbols:?}");
    }
}
