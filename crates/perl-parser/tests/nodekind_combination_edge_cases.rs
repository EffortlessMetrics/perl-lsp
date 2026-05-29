//! Edge-case coverage for under-tested NodeKind variants.
//!
//! Each NodeKind variant has at least one corpus example via
//! `corpus_nodekind_coverage_test.rs`, but several variants lack focused
//! edge-case coverage — particularly around modifier combinations,
//! aliasing patterns, control-flow positions, and rarer surface syntax.
//!
//! Tests here lock in:
//!   * `Defer` — ordering, nesting, control-flow context
//!   * `Ellipsis` — yada-yada in subroutine bodies and statements
//!   * `IndirectCall` — `new Class @args` and filehandle `print $fh "..."`
//!   * `LabeledStatement` — bare blocks with `last`/`redo`
//!   * `Goto` — expression and code-ref targets
//!   * `Transliteration` — every modifier (`c`, `d`, `s`, `r`) and `y///`
//!   * `Typeglob` — aliasing and cross-package assignment
//!   * `PhaseBlock` — all five compile/runtime phases
//!   * Diamond / Readline / Glob — disambiguating `<>`, `<FH>`, `<*.pat>`

use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};
use perl_tdd_support::{must, must_some};

mod nodekind_helpers;
use nodekind_helpers::{find_first_node_of_kind, has_node_kind};

/// Count every node in the AST matching the predicate.
fn count_nodes<F: Fn(&NodeKind) -> bool>(ast: &Node, pred: &F) -> usize {
    let mut n = if pred(&ast.kind) { 1 } else { 0 };
    ast.for_each_child(|c| n += count_nodes(c, pred));
    n
}

/// Multiple defer blocks in the same scope (LIFO at runtime — parser must
/// preserve every one), defer nested inside defer, defer inside control flow.
#[test]
fn test_defer_ordering_nesting_and_control_flow() {
    let code = r#"
use feature 'defer';

sub cleanup_pipeline {
    my ($resource) = @_;

    defer { print "step 1 cleanup\n"; }
    defer { print "step 2 cleanup\n"; }
    defer { print "step 3 cleanup\n"; }

    # Defer inside conditional — still attached to the enclosing scope.
    if ($resource->{needs_lock}) {
        defer { $resource->release_lock(); }
        $resource->acquire_lock();
    }

    # Defer inside a loop — one cleanup per iteration scope.
    for my $item (@{ $resource->{items} }) {
        defer { print "iter cleanup for $item\n"; }
        process($item);
    }

    # Defer with early return — cleanup must still run.
    return if $resource->{abort};

    # Defer inside defer block.
    defer {
        defer { print "innermost\n"; }
        print "outer defer body\n";
    }

    return $resource;
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let defer_count = count_nodes(&ast, &|k| matches!(k, NodeKind::Defer { .. }));
    assert_eq!(
        defer_count, 7,
        "Expected 7 defer blocks (3 top-level + cond + loop + outer + inner)"
    );

    // The innermost defer must live inside another defer block.
    let outer_defer = must_some(find_first_node_of_kind(&ast, "Defer"));
    fn defer_contains_defer(node: &Node) -> bool {
        match &node.kind {
            NodeKind::Defer { block } => {
                let mut found = false;
                block.for_each_child(|c| {
                    if !found && matches!(c.kind, NodeKind::Defer { .. }) {
                        found = true;
                    }
                });
                found || {
                    let mut child_found = false;
                    node.for_each_child(|c| {
                        if !child_found {
                            child_found = defer_contains_defer(c);
                        }
                    });
                    child_found
                }
            }
            _ => {
                let mut found = false;
                node.for_each_child(|c| {
                    if !found {
                        found = defer_contains_defer(c);
                    }
                });
                found
            }
        }
    }
    assert!(
        defer_contains_defer(outer_defer) || defer_contains_defer(&ast),
        "Should find a Defer block containing another Defer"
    );

    // Defer + loop coexist.
    assert!(has_node_kind(&ast, "Foreach") || has_node_kind(&ast, "For"));
    assert!(has_node_kind(&ast, "If"));
    assert!(has_node_kind(&ast, "Return"));
}

/// Yada-yada (`...`) appears as a subroutine body, inside a block, and as the
/// sole statement of a conditional branch. Each surface form must produce
/// `NodeKind::Ellipsis` (not an error or `UnknownRest`).
#[test]
fn test_ellipsis_yada_yada_placeholder_contexts() {
    let code = r#"
# yada-yada as a complete subroutine body — common stub pattern.
sub not_implemented_yet { ... }

# yada-yada inside a multi-statement subroutine.
sub partially_implemented {
    my ($x) = @_;
    return 0 if !defined $x;
    ...;
}

# yada-yada inside each branch of a conditional.
sub branching_stub {
    my ($mode) = @_;
    if ($mode eq 'read') {
        ...;
    } elsif ($mode eq 'write') {
        ...;
    } else {
        ...;
    }
}

# yada-yada inside an eval — must not be swallowed by the recovery path.
my $maybe = eval { ...; 42 };
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let ellipsis_count = count_nodes(&ast, &|k| matches!(k, NodeKind::Ellipsis));
    assert_eq!(
        ellipsis_count, 6,
        "Expected 6 yada-yada occurrences (1 sub body + 1 in partial sub + 3 in branches + 1 in eval)"
    );
    assert!(has_node_kind(&ast, "Eval"));
    assert!(!has_node_kind(&ast, "UnknownRest"), "yada-yada must not trigger recovery fallback");
}

/// Classical indirect-object syntax for both user methods (`new Class @args`)
/// and core builtins (`print $fh "...";`). These must parse as `IndirectCall`,
/// not as a `FunctionCall` swallowing the object.
#[test]
fn test_indirect_call_classical_and_builtin_forms() {
    let code = r#"
package Player;
sub new {
    my ($class, $name) = @_;
    return bless { name => $name }, $class;
}

# Classical indirect-object constructor call — `new Class @args`.
my $hero = new Player "Galadriel";

# Indirect-object on a user method (`method $obj @args` form).
sub greet {
    my ($self, $msg) = @_;
    return "$self->{name}: $msg";
}
my $line = greet $hero "hello";

# Builtin indirect filehandle syntax — `print $fh "..."`.
open my $log, '>>', '/tmp/log' or die $!;
print $log "indirect filehandle write\n";
printf $log "%s = %d\n", "count", 42;
say $log "say with indirect handle";
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let indirect_count = count_nodes(&ast, &|k| matches!(k, NodeKind::IndirectCall { .. }));
    assert!(
        indirect_count >= 4,
        "Expected at least 4 indirect calls (new + greet + print + printf/say), got {indirect_count}"
    );

    // `new Player "Galadriel"` — object is an Identifier, not a Variable.
    let mut saw_new = false;
    fn walk_indirect(node: &Node, saw_new: &mut bool) {
        if let NodeKind::IndirectCall { method, object, .. } = &node.kind
            && method == "new"
            && matches!(object.kind, NodeKind::Identifier { .. })
        {
            *saw_new = true;
        }
        node.for_each_child(|c| walk_indirect(c, saw_new));
    }
    walk_indirect(&ast, &mut saw_new);
    assert!(
        saw_new,
        "Expected `new Player ...` to surface as IndirectCall {{ method: \"new\", object: Identifier, .. }}"
    );

    // `print $log "..."` — object is a Variable.
    let mut saw_print_fh = false;
    fn walk_print_fh(node: &Node, saw: &mut bool) {
        if let NodeKind::IndirectCall { method, object, .. } = &node.kind
            && method == "print"
            && matches!(object.kind, NodeKind::Variable { .. })
        {
            *saw = true;
        }
        node.for_each_child(|c| walk_print_fh(c, saw));
    }
    walk_print_fh(&ast, &mut saw_print_fh);
    assert!(
        saw_print_fh,
        "Expected `print $log \"...\"` to surface as IndirectCall with Variable object"
    );
}

/// Labels on bare blocks (`LABEL: { ... }`) drive `last`/`redo` from the
/// block itself — a documented Perl construct often missed by parsers.
/// Loop control with explicit label targets must resolve to the same labels.
#[test]
fn test_labeled_bare_blocks_with_loop_control() {
    let code = r#"
my $attempts = 0;

ATTEMPT: {
    $attempts++;
    last ATTEMPT if $attempts > 5;
    redo ATTEMPT if rand() < 0.5;
    # falls off the end normally
}

# Nested labeled bare blocks — inner `last OUTER` must escape both.
OUTER: {
    INNER: {
        last OUTER if condition();
        next INNER if other_condition();   # `next` on bare blocks is a no-op
                                            # but must still parse cleanly.
        do_work();
    }
    cleanup_after_inner();
}

# Label colliding with a normal identifier name — parser must keep them
# separate (no confusion with sub call).
SEARCH: {
    for my $row (@rows) {
        last SEARCH if $row->{terminal};
    }
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let labeled = count_nodes(&ast, &|k| matches!(k, NodeKind::LabeledStatement { .. }));
    assert_eq!(labeled, 4, "Expected 4 labeled bare blocks (ATTEMPT, OUTER, INNER, SEARCH)");

    // Collect every label name + verify the loop-control ops reference them.
    let mut label_names: std::collections::BTreeSet<String> = Default::default();
    fn walk_labels(n: &Node, out: &mut std::collections::BTreeSet<String>) {
        if let NodeKind::LabeledStatement { label, .. } = &n.kind {
            out.insert(label.clone());
        }
        n.for_each_child(|c| walk_labels(c, out));
    }
    walk_labels(&ast, &mut label_names);
    for expected in ["ATTEMPT", "OUTER", "INNER", "SEARCH"] {
        assert!(
            label_names.contains(expected),
            "Missing label `{expected}` — found {label_names:?}"
        );
    }

    // Loop-control statements with explicit labels.
    let labeled_control =
        count_nodes(&ast, &|k| matches!(k, NodeKind::LoopControl { label: Some(_), .. }));
    assert!(
        labeled_control >= 5,
        "Expected at least 5 labeled loop-control ops, got {labeled_control}"
    );

    // The bare block under ATTEMPT must contain a `redo` op.
    let mut saw_redo = false;
    fn walk_redo(n: &Node, saw: &mut bool) {
        if let NodeKind::LoopControl { op, label } = &n.kind
            && op == "redo"
            && label.as_deref() == Some("ATTEMPT")
        {
            *saw = true;
        }
        n.for_each_child(|c| walk_redo(c, saw));
    }
    walk_redo(&ast, &mut saw_redo);
    assert!(
        saw_redo,
        "Expected `redo ATTEMPT` to surface as LoopControl op=\"redo\" label=ATTEMPT"
    );
}

/// `goto` has three target shapes: bare label, sub reference (`goto &name`),
/// and arbitrary expression (`goto $coderef`). Each must parse as
/// `NodeKind::Goto { target }` with the right inner node.
#[test]
fn test_goto_target_shapes() {
    let code = r#"
sub dispatch {
    my ($cmd) = @_;

    goto BAIL if !defined $cmd;

    my $handler = $handlers{$cmd};
    goto $handler if ref($handler) eq 'CODE';

    goto &fallback_handler;

  BAIL:
    return "bailed";
}

sub fallback_handler {
    return "fallback";
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Three gotos total.
    let gotos = count_nodes(&ast, &|k| matches!(k, NodeKind::Goto { .. }));
    assert_eq!(gotos, 3, "Expected exactly 3 goto statements");

    // Inspect each goto target shape.
    let mut target_kinds: Vec<&'static str> = Vec::new();
    fn walk(n: &Node, out: &mut Vec<&'static str>) {
        if let NodeKind::Goto { target } = &n.kind {
            out.push(target.kind.kind_name());
        }
        n.for_each_child(|c| walk(c, out));
    }
    walk(&ast, &mut target_kinds);

    assert!(
        target_kinds.contains(&"Identifier"),
        "Expected `goto BAIL` target=Identifier; got {target_kinds:?}"
    );
    assert!(
        target_kinds.contains(&"Variable"),
        "Expected `goto $handler` target=Variable; got {target_kinds:?}"
    );
    assert!(
        target_kinds.iter().any(|k| matches!(*k, "Unary" | "FunctionCall" | "Identifier")),
        "Expected `goto &fallback_handler` target as Unary/FunctionCall/Identifier; got {target_kinds:?}"
    );
}

/// `tr///` (and its `y///` alias) accept four modifiers: `c` complement,
/// `d` delete, `s` squeeze, `r` return-without-modifying. Each must be
/// preserved in `NodeKind::Transliteration.modifiers`, including
/// combinations.
#[test]
fn test_transliteration_every_modifier() {
    let code = r#"
my $text   = "Hello, World!";

# Each modifier in isolation
my $vow_d  = $text =~ tr/aeiouAEIOU//d;          # delete vowels (in place, returns count)
my $cnt_s  = $text =~ tr/ //s;                   # squeeze runs of spaces
my $cnt_c  = $text =~ tr/a-zA-Z//c;              # complement: match non-letters
my $upper  = $text =~ tr/a-z/A-Z/r;              # return modified copy (non-destructive)

# Combined modifiers
my $munged = $text =~ tr/a-zA-Z/A-Za-z/cdsr;     # complement+delete+squeeze+return

# `y///` is the historical alias for `tr///`
my $count_y = $text =~ y/!,.//d;

# `tr` with bracketed delimiters
$text =~ tr{aeiou}{AEIOU};
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Collect every Transliteration node and its modifiers.
    let mut all_modifiers: Vec<String> = Vec::new();
    fn walk(n: &Node, out: &mut Vec<String>) {
        if let NodeKind::Transliteration { modifiers, .. } = &n.kind {
            out.push(modifiers.clone());
        }
        n.for_each_child(|c| walk(c, out));
    }
    walk(&ast, &mut all_modifiers);

    assert_eq!(all_modifiers.len(), 7, "Expected 7 Transliteration nodes — got {all_modifiers:?}");

    let joined: String = all_modifiers.join("");
    for required in ['c', 'd', 's', 'r'] {
        assert!(
            joined.contains(required),
            "Modifier `{required}` missing from transliteration coverage: {all_modifiers:?}"
        );
    }

    // The combined-modifier case must carry all four flags simultaneously.
    let combined = all_modifiers
        .iter()
        .find(|m| m.contains('c') && m.contains('d') && m.contains('s') && m.contains('r'));
    assert!(combined.is_some(), "Expected one Transliteration with `cdsr` — got {all_modifiers:?}");
}

/// Typeglob aliasing patterns: `*alias = \&original` (sub alias),
/// `*Pkg::name = ...` (package-qualified), and `*FOO = *BAR` (full slot
/// copy). Each must surface a `Typeglob { name }` node on either side of
/// the assignment.
#[test]
fn test_typeglob_aliasing_patterns() {
    let code = r#"
package Original;
sub greet { return "hello" }
our $config = 42;
our @items  = (1, 2, 3);

package main;

# Sub alias: *alias = \&Original::greet
*alias_greet = \&Original::greet;

# Package-qualified destination
*Helpers::say_hi = \&Original::greet;

# Full typeglob copy — every slot (SCALAR, ARRAY, HASH, CODE, IO, FORMAT)
*Mirror = *Original::greet;

# Cross-package alias both sides qualified
*Other::name = *Original::name;

# Selective slot assignment (legal but rare)
*scalar_alias = \$Original::config;
*array_alias  = \@Original::items;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let typeglob_count = count_nodes(&ast, &|k| matches!(k, NodeKind::Typeglob { .. }));
    assert!(
        typeglob_count >= 7,
        "Expected at least 7 Typeglob nodes (5 LHS + 2 RHS at minimum), got {typeglob_count}"
    );

    // Verify both bare and package-qualified names surface.
    let mut names: Vec<String> = Vec::new();
    fn walk(n: &Node, out: &mut Vec<String>) {
        if let NodeKind::Typeglob { name } = &n.kind {
            out.push(name.clone());
        }
        n.for_each_child(|c| walk(c, out));
    }
    walk(&ast, &mut names);

    assert!(
        names.iter().any(|n| n == "alias_greet"),
        "Bare typeglob `alias_greet` missing; got {names:?}"
    );
    assert!(
        names.iter().any(|n| n.contains("::")),
        "Package-qualified typeglob (containing `::`) missing; got {names:?}"
    );

    // Each typeglob aliasing line is an Assignment.
    let assigns = count_nodes(&ast, &|k| matches!(k, NodeKind::Assignment { .. }));
    assert!(assigns >= 5, "Expected at least 5 typeglob assignments, got {assigns}");
}

/// All five compile/runtime phase blocks (`BEGIN`, `END`, `CHECK`, `INIT`,
/// `UNITCHECK`) must each surface as `NodeKind::PhaseBlock` with the right
/// `phase` field — distinguishing them from `Block` and from one another.
#[test]
fn test_all_phase_block_variants() {
    let code = r#"
BEGIN {
    $main::loaded = 1;
}

UNITCHECK {
    warn "unit-check phase\n" if $ENV{TRACE};
}

CHECK {
    warn "check phase\n" if $ENV{TRACE};
}

INIT {
    warn "init phase\n" if $ENV{TRACE};
}

END {
    warn "exit phase\n" if $ENV{TRACE};
}

# Multiple BEGINs in the same compilation unit — each is a separate phase block.
BEGIN { $main::extra = 'second BEGIN' }
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut phases: Vec<String> = Vec::new();
    fn walk(n: &Node, out: &mut Vec<String>) {
        if let NodeKind::PhaseBlock { phase, .. } = &n.kind {
            out.push(phase.clone());
        }
        n.for_each_child(|c| walk(c, out));
    }
    walk(&ast, &mut phases);

    assert_eq!(
        phases.len(),
        6,
        "Expected 6 PhaseBlocks (5 unique phases + 2nd BEGIN), got {phases:?}"
    );
    for expected in ["BEGIN", "END", "CHECK", "INIT", "UNITCHECK"] {
        assert!(phases.iter().any(|p| p == expected), "Missing phase `{expected}` in {phases:?}");
    }
    assert_eq!(phases.iter().filter(|p| *p == "BEGIN").count(), 2, "Expected two BEGIN blocks");
}

/// Disambiguate the angle-bracket constructs:
///   * `<>` — bare diamond → `NodeKind::Diamond`
///   * `<STDIN>` / `<FH>` — bareword filehandle readline → `NodeKind::Readline`
///   * `<*.pat>` — file glob → `NodeKind::Glob { pattern }`
///   * `<$fh>` — scalar-filehandle readline (an ambiguous Perl construct: the
///     reference parser resolves at compile time by checking whether `$fh` is
///     a filehandle, but a static parser cannot know that, so it picks one
///     surface representation). The current implementation classifies
///     `<$fh>` as `Glob { pattern: "$fh" }`; this test locks that behavior
///     in to prevent silent regression and to make any future reclassification
///     explicit.
#[test]
fn test_angle_bracket_variants_disambiguation() {
    let code = r#"
# Bare diamond — reads from @ARGV.
while (my $line = <>) {
    chomp $line;
}

# Bareword filehandle readline — two forms.
my $first = <STDIN>;
open FH, '<', '/etc/hostname' or die $!;
my $host = <FH>;
close FH;

# Explicit glob pattern.
my @pms     = <*.pm>;
my @configs = <conf/*.ini>;

# Scalar-handle form — currently classified as Glob (see test doc).
open my $fh, '<', '/etc/hosts' or die $!;
my $line = <$fh>;
close $fh;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let diamonds = count_nodes(&ast, &|k| matches!(k, NodeKind::Diamond));
    assert_eq!(diamonds, 1, "Expected exactly 1 Diamond node");

    let readlines = count_nodes(&ast, &|k| matches!(k, NodeKind::Readline { .. }));
    assert_eq!(readlines, 2, "Expected 2 Readline nodes (<STDIN> and <FH>)");

    // Three Globs: <*.pm>, <conf/*.ini>, and <$fh> (parser quirk).
    let globs = count_nodes(&ast, &|k| matches!(k, NodeKind::Glob { .. }));
    assert_eq!(globs, 3, "Expected 3 Glob nodes (<*.pm>, <conf/*.ini>, <$fh>)");

    // Verify Readline filehandle text is preserved for both bareword forms.
    let mut handles: Vec<Option<String>> = Vec::new();
    fn walk_rl(n: &Node, out: &mut Vec<Option<String>>) {
        if let NodeKind::Readline { filehandle } = &n.kind {
            out.push(filehandle.clone());
        }
        n.for_each_child(|c| walk_rl(c, out));
    }
    walk_rl(&ast, &mut handles);
    assert!(
        handles.iter().any(|h| h.as_deref() == Some("STDIN")),
        "Readline for <STDIN> should preserve filehandle=Some(\"STDIN\"), got {handles:?}"
    );
    assert!(
        handles.iter().any(|h| h.as_deref() == Some("FH")),
        "Readline for <FH> should preserve filehandle=Some(\"FH\"), got {handles:?}"
    );

    // Verify Glob patterns are preserved, including the `$fh` quirk form.
    let mut patterns: Vec<String> = Vec::new();
    fn walk_glob(n: &Node, out: &mut Vec<String>) {
        if let NodeKind::Glob { pattern } = &n.kind {
            out.push(pattern.clone());
        }
        n.for_each_child(|c| walk_glob(c, out));
    }
    walk_glob(&ast, &mut patterns);
    assert!(patterns.iter().any(|p| p.contains("*.pm")), "Glob `*.pm` missing in {patterns:?}");
    assert!(patterns.iter().any(|p| p.contains("*.ini")), "Glob `*.ini` missing in {patterns:?}");
    assert!(
        patterns.iter().any(|p| p == "$fh"),
        "<$fh> should currently parse as Glob {{ pattern: \"$fh\" }} (see test doc); got {patterns:?}"
    );
}

/// Statement modifiers share a compact grammar across conditionals and loops.
/// This locks each supported modifier keyword to `NodeKind::StatementModifier`
/// so future parser changes cannot silently collapse one spelling into a plain
/// call or binary expression.
#[test]
fn test_statement_modifier_keyword_matrix() {
    let code = r#"
print "ready" if $ready;
print "missing" unless $ready;
$tries++ while $tries < 3;
$waited++ until $done;
print $item for @items;
print $entry foreach @entries;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut modifiers: std::collections::BTreeMap<String, usize> = Default::default();
    fn walk(n: &Node, out: &mut std::collections::BTreeMap<String, usize>) {
        if let NodeKind::StatementModifier { modifier, statement, condition } = &n.kind {
            *out.entry(modifier.clone()).or_default() += 1;
            assert!(
                !matches!(statement.kind, NodeKind::MissingStatement | NodeKind::UnknownRest),
                "StatementModifier `{modifier}` must keep its statement subtree"
            );
            assert!(
                !matches!(condition.kind, NodeKind::MissingExpression | NodeKind::UnknownRest),
                "StatementModifier `{modifier}` must keep its condition/list subtree"
            );
        }
        n.for_each_child(|c| walk(c, out));
    }
    walk(&ast, &mut modifiers);

    for expected in ["if", "unless", "while", "until", "for", "foreach"] {
        assert_eq!(
            modifiers.get(expected).copied().map_or(0, |count| count),
            1,
            "Missing exactly one StatementModifier `{expected}` in {modifiers:?}"
        );
    }
    assert_eq!(
        modifiers.values().sum::<usize>(),
        6,
        "Unexpected modifier inventory: {modifiers:?}"
    );
}

/// Signature nodes are only useful to downstream semantic analysis if the
/// parser preserves each parameter shape instead of reducing the whole
/// signature to a flat token list.  Cover mandatory, optional, slurpy, and
/// named-parameter forms.
#[test]
fn test_signature_parameter_nodekind_shapes() {
    let code = r#"
use feature 'signatures';
sub configure($required, $optional = 10, @rest, :$named) {
    return $required + $optional;
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let signature_count = count_nodes(&ast, &|k| matches!(k, NodeKind::Signature { .. }));
    assert!(
        signature_count >= 1,
        "Expected at least the subroutine signature to surface, got {signature_count}"
    );

    let mandatory = count_nodes(&ast, &|k| matches!(k, NodeKind::MandatoryParameter { .. }));
    let optional = count_nodes(&ast, &|k| matches!(k, NodeKind::OptionalParameter { .. }));
    let slurpy = count_nodes(&ast, &|k| matches!(k, NodeKind::SlurpyParameter { .. }));
    let named = count_nodes(&ast, &|k| matches!(k, NodeKind::NamedParameter { .. }));

    assert!(mandatory >= 1, "Expected mandatory parameters, got {mandatory}");
    assert!(optional >= 1, "Expected optional parameter with default, got {optional}");
    assert!(slurpy >= 1, "Expected slurpy parameter, got {slurpy}");
    assert_eq!(named, 1, "Expected :$named to surface as NamedParameter");
}

/// Package declarations have both statement and inline-block forms.  Cover the
/// block-bearing package shape together with `use`/`no` arguments so provider
/// traversals can rely on the stored module metadata and package body.
#[test]
fn test_package_use_no_block_shapes() {
    let code = r#"
package Outer::One;
use strict;
no warnings 'experimental::signatures';

package Outer::Two {
    use feature qw(signatures class);
    no strict 'refs';

    sub inside($name) {
        return $name;
    }
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut package_names = Vec::new();
    let mut block_package_names = Vec::new();
    let mut use_modules = Vec::new();
    let mut no_modules = Vec::new();
    fn walk(
        n: &Node,
        packages: &mut Vec<String>,
        block_packages: &mut Vec<String>,
        uses: &mut Vec<(String, Vec<String>)>,
        nos: &mut Vec<(String, Vec<String>)>,
    ) {
        match &n.kind {
            NodeKind::Package { name, block, .. } => {
                packages.push(name.clone());
                if block.is_some() {
                    block_packages.push(name.clone());
                }
            }
            NodeKind::Use { module, args, .. } => uses.push((module.clone(), args.clone())),
            NodeKind::No { module, args, .. } => nos.push((module.clone(), args.clone())),
            _ => {}
        }
        n.for_each_child(|c| walk(c, packages, block_packages, uses, nos));
    }
    walk(&ast, &mut package_names, &mut block_package_names, &mut use_modules, &mut no_modules);

    assert!(
        package_names.iter().any(|name| name == "Outer::One"),
        "Missing statement package: {package_names:?}"
    );
    assert!(
        package_names.iter().any(|name| name == "Outer::Two"),
        "Missing block package: {package_names:?}"
    );
    assert_eq!(
        block_package_names,
        vec!["Outer::Two".to_string()],
        "Only package Outer::Two should own a block"
    );
    assert!(
        use_modules.iter().any(|(module, _)| module == "strict")
            && use_modules.iter().any(|(module, args)| {
                module == "feature" && args.iter().any(|arg| arg.contains("signatures"))
            }),
        "Expected strict and feature uses, got {use_modules:?}"
    );
    assert!(
        no_modules.iter().any(|(module, args)| module == "warnings"
            && args.iter().any(|arg| arg.contains("experimental::signatures")))
            && no_modules
                .iter()
                .any(|(module, args)| module == "strict"
                    && args.iter().any(|arg| arg.contains("refs"))),
        "Expected warnings/strict no declarations with arguments, got {no_modules:?}"
    );
}

/// Regex-family NodeKinds carry important flags beyond merely existing:
/// modifiers, negated bind operators, replacement text, and embedded-code
/// detection.  Exercise `Regex`, `Match`, and `Substitution` together.
#[test]
fn test_regex_family_payload_shapes() {
    let code = r#"
my $subject = "abc123";
my $compiled = qr/a(?{ track() })c/ix;
my $positive = $subject =~ /abc/i;
my $negative = $subject !~ /def/ms;
my $changed = $subject =~ s/(abc)(\d+)/$1-$2/ge;
my $unchanged = $subject !~ s/xyz/uvw/r;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut regexes = Vec::new();
    let mut matches = Vec::new();
    let mut substitutions = Vec::new();
    fn walk(
        n: &Node,
        regexes: &mut Vec<(String, String, bool)>,
        matches_out: &mut Vec<(String, String, bool)>,
        substitutions_out: &mut Vec<(String, String, String, bool)>,
    ) {
        match &n.kind {
            NodeKind::Regex { pattern, modifiers, has_embedded_code, .. } => {
                regexes.push((pattern.clone(), modifiers.clone(), *has_embedded_code));
            }
            NodeKind::Match { pattern, modifiers, negated, .. } => {
                matches_out.push((pattern.clone(), modifiers.clone(), *negated));
            }
            NodeKind::Substitution { pattern, replacement, modifiers, negated, .. } => {
                substitutions_out.push((
                    pattern.clone(),
                    replacement.clone(),
                    modifiers.clone(),
                    *negated,
                ));
            }
            _ => {}
        }
        n.for_each_child(|c| walk(c, regexes, matches_out, substitutions_out));
    }
    walk(&ast, &mut regexes, &mut matches, &mut substitutions);

    assert!(
        regexes.iter().any(|(pattern, modifiers, embedded)| {
            pattern.contains("track")
                && modifiers.contains('i')
                && modifiers.contains('x')
                && *embedded
        }),
        "Expected qr// with embedded code and ix modifiers, got {regexes:?}"
    );
    assert!(
        matches.iter().any(|(pattern, modifiers, negated)| {
            pattern.contains("abc") && modifiers.contains('i') && !negated
        }),
        "Expected positive match with abc pattern and i modifier, got {matches:?}"
    );
    assert!(
        matches.iter().any(|(pattern, modifiers, negated)| pattern.contains("def")
            && modifiers.contains('m')
            && modifiers.contains('s')
            && *negated),
        "Expected negated match with ms modifiers, got {matches:?}"
    );
    assert!(
        substitutions.iter().any(|(pattern, replacement, modifiers, negated)| {
            pattern.contains("abc")
                && replacement == "$1-$2"
                && modifiers.contains('g')
                && modifiers.contains('e')
                && !negated
        }),
        "Expected positive substitution payload, got {substitutions:?}"
    );
    assert!(
        substitutions.iter().any(|(pattern, replacement, modifiers, negated)| pattern == "xyz"
            && replacement == "uvw"
            && modifiers == "r"
            && *negated),
        "Expected negated substitution payload, got {substitutions:?}"
    );
}
