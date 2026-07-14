use super::super::{
    CompletionContext, CompletionItem, CompletionProvider, builtins, file_path, functions,
    keywords, methods, packages, snippets, test_more, variables, workspace, xs_api,
};
use super::CompletionFlow;
use perl_semantic_analyzer::symbol::SymbolKind;

pub(super) fn complete_dispatch(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    position: usize,
    filepath: Option<&str>,
    is_cancelled: &dyn Fn() -> bool,
) -> CompletionFlow {
    if complete_use_or_structural_context(
        provider,
        completions,
        context,
        source,
        position,
        is_cancelled,
    ) {
        return CompletionFlow::SortAndReturn;
    }

    if context.in_string {
        complete_file_path_context(completions, context, source, is_cancelled);
        return CompletionFlow::SortAndReturn;
    }

    if let Some(flow) = complete_sigil_context(provider, completions, context, source, is_cancelled)
    {
        return flow;
    }

    if complete_symbol_namespace_context(provider, completions, context, source) {
        return CompletionFlow::SortAndReturn;
    }

    complete_general_context(provider, completions, context, source, filepath, is_cancelled)
}

fn complete_use_or_structural_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    position: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> bool {
    if let Some((module_name, qw_prefix)) =
        CompletionProvider::detect_use_qw_import_context(source, position)
    {
        workspace::add_use_qw_import_completions(
            completions,
            context,
            &provider.workspace_index,
            &module_name,
            &qw_prefix,
        );
        return true;
    }

    if context.in_use_statement && !context.prefix.starts_with('$') {
        workspace::add_use_module_completions_with_cache(
            completions,
            context,
            &provider.workspace_index,
            &provider.include_paths,
            &provider.system_inc_paths,
            provider.include_system_inc,
            provider.scan_cache.as_deref(),
            is_cancelled,
        );
        return true;
    }

    if provider.is_has_type_value_context(source, position) {
        provider.add_has_type_completions(completions, context);
        return true;
    }

    if provider.is_has_options_key_context(source, position) {
        provider.add_has_option_completions(completions, context);
        return true;
    }

    if !context.in_string
        && !context.in_regex
        && let Some((varname, key_prefix)) =
            CompletionProvider::detect_hash_key_context(source, position)
    {
        CompletionProvider::add_hash_key_completions(
            completions,
            context,
            source,
            &varname,
            &key_prefix,
        );
        return true;
    }

    if let Some(package_name) = provider.object_pad_constructor_package(source, position) {
        provider.add_object_pad_constructor_completions(completions, context, &package_name);
        return true;
    }

    if !context.in_string && is_method_arrow_context(context) {
        methods::add_method_completions(completions, context, source, &provider.symbol_table);
        workspace::add_workspace_method_completions(
            completions,
            context,
            source,
            provider.type_engine.as_ref(),
            &provider.workspace_index,
            &provider.used_modules,
        );
        return true;
    }

    if complete_indirect_method_context(provider, completions, context, source) {
        return true;
    }

    false
}

fn is_method_arrow_context(context: &CompletionContext) -> bool {
    (context.trigger_character == Some('>') || context.trigger_character == Some('-'))
        && context.prefix.ends_with("->")
        && context.prefix.len() > 2
}

/// Statement-level keywords and I/O builtins that look like a bareword method
/// at a statement start but are never a user-defined indirect-method call we
/// want to complete (`my $x`, `return $foo`, `print $fh`, `die $e`, ...).
///
/// The dual gate (classify_receiver + workspace probe) already filters most
/// false positives, but these builtins are excluded eagerly to prevent
/// spurious method completions in the very common `die $exception_obj` and
/// `warn $obj` patterns where the receiver resolves to a real class.
const INDIRECT_METHOD_EXCLUDED: &[&str] = &[
    "my", "our", "local", "state", "sub", "return", "if", "unless", "while", "until", "for",
    "foreach", "do", "use", "no", "require", "else", "elsif", "print", "printf", "say", "and",
    "or", "not", "eq", "ne", "lt", "gt", "le", "ge", "cmp", "x", "package", "qw",
    // Exception/error builtins — very commonly called with exception objects
    // (`die $e`, `warn $msg`), triggering false method completions otherwise.
    "die", "warn", "eval",
    // Object-inspection builtins — `ref $obj`, `defined $obj`, `bless $ref`
    // take an object as argument but are not method calls.
    "ref", "defined", "bless",
    // List/array builtins that may take a variable-length list starting with
    // what looks like a receiver.
    "push", "pop", "shift", "unshift", "splice", "grep", "map", "sort",
    // File/IO builtins not already covered by `print`/`printf`/`say`.
    "open", "close", "read", "write", "seek", "tell", "eof", "binmode", "chomp", "chop", "chdir",
    "stat", "unlink", "rename", "chmod", "undef",
    // Carp exporters — `croak $obj` / `confess $msg` are diagnostic calls, not
    // method calls. These are imported subs, so the lexer's builtin set below
    // does not cover them; list them explicitly.
    "croak", "carp", "confess", "cluck",
];

/// True when `word` is a plausible indirect-method name: a lowercase-initial
/// bareword (`new`, `process`, ...) that is not a statement keyword, Carp
/// exporter, or Perl builtin function. Uppercase-initial words (`Foo`,
/// `STDOUT`) and sigil/`::` tokens are rejected so we only fire on the method
/// slot of `method RECEIVER ...`.
fn is_indirect_method_word(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first == '_') {
        return false;
    }
    if !word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    if INDIRECT_METHOD_EXCLUDED.contains(&word) {
        return false;
    }
    // Perl builtin functions (`length $s`, `keys %h`, `scalar @a`, `delete
    // $h{k}`, ...) take their argument as a list, not an indirect-object
    // receiver. Defer to the lexer's authoritative builtin set so we don't
    // hand-maintain every name; `new` and user subs are not builtins and pass.
    !perl_lexer::builtins::builtin_signatures_phf::is_builtin(word)
}

/// Advance over `[A-Za-z0-9_]` from `from`, returning the byte offset of the
/// first non-word byte (the end of the method token under the cursor).
fn indirect_word_end(source: &str, from: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = from.min(bytes.len());
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    i
}

/// Parse the receiver token that follows the method name in indirect-object
/// syntax: a `$scalar` variable (`method $obj`) or an uppercase-initial bareword
/// class (`new Class`, `new Class::Name`). Requires at least one separating
/// space. Returns `None` for array/hash receivers (`method @args`) and anything
/// else, so those degrade gracefully to ordinary completion.
fn parse_indirect_receiver(source: &str, from: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = from;
    let ws_start = i;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i == ws_start || i >= bytes.len() {
        return None;
    }

    if bytes[i] == b'$' {
        let start = i;
        i += 1;
        let id_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if i == id_start {
            return None;
        }
        return Some(source[start..i].to_string());
    }

    if bytes[i].is_ascii_uppercase() {
        let start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
        {
            i += 1;
        }
        return Some(source[start..i].to_string());
    }

    None
}

/// Route Perl indirect-object method calls (`method $obj @args`, `new Class`)
/// through the same method-completion providers as the arrow form (#1758).
///
/// The receiver follows the method in indirect syntax, so we detect the
/// `method RECEIVER` shape from source text and synthesize an equivalent
/// arrow-form context (`RECEIVER->`) that the existing receiver-classification
/// and workspace-method logic already understands. We only commit to method
/// completion when the receiver resolves to a concrete package that actually
/// contributes workspace methods — otherwise ordinary statements (`my $x`,
/// `print $fh`) fall through unchanged.
fn complete_indirect_method_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
) -> bool {
    if context.in_string || context.in_regex || context.in_comment {
        return false;
    }
    if !is_indirect_method_word(&context.prefix) {
        return false;
    }
    // Reject method/glob/qualified segments (`$x->word`, `&word`, `Foo::word`):
    // those are not statement-level indirect calls.
    if context.prefix_start > 0 {
        let prev = source.as_bytes()[context.prefix_start - 1];
        if matches!(prev, b'>' | b'&' | b'$' | b'@' | b'%' | b':') {
            return false;
        }
    }

    let word_end = indirect_word_end(source, context.position);
    let Some(receiver) = parse_indirect_receiver(source, word_end) else {
        return false;
    };

    // Synthesize the equivalent arrow boundary `receiver->`. `prefix_start` and
    // `position` are preserved so completion text edits still replace the
    // indirect method token under the cursor.
    let mut synth = context.clone();
    synth.prefix = format!("{receiver}->");

    // Gate: require a concrete receiver package. `Dynamic`/`Unknown` receivers
    // (e.g. `print $fh`, `return $foo`) carry no package and fall through.
    let evidence = workspace::classify_receiver(&synth, source, provider.type_engine.as_ref());
    if evidence.package().is_none() {
        return false;
    }

    // Second gate: the resolved package must actually contribute workspace
    // methods. An unindexed class (`new SomeUnknownThing`) yields nothing and
    // falls through rather than offering bare object defaults.
    let mut probe = Vec::new();
    workspace::add_workspace_method_completions(
        &mut probe,
        &synth,
        source,
        provider.type_engine.as_ref(),
        &provider.workspace_index,
        &provider.used_modules,
    );
    if probe.is_empty() {
        return false;
    }

    let inserted_start = completions.len();
    methods::add_method_completions(completions, &synth, source, &provider.symbol_table);
    workspace::add_workspace_method_completions(
        completions,
        &synth,
        source,
        provider.type_engine.as_ref(),
        &provider.workspace_index,
        &provider.used_modules,
    );

    // The arrow-form providers emit parenthesized insert text (`run()`), which is
    // correct for `$obj->run()` but invalid in indirect syntax: accepting it in
    // `new Child` would produce `run() Child`. The edit range only replaces the
    // method token, so normalize the inserted items to the bare method name
    // (`run`) — yielding the valid indirect call `run Child` / `run $obj`.
    for item in completions.iter_mut().skip(inserted_start) {
        item.insert_text = Some(item.label.clone());
    }
    true
}

fn complete_sigil_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<CompletionFlow> {
    let (sigil, kind) = sigil_kind(context)?;

    if context.prefix.contains("::") {
        packages::add_package_completions(
            completions,
            context,
            source,
            &provider.symbol_table,
            &provider.workspace_index,
        );
        if !completions.is_empty() {
            return Some(CompletionFlow::Return(std::mem::take(completions)));
        }
    }

    variables::add_variable_completions(completions, context, kind, &provider.symbol_table);
    if is_cancelled() {
        return Some(CompletionFlow::Cancelled);
    }
    variables::add_special_variables(completions, context, sigil);
    Some(CompletionFlow::SortAndReturn)
}

fn sigil_kind(context: &CompletionContext) -> Option<(&'static str, SymbolKind)> {
    if context.prefix.starts_with('$') {
        Some(("$", SymbolKind::scalar()))
    } else if context.prefix.starts_with('@') {
        Some(("@", SymbolKind::array()))
    } else if context.prefix.starts_with('%') {
        Some(("%", SymbolKind::hash()))
    } else {
        None
    }
}

fn complete_symbol_namespace_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
) -> bool {
    if context.prefix.starts_with('&') {
        functions::add_function_completions(completions, context, &provider.symbol_table);
        return true;
    }

    if context.prefix.contains("::") {
        packages::add_package_completions(
            completions,
            context,
            source,
            &provider.symbol_table,
            &provider.workspace_index,
        );
        return true;
    }

    false
}

fn complete_file_path_context(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    is_cancelled: &dyn Fn() -> bool,
) {
    let line_prefix = &source[..context.position];
    let Some(start) = line_prefix.rfind(['"', '\'']) else {
        return;
    };

    let Some(quote_char) = source.get(start..).and_then(|text| text.chars().next()) else {
        return;
    };
    let string_end =
        source[start + 1..].find(quote_char).map(|index| start + 1 + index).unwrap_or(source.len());
    let full_string_content = &source[start + 1..string_end];
    if full_string_content.contains('\0') {
        return;
    }

    let path_prefix = &line_prefix[start + 1..];
    if looks_like_path_prefix(path_prefix) {
        let file_context =
            file_path::FileCompletionContext::new(path_prefix, start + 1, context.position);
        completions.extend(file_path::complete_file_paths(&file_context, is_cancelled));
    }
}

fn looks_like_path_prefix(path_prefix: &str) -> bool {
    path_prefix.contains('/')
        || path_prefix.contains('\\')
        || path_prefix.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn complete_general_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    filepath: Option<&str>,
    is_cancelled: &dyn Fn() -> bool,
) -> CompletionFlow {
    let keyword_set = keywords::keywords();
    if context.prefix.is_empty() || provider.could_be_keyword(&context.prefix, keyword_set) {
        keywords::add_keyword_completions(completions, context, keyword_set);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
    }

    let builtin_set = builtins::create_builtins();
    xs_api::add_xs_api_completions(completions, context, source, filepath);
    if context.prefix.is_empty() || provider.could_be_function(&context.prefix, &builtin_set) {
        builtins::add_builtin_completions(completions, context, &builtin_set);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
        functions::add_function_completions(completions, context, &provider.symbol_table);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
    }

    snippets::add_snippet_completions(completions, context);
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    variables::add_all_variables(completions, context, &provider.symbol_table);
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    workspace::add_visible_symbol_completions(
        completions,
        context,
        &provider.workspace_index,
        filepath,
    );
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    workspace::add_workspace_symbol_completions(
        completions,
        context,
        source,
        &provider.workspace_index,
        &provider.import_map,
    );
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    if provider.is_test_context(source, filepath) {
        test_more::add_test_more_completions(completions, context);
    }

    CompletionFlow::SortAndReturn
}

#[cfg(test)]
mod indirect_helper_tests {
    use super::{indirect_word_end, is_indirect_method_word, parse_indirect_receiver};

    #[test]
    fn is_indirect_method_word_accepts_lowercase_barewords() {
        assert!(is_indirect_method_word("new"));
        assert!(is_indirect_method_word("process"));
        assert!(is_indirect_method_word("_private"));
        assert!(is_indirect_method_word("spawn2"));
    }

    #[test]
    fn is_indirect_method_word_call_presence_observer() {
        assert_eq!(is_indirect_method_word("new"), true, "input that reaches call word.chars()");
        assert_eq!(
            is_indirect_method_word(""),
            false,
            "input that reaches call chars.next() and takes the empty-word branch"
        );
        assert_eq!(
            is_indirect_method_word("Foo"),
            false,
            "input that reaches call first.is_ascii_lowercase() and rejects uppercase receivers"
        );
        assert_eq!(
            is_indirect_method_word("_private"),
            true,
            "input that reaches call first.is_ascii_lowercase() and accepts underscore methods"
        );
        assert_eq!(
            is_indirect_method_word("new::Child"),
            false,
            "input that reaches call word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')"
        );
        assert_eq!(
            is_indirect_method_word("print"),
            false,
            "input that reaches call INDIRECT_METHOD_EXCLUDED.contains(&word)"
        );
        assert_eq!(
            is_indirect_method_word("length"),
            false,
            "input that reaches call perl_lexer::builtins::builtin_signatures_phf::is_builtin(word)"
        );
    }

    #[test]
    fn is_indirect_method_word_rejects_non_method_words() {
        // Uppercase-initial (classes / filehandles) are receivers, not methods.
        assert!(!is_indirect_method_word("Foo"));
        assert!(!is_indirect_method_word("STDOUT"));
        // Empty / sigil / non-word.
        assert!(!is_indirect_method_word(""));
        assert!(!is_indirect_method_word("$obj"));
        // Statement keywords and Carp exporters.
        assert!(!is_indirect_method_word("my"));
        assert!(!is_indirect_method_word("return"));
        assert!(!is_indirect_method_word("croak"));
        // Perl builtin functions (via the lexer set).
        assert!(!is_indirect_method_word("length"));
        assert!(!is_indirect_method_word("keys"));
        assert!(!is_indirect_method_word("delete"));
        assert!(!is_indirect_method_word("print"));
    }

    #[test]
    fn indirect_word_end_stops_at_first_non_word_byte() {
        assert_eq!(indirect_word_end("new Child", 0), 3);
        assert_eq!(indirect_word_end("new Child", 3), 3); // already at the space
        assert_eq!(indirect_word_end("process $obj", 0), 7);
        assert_eq!(indirect_word_end("run", 0), 3); // word runs to end of input
        assert_eq!(indirect_word_end("", 0), 0);
        assert_eq!(indirect_word_end("ab", 9), 2); // clamps out-of-range start
    }

    #[test]
    fn indirect_word_end_call_presence_observer() {
        assert_eq!(
            indirect_word_end("process $obj", 0),
            7,
            "input that reaches call source.as_bytes()"
        );
        assert_eq!(
            indirect_word_end("process $obj", 9),
            12,
            "input that reaches call from.min(bytes.len())"
        );
        assert_eq!(
            indirect_word_end("process $obj", 7),
            7,
            "input that rejects non-word boundary after bytes[i].is_ascii_alphanumeric()"
        );
        assert_eq!(
            indirect_word_end("run_more Child", 0),
            8,
            "input that reaches call bytes[i].is_ascii_alphanumeric()"
        );
    }

    #[test]
    fn parse_indirect_receiver_reads_uppercase_class() {
        // Grips dispatch.rs:231 — the uppercase-class branch.
        assert_eq!(parse_indirect_receiver("new Child", 3), Some("Child".to_string()));
        // Grips dispatch.rs:234 — the `::` scan for qualified class names.
        assert_eq!(parse_indirect_receiver("new Foo::Bar", 3), Some("Foo::Bar".to_string()));
        // Trailing punctuation ends the class token.
        assert_eq!(parse_indirect_receiver("new Child, 1", 3), Some("Child".to_string()));
    }

    #[test]
    fn parse_indirect_receiver_reads_scalar_variable() {
        assert_eq!(parse_indirect_receiver("process $obj", 7), Some("$obj".to_string()));
        assert_eq!(parse_indirect_receiver("m $self_ref", 1), Some("$self_ref".to_string()));
    }

    #[test]
    fn parse_indirect_receiver_call_presence_observer() {
        assert_eq!(
            parse_indirect_receiver("process $obj", 7),
            Some("$obj".to_string()),
            "input that reaches call source.as_bytes()"
        );
        assert_eq!(
            parse_indirect_receiver("process $obj_2", 7),
            Some("$obj_2".to_string()),
            "input that reaches call bytes[i].is_ascii_alphanumeric()"
        );
        assert_eq!(
            parse_indirect_receiver("process $", 7),
            None,
            "input that hits scalar boundary i == id_start"
        );
        assert_eq!(
            parse_indirect_receiver("new Child::Package", 3),
            Some("Child::Package".to_string()),
            "input that reaches class scan boundary bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':'"
        );
        assert_eq!(
            parse_indirect_receiver("new child", 3),
            None,
            "input that rejects lowercase receiver at bytes[i].is_ascii_uppercase()"
        );
    }

    #[test]
    fn parse_indirect_receiver_rejects_non_receivers() {
        // Lowercase bareword is NOT a receiver — deleting the uppercase guard at
        // dispatch.rs:231 would wrongly accept it, so this pins that branch.
        assert_eq!(parse_indirect_receiver("foo bar", 3), None);
        // Array/hash sigils are not single-object receivers.
        assert_eq!(parse_indirect_receiver("method @args", 6), None);
        assert_eq!(parse_indirect_receiver("method %opts", 6), None);
        // Bare `$` with no identifier.
        assert_eq!(parse_indirect_receiver("method $", 6), None);
        // No separating whitespace.
        assert_eq!(parse_indirect_receiver("methodChild", 6), None);
        // Nothing after the method word.
        assert_eq!(parse_indirect_receiver("method ", 6), None);
        assert_eq!(parse_indirect_receiver("method", 6), None);
    }
}
