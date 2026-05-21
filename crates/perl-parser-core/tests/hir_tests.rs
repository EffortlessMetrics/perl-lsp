use perl_parser_core::hir::{
    COMPILE_EFFECT_MODEL_VERSION, CompileConfidence, CompileEffectFactKind, CompileEffectKind,
    CompileEffectSourceKind, CompileEnvironment, CompileEnvironmentBoundaryKind, CompileProvenance,
    DynamicBoundaryKind, FrameworkAdapterKind, FrameworkAdapterRegistry,
    FrameworkExportedSymbolKind, GlobSlotSource, HirFile, HirKind, IncRootKind,
    ModuleResolutionRoot, PragmaArgumentKind, RecoveryConfidence, ScopeGraph, StashConfidence,
    StashGraph, StashProvenance, lower_ast,
};
use perl_parser_core::{Node, NodeKind, Parser, SourceLocation};
use perl_semantic_facts::{
    AnchorId, Confidence, ExportSet, ExportTag, FileId, ImportKind, ImportSpec, ImportSymbols,
    Provenance,
};
use std::fs;

fn lower_source(source: &str) -> HirFile {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

fn render_hir(file: &HirFile) -> String {
    file.items.iter().map(render_item).collect::<Vec<_>>().join("\n")
}

fn render_item(item: &perl_parser_core::hir::HirItem) -> String {
    let package = item.package_context.as_deref().unwrap_or("<none>");
    let scope =
        item.scope_context.map(|id| id.index().to_string()).unwrap_or_else(|| "<none>".to_string());
    let name_anchor = if item.anchor.name_range.is_some() { "name" } else { "node" };
    let kind = match &item.kind {
        HirKind::PackageDecl(decl) => format!("PackageDecl {}", decl.name),
        HirKind::SubDecl(decl) => {
            let name = decl.name.as_deref().unwrap_or("<anonymous>");
            format!(
                "SubDecl {name} proto={} sig={} attrs={}",
                decl.has_prototype, decl.has_signature, decl.attribute_count
            )
        }
        HirKind::MethodDecl(decl) => format!(
            "MethodDecl {} sig={} attrs={}",
            decl.name, decl.has_signature, decl.attribute_count
        ),
        HirKind::UseDecl(decl) => format!(
            "UseDecl {} args={} filter={}",
            decl.module,
            decl.args.join(","),
            decl.has_filter_risk
        ),
        HirKind::RequireDecl(decl) => {
            let target = decl.target.as_deref().unwrap_or("<dynamic>");
            format!("RequireDecl {target} args={}", decl.arg_count)
        }
        HirKind::VariableDecl(decl) => {
            let variables = decl
                .variables
                .iter()
                .map(|variable| format!("{}{}", variable.sigil, variable.name))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "VariableDecl {} vars={} attrs={} init={} list={}",
                decl.declarator,
                variables,
                decl.attribute_count,
                decl.has_initializer,
                decl.is_list
            )
        }
        HirKind::CallExpr(expr) => {
            format!("CallExpr {} args={} form={:?}", expr.name, expr.arg_count, expr.form)
        }
        HirKind::MethodCallExpr(expr) => format!(
            "MethodCallExpr {} args={} object={}",
            expr.method, expr.arg_count, expr.object_kind
        ),
        HirKind::IndirectCallExpr(expr) => format!(
            "IndirectCallExpr {} args={} object={}",
            expr.method, expr.arg_count, expr.object_kind
        ),
        HirKind::BarewordExpr(expr) => format!("BarewordExpr {}", expr.name),
        HirKind::LiteralExpr(expr) => {
            let value = expr.value.as_deref().unwrap_or("<none>");
            let interpolated = expr
                .interpolated
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            let element_count = expr
                .element_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            let pair_count = expr
                .pair_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            format!(
                "LiteralExpr {:?} value={} interp={} elements={} pairs={}",
                expr.kind, value, interpolated, element_count, pair_count
            )
        }
        HirKind::BlockShell(shell) => format!("BlockShell statements={}", shell.statement_count),
        HirKind::DynamicBoundary(boundary) => {
            format!("DynamicBoundary {:?} reason={}", boundary.kind, boundary.reason)
        }
        _ => "UnknownFutureHirKind".to_string(),
    };
    format!(
        "{} {kind} pkg={package} recovery={:?} anchor={} via={name_anchor} scope={scope}",
        item.id.index(),
        item.recovery_confidence,
        item.anchor.node_kind
    )
}

fn render_scope_graph(graph: &ScopeGraph) -> String {
    let mut lines = Vec::new();
    lines.push("[scopes]".to_string());
    for scope in &graph.scopes {
        let parent =
            scope.parent.map(|id| id.index().to_string()).unwrap_or_else(|| "<none>".to_string());
        let package = scope.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{} {:?} parent={} pkg={}",
            scope.id.index(),
            scope.kind,
            parent,
            package
        ));
    }

    lines.push("[bindings]".to_string());
    for binding in &graph.bindings {
        let shadows = binding
            .shadows
            .map(|id| id.index().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let package = binding.package_context.as_deref().unwrap_or("<none>");
        let item = binding
            .declaration_item
            .map(|id| id.index().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        lines.push(format!(
            "{} {}{} {:?} scope={} pkg={} item={} shadows={}",
            binding.id.index(),
            binding.sigil,
            binding.name,
            binding.storage,
            binding.scope_id.index(),
            package,
            item,
            shadows
        ));
    }

    lines.push("[references]".to_string());
    for reference in &graph.references {
        let target = reference
            .resolved_binding
            .map(|id| id.index().to_string())
            .unwrap_or_else(|| "<unresolved>".to_string());
        lines.push(format!(
            "{}{} scope={} target={}",
            reference.sigil,
            reference.name,
            reference.scope_id.index(),
            target
        ));
    }

    lines.join("\n")
}

fn render_stash_graph(graph: &StashGraph) -> String {
    let mut lines = Vec::new();
    lines.push("[packages]".to_string());
    for package in &graph.packages {
        let item = package
            .declaration_item
            .map(|id| id.index().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        lines.push(format!(
            "{} item={} {:?} {:?}",
            package.package, item, package.provenance, package.confidence
        ));
    }

    lines.push("[slots]".to_string());
    for package in &graph.packages {
        for slot in &package.slots {
            let target = slot.alias_target.as_deref().unwrap_or("<none>");
            lines.push(format!(
                "{}::{} {:?} {:?} {:?} {:?} target={}",
                package.package,
                slot.name,
                slot.kind,
                slot.source,
                slot.provenance,
                slot.confidence,
                target
            ));
        }
    }

    lines.push("[inheritance]".to_string());
    for edge in &graph.inheritance_edges {
        lines.push(format!(
            "{} -> {} {:?} {:?} {:?}",
            edge.from_package, edge.to_package, edge.source, edge.provenance, edge.confidence
        ));
    }

    lines.push("[boundaries]".to_string());
    for boundary in &graph.dynamic_boundaries {
        let package = boundary.package.as_deref().unwrap_or("<unknown>");
        let symbol = boundary.symbol.as_deref().unwrap_or("<unknown>");
        lines.push(format!(
            "{}::{} {:?} {:?} {:?} reason={}",
            package,
            symbol,
            boundary.kind,
            boundary.provenance,
            boundary.confidence,
            boundary.reason
        ));
    }

    lines.join("\n")
}

fn render_compile_environment(environment: &CompileEnvironment) -> String {
    let mut lines = Vec::new();
    lines.push("[directives]".to_string());
    for directive in &environment.directives {
        let module = directive.module.as_deref().unwrap_or("<dynamic>");
        let package = directive.package_context.as_deref().unwrap_or("<none>");
        let scope = directive
            .scope_id
            .map(|id| id.index().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        lines.push(format!(
            "{:?} {} {:?} args={} scope={} pkg={} {:?} {:?}",
            directive.action,
            module,
            directive.kind,
            directive.args.join(","),
            scope,
            package,
            directive.provenance,
            directive.confidence
        ));
    }

    lines.push("[pragmas]".to_string());
    for effect in &environment.pragma_effects {
        let package = effect.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{} enabled={} kind={:?} args={} pkg={} {:?} {:?}",
            effect.pragma,
            effect.enabled,
            effect.argument_kind,
            effect.args.join(","),
            package,
            effect.provenance,
            effect.confidence
        ));
    }

    lines.push("[inc]".to_string());
    for root in &environment.inc_roots {
        let package = root.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{} {:?} {:?} pkg={} {:?} {:?}",
            root.path, root.action, root.kind, package, root.provenance, root.confidence
        ));
    }

    lines.push("[modules]".to_string());
    for request in &environment.module_requests {
        let target = request.target.as_deref().unwrap_or("<dynamic>");
        let package = request.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{} {:?} {:?} pkg={} {:?} {:?}",
            target,
            request.kind,
            request.resolution,
            package,
            request.provenance,
            request.confidence
        ));
    }

    lines.push("[phase-blocks]".to_string());
    for phase in &environment.phase_blocks {
        let package = phase.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{:?} pkg={} {:?} {:?}",
            phase.phase, package, phase.provenance, phase.confidence
        ));
    }

    lines.push("[boundaries]".to_string());
    for boundary in &environment.dynamic_boundaries {
        let package = boundary.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "{:?} pkg={} {:?} {:?} reason={}",
            boundary.kind, package, boundary.provenance, boundary.confidence, boundary.reason
        ));
    }

    lines.join("\n")
}

fn render_module_resolution_candidates(
    environment: &CompileEnvironment,
    supplied_roots: &[ModuleResolutionRoot],
) -> String {
    let mut lines = Vec::new();
    lines.push("[module-candidates]".to_string());

    for candidate in environment.module_resolution_candidates(supplied_roots) {
        let package = candidate.package_context.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "request={} target={} kind={:?} rel={} status={:?} roots={} pkg={} {:?} {:?}",
            candidate.request_index,
            candidate.target,
            candidate.request_kind,
            candidate.relative_path,
            candidate.status,
            candidate.roots.len(),
            package,
            candidate.provenance,
            candidate.confidence
        ));

        for root in candidate.roots {
            lines.push(format!(
                "root={} {:?} source={} candidate={} precedence={}",
                root.path, root.kind, root.source, root.candidate_path, root.precedence
            ));
        }
    }

    lines.join("\n")
}

fn render_resolved_module_resolution_candidates(
    environment: &CompileEnvironment,
    supplied_roots: &[ModuleResolutionRoot],
    path_exists: impl FnMut(&str) -> bool,
) -> String {
    let mut lines = Vec::new();
    lines.push("[module-candidates]".to_string());

    for candidate in environment.resolved_module_resolution_candidates(supplied_roots, path_exists)
    {
        let package = candidate.package_context.as_deref().unwrap_or("<none>");
        let resolved_path = candidate.resolved_path.as_deref().unwrap_or("<none>");
        lines.push(format!(
            "request={} target={} kind={:?} rel={} status={:?} resolved={} roots={} pkg={} {:?} {:?}",
            candidate.request_index,
            candidate.target,
            candidate.request_kind,
            candidate.relative_path,
            candidate.status,
            resolved_path,
            candidate.roots.len(),
            package,
            candidate.provenance,
            candidate.confidence
        ));

        for root in candidate.roots {
            lines.push(format!(
                "root={} {:?} source={} candidate={} precedence={}",
                root.path, root.kind, root.source, root.candidate_path, root.precedence
            ));
        }
    }

    lines.join("\n")
}

fn render_framework_facts(file: &HirFile) -> String {
    let graph = file.framework_facts();
    let mut lines = Vec::new();

    lines.push("[exports]".to_string());
    for fact in &graph.exported_symbols {
        let tag = fact.tag_name.as_deref().unwrap_or("<none>");
        let context = fact
            .visible_symbol
            .context
            .as_ref()
            .and_then(|context| context.source_module.as_deref())
            .unwrap_or("<none>");
        lines.push(format!(
            "{:?} {}::{} {:?} tag={} visible={:?} ctx={} source={:?}/{:?} fact={:?}/{:?}",
            fact.adapter,
            fact.package,
            fact.name,
            fact.kind,
            tag,
            fact.visible_symbol.source,
            context,
            fact.source_provenance,
            fact.source_confidence,
            fact.provenance,
            fact.confidence
        ));
    }

    lines.push("[boundaries]".to_string());
    for boundary in &graph.dynamic_boundaries {
        let package = boundary.package.as_deref().unwrap_or("<unknown>");
        let symbol = boundary.symbol.as_deref().unwrap_or("<unknown>");
        lines.push(format!(
            "{:?} {}::{} {:?} {:?}/{:?} reason={}",
            boundary.adapter,
            package,
            symbol,
            boundary.kind,
            boundary.provenance,
            boundary.confidence,
            boundary.reason
        ));
    }

    lines.join("\n")
}

fn render_compile_effects(file: &HirFile) -> String {
    file.compile_effects_with_source_hash(Some("fixture-source-sha".to_string()))
        .into_iter()
        .filter(|effect| {
            matches!(
                effect.kind,
                CompileEffectKind::DeclarePackage
                    | CompileEffectKind::SetPragmaState
                    | CompileEffectKind::AddIncludePath
                    | CompileEffectKind::RequestModule
                    | CompileEffectKind::ImportSymbols
                    | CompileEffectKind::AssignInheritance
                    | CompileEffectKind::AssignGlobAlias
                    | CompileEffectKind::DefineConstant
                    | CompileEffectKind::RegisterPrototype
                    | CompileEffectKind::EmitDynamicBoundary
            )
        })
        .map(|effect| {
            let name = effect.fact_name.as_deref().unwrap_or("<none>");
            let package = effect.package_context.as_deref().unwrap_or("<none>");
            let reason = effect.dynamic_reason.as_deref().unwrap_or("<none>");
            let hash = effect.source_hash.as_deref().unwrap_or("<none>");
            format!(
                "{} {:?} source={:?} fact={:?} name={} pkg={} model={} hash={} {:?}/{:?} reason={}",
                effect.ordinal,
                effect.kind,
                effect.source_kind,
                effect.fact_kind,
                name,
                package,
                effect.model_version,
                hash,
                effect.provenance,
                effect.confidence,
                reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_import_spec<'a>(
    specs: &'a [ImportSpec],
    module: &str,
) -> Result<&'a ImportSpec, Box<dyn std::error::Error>> {
    specs
        .iter()
        .find(|spec| spec.module == module)
        .ok_or_else(|| format!("expected ImportSpec for {module}").into())
}

fn find_export_set<'a>(
    sets: &'a [ExportSet],
    module: &str,
) -> Result<&'a ExportSet, Box<dyn std::error::Error>> {
    sets.iter()
        .find(|set| set.module_name.as_deref() == Some(module))
        .ok_or_else(|| format!("expected ExportSet for {module}").into())
}

#[test]
fn hir_lowers_first_slice_constructs_with_stable_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         use List::Util qw(sum);\n\
         require Other::Module;\n\
         sub greet ($) :lvalue { 1; }\n\
         method run { 1; }\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=1\n\
         1 UseDecl List::Util args=qw(sum) filter=false pkg=My::Module recovery=Parsed anchor=Use via=node scope=1\n\
         2 RequireDecl Other::Module args=1 pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         3 BarewordExpr Other::Module pkg=My::Module recovery=Parsed anchor=Identifier via=name scope=1\n\
         4 SubDecl greet proto=true sig=false attrs=1 pkg=My::Module recovery=Parsed anchor=Subroutine via=name scope=2\n\
         5 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=3\n\
         6 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=3\n\
         7 MethodDecl run sig=false attrs=0 pkg=My::Module recovery=Parsed anchor=Method via=node scope=4\n\
         8 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=5\n\
         9 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=5"
    );

    for (index, item) in file.items.iter().enumerate() {
        assert_eq!(item.id.index(), index as u32);
        assert!(item.range.end >= item.range.start, "HIR item range should be ordered: {:?}", item);
        assert_eq!(item.range, item.anchor.range);
    }

    assert_eq!(
        render_scope_graph(&file.scope_graph),
        "[scopes]\n\
         0 File parent=<none> pkg=<none>\n\
         1 Package parent=0 pkg=My::Module\n\
         2 Subroutine parent=1 pkg=My::Module\n\
         3 Block parent=2 pkg=My::Module\n\
         4 Method parent=1 pkg=My::Module\n\
         5 Block parent=4 pkg=My::Module\n\
         [bindings]\n\
         [references]"
    );

    Ok(())
}

#[test]
fn hir_lowers_variable_declarations_with_scope_graph_without_provider_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package My::Module;\n\
         my $scalar = 1;\n\
         our @EXPORT_OK;\n\
         state %cache;\n\
         local $temp = undef;\n\
         local *FH;\n\
         my ($first, undef, @rest) = @_;\n\
         open(my $fh, '<', $path);\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=1\n\
         1 VariableDecl my vars=$scalar attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         2 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=1\n\
         3 VariableDecl our vars=@EXPORT_OK attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         4 VariableDecl state vars=%cache attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         5 VariableDecl local vars=$temp attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         6 LiteralExpr Undef value=<none> interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Undef via=node scope=1\n\
         7 VariableDecl local vars=*FH attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         8 VariableDecl my vars=$first,@rest attrs=0 init=true list=true pkg=My::Module recovery=Parsed anchor=VariableListDeclaration via=node scope=1\n\
         9 LiteralExpr Undef value=<none> interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Undef via=node scope=1\n\
         10 CallExpr open args=1 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         11 LiteralExpr Array value=<none> interp=<none> elements=3 pairs=<none> pkg=My::Module recovery=Parsed anchor=ArrayLiteral via=node scope=1\n\
         12 VariableDecl my vars=$fh attrs=0 init=false list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         13 LiteralExpr String value='<' interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=1"
    );

    assert_eq!(
        render_scope_graph(&file.scope_graph),
        "[scopes]\n\
         0 File parent=<none> pkg=<none>\n\
         1 Package parent=0 pkg=My::Module\n\
         [bindings]\n\
         0 $scalar LexicalMy scope=1 pkg=My::Module item=1 shadows=<none>\n\
         1 @EXPORT_OK PackageOur scope=1 pkg=My::Module item=3 shadows=<none>\n\
         2 %cache LexicalState scope=1 pkg=My::Module item=4 shadows=<none>\n\
         3 $temp LocalizedPackage scope=1 pkg=My::Module item=5 shadows=<none>\n\
         4 *FH LocalizedPackage scope=1 pkg=My::Module item=7 shadows=<none>\n\
         5 $first LexicalMy scope=1 pkg=My::Module item=8 shadows=<none>\n\
         6 @rest LexicalMy scope=1 pkg=My::Module item=8 shadows=<none>\n\
         7 $fh LexicalMy scope=1 pkg=My::Module item=12 shadows=<none>\n\
         [references]\n\
         @_ scope=1 target=<unresolved>\n\
         $path scope=1 target=<unresolved>"
    );

    Ok(())
}

#[test]
fn hir_lowers_expression_shells_without_provider_cutover() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         helper($value, 42, \"x\");\n\
         $self->method($value);\n\
         new Widget 1;\n\
         $callback->(42);\n\
         grep { $_->ok } @items;\n\
         eval $source;\n\
         do $file;\n\
         my $settings = { foo => 1, bar => \"x\" };\n",
    );

    assert_eq!(
        render_hir(&file),
        "0 PackageDecl My::Module pkg=My::Module recovery=Parsed anchor=Package via=name scope=1\n\
         1 CallExpr helper args=3 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         2 LiteralExpr Number value=42 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=1\n\
         3 LiteralExpr String value=\"x\" interp=true elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=1\n\
         4 MethodCallExpr method args=1 object=Variable pkg=My::Module recovery=Parsed anchor=MethodCall via=node scope=1\n\
         5 IndirectCallExpr new args=1 object=Identifier pkg=My::Module recovery=Parsed anchor=IndirectCall via=node scope=1\n\
         6 BarewordExpr Widget pkg=My::Module recovery=Parsed anchor=Identifier via=name scope=1\n\
         7 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=1\n\
         8 DynamicBoundary CoderefCall reason=coderef or dynamic callee invoked through ->() pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         9 CallExpr ->() args=1 form=Coderef pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         10 LiteralExpr Number value=42 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=1\n\
         11 CallExpr grep args=2 form=NamedFunction pkg=My::Module recovery=Parsed anchor=FunctionCall via=node scope=1\n\
         12 BlockShell statements=1 pkg=My::Module recovery=Parsed anchor=Block via=node scope=2\n\
         13 MethodCallExpr ok args=0 object=Variable pkg=My::Module recovery=Parsed anchor=MethodCall via=node scope=2\n\
         14 DynamicBoundary EvalExpression reason=eval body is an expression rather than a parsed block pkg=My::Module recovery=Parsed anchor=Eval via=node scope=1\n\
         15 DynamicBoundary DoExpression reason=do body is an expression rather than a parsed block pkg=My::Module recovery=Parsed anchor=Do via=node scope=1\n\
         16 VariableDecl my vars=$settings attrs=0 init=true list=false pkg=My::Module recovery=Parsed anchor=VariableDeclaration via=name scope=1\n\
         17 LiteralExpr Hash value=<none> interp=<none> elements=<none> pairs=2 pkg=My::Module recovery=Parsed anchor=HashLiteral via=node scope=1\n\
         18 LiteralExpr String value=foo interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=1\n\
         19 LiteralExpr Number value=1 interp=<none> elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=Number via=node scope=1\n\
         20 LiteralExpr String value=bar interp=false elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=1\n\
         21 LiteralExpr String value=\"x\" interp=true elements=<none> pairs=<none> pkg=My::Module recovery=Parsed anchor=String via=node scope=1"
    );

    assert_eq!(
        render_scope_graph(&file.scope_graph),
        "[scopes]\n\
         0 File parent=<none> pkg=<none>\n\
         1 Package parent=0 pkg=My::Module\n\
         2 Block parent=1 pkg=My::Module\n\
         [bindings]\n\
         0 $settings LexicalMy scope=1 pkg=My::Module item=16 shadows=<none>\n\
         [references]\n\
         $value scope=1 target=<unresolved>\n\
         $self scope=1 target=<unresolved>\n\
         $value scope=1 target=<unresolved>\n\
         $callback scope=1 target=<unresolved>\n\
         $_ scope=2 target=<unresolved>\n\
         @items scope=1 target=<unresolved>\n\
         $source scope=1 target=<unresolved>\n\
         $file scope=1 target=<unresolved>"
    );

    Ok(())
}

#[test]
fn hir_scope_graph_resolves_lexicals_and_marks_shadowing() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source(
        "package My::Module;\n\
         use feature 'signatures';\n\
         my $value = 1;\n\
         sub run ($param) {\n\
             my $value = $param;\n\
             our $shared;\n\
             state $cache;\n\
             local $temp;\n\
             $value;\n\
             $shared;\n\
             $cache;\n\
             $temp;\n\
         }\n\
         $value;\n",
    );

    assert_eq!(
        render_scope_graph(&file.scope_graph),
        "[scopes]\n\
         0 File parent=<none> pkg=<none>\n\
         1 Package parent=0 pkg=My::Module\n\
         2 Subroutine parent=1 pkg=My::Module\n\
         3 Signature parent=2 pkg=My::Module\n\
         4 Block parent=3 pkg=My::Module\n\
         [bindings]\n\
         0 $value LexicalMy scope=1 pkg=My::Module item=2 shadows=<none>\n\
         1 $param Parameter scope=3 pkg=My::Module item=<none> shadows=<none>\n\
         2 $value LexicalMy scope=4 pkg=My::Module item=6 shadows=0\n\
         3 $shared PackageOur scope=4 pkg=My::Module item=7 shadows=<none>\n\
         4 $cache LexicalState scope=4 pkg=My::Module item=8 shadows=<none>\n\
         5 $temp LocalizedPackage scope=4 pkg=My::Module item=9 shadows=<none>\n\
         [references]\n\
         $param scope=4 target=1\n\
         $value scope=4 target=2\n\
         $shared scope=4 target=3\n\
         $cache scope=4 target=4\n\
         $temp scope=4 target=5\n\
         $value scope=1 target=0"
    );

    Ok(())
}

#[test]
fn hir_stash_graph_records_package_slots_inheritance_and_dynamic_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Child;\n\
         our @ISA = qw(Base::One Base::Two);\n\
         @Other::ISA = 'Other::Base';\n\
         use parent 'Parent::Base';\n\
         use base qw(Exporter Local::Base);\n\
         use constant PI => 3.14;\n\
         use constant LABEL => 'foo';\n\
         sub ANSWER () { 42; }\n\
         sub foo { 1; }\n\
         method run { 1; }\n\
         our $VERSION;\n\
         our @EXPORT_OK;\n\
         our %CACHE;\n\
         *alias = \\&foo;\n\
         *dynamic = $target;\n\
         sub AUTOLOAD { our $AUTOLOAD; }\n",
    );

    assert_eq!(
        render_stash_graph(&file.stash_graph),
        "[packages]\n\
         Child item=0 ExactAst High\n\
         Base::One item=<none> ExactAst High\n\
         Base::Two item=<none> ExactAst High\n\
         Other item=<none> ExactAst High\n\
         Other::Base item=<none> ExactAst High\n\
         Parent::Base item=<none> ExactAst High\n\
         Exporter item=<none> ExactAst High\n\
         Local::Base item=<none> ExactAst High\n\
         [slots]\n\
         Child::ISA Array OurDeclaration ExactAst High target=<none>\n\
         Child::PI Code ConstantDeclaration DesugaredAst High target=<none>\n\
         Child::LABEL Code ConstantDeclaration DesugaredAst High target=<none>\n\
         Child::ANSWER Code ConstantDeclaration ExactAst High target=<none>\n\
         Child::foo Code SubDeclaration ExactAst High target=<none>\n\
         Child::run Code MethodDeclaration ExactAst High target=<none>\n\
         Child::VERSION Scalar OurDeclaration ExactAst High target=<none>\n\
         Child::EXPORT_OK Array OurDeclaration ExactAst High target=<none>\n\
         Child::CACHE Hash OurDeclaration ExactAst High target=<none>\n\
         Child::alias Code TypeglobAlias ExactAst Medium target=foo\n\
         Child::AUTOLOAD Code SubDeclaration ExactAst High target=<none>\n\
         Child::AUTOLOAD Scalar OurDeclaration ExactAst High target=<none>\n\
         Other::ISA Array PackageAssignment ExactAst High target=<none>\n\
         [inheritance]\n\
         Child -> Base::One IsaAssignment ExactAst High\n\
         Child -> Base::Two IsaAssignment ExactAst High\n\
         Other -> Other::Base IsaAssignment ExactAst High\n\
         Child -> Parent::Base UseParent DesugaredAst High\n\
         Child -> Exporter UseBase DesugaredAst High\n\
         Child -> Local::Base UseBase DesugaredAst High\n\
         [boundaries]\n\
         Child::dynamic DynamicStashMutation DynamicBoundary Low reason=typeglob assignment has a non-static RHS\n\
         Child::AUTOLOAD Autoload DynamicBoundary Low reason=AUTOLOAD declaration makes method dispatch dynamic"
    );

    assert!(
        file.items.iter().any(|item| {
            matches!(
                &item.kind,
                HirKind::DynamicBoundary(boundary)
                    if boundary.reason == "typeglob assignment has a non-static RHS"
            )
        }),
        "dynamic typeglob mutation should emit a HIR dynamic boundary"
    );
    assert!(
        file.items.iter().any(|item| {
            matches!(
                &item.kind,
                HirKind::DynamicBoundary(boundary)
                    if boundary.reason == "AUTOLOAD declaration makes method dispatch dynamic"
            )
        }),
        "AUTOLOAD should emit a HIR dynamic boundary"
    );

    Ok(())
}

#[test]
fn hir_stash_graph_projects_constant_table_from_static_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Constants;\n\
         use constant PI => 3.14;\n\
         use constant { MIN => 1, MAX => 100 };\n\
         sub ANSWER () { 42; }\n\
         sub ordinary { 1; }\n\
         *alias = \\&ordinary;\n",
    );

    let table = file.stash_graph.constant_table();
    assert!(!table.is_empty(), "constant table should contain static constants");

    let names: Vec<_> = table.entries.iter().map(|entry| entry.canonical_name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Constants::PI", "Constants::MAX", "Constants::MIN", "Constants::ANSWER"]
    );
    assert!(
        !names.contains(&"Constants::ordinary"),
        "ordinary subs must not be promoted into the constant table"
    );
    assert!(
        !names.contains(&"Constants::alias"),
        "typeglob aliases must not be promoted into the constant table"
    );

    let pi = table
        .entries
        .iter()
        .find(|entry| entry.canonical_name == "Constants::PI")
        .ok_or("missing PI constant entry")?;
    assert_eq!(pi.package, "Constants");
    assert_eq!(pi.name, "PI");
    assert_eq!(pi.source, GlobSlotSource::ConstantDeclaration);
    assert_eq!(pi.provenance, StashProvenance::DesugaredAst);
    assert_eq!(pi.confidence, StashConfidence::High);

    let answer = table
        .entries
        .iter()
        .find(|entry| entry.canonical_name == "Constants::ANSWER")
        .ok_or("missing ANSWER constant entry")?;
    assert_eq!(answer.source, GlobSlotSource::ConstantDeclaration);
    assert_eq!(answer.provenance, StashProvenance::ExactAst);
    assert_eq!(answer.confidence, StashConfidence::High);

    Ok(())
}

#[test]
fn hir_compile_environment_projects_import_specs_from_directives()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Import::Demo;\n\
         use Foo;\n\
         use Empty ();\n\
         use Explicit qw(alpha beta);\n\
         use Brackets qw[one two];\n\
         use Braces qw{:all delta};\n\
         use Tags qw(:all gamma);\n\
         use constant PI => 3;\n\
         use Runtime @names;\n\
         use CodeImport &handler;\n\
         use GlobImport *slot;\n\
         require Foo::Bar;\n\
         require $runtime;\n",
    );

    let specs = file.compile_environment.import_specs(FileId(7));
    let foo = find_import_spec(&specs, "Foo")?;
    assert_eq!(foo.kind, ImportKind::Use);
    assert_eq!(foo.symbols, ImportSymbols::Default);
    assert_eq!(foo.provenance, Provenance::ExactAst);
    assert_eq!(foo.file_id, Some(FileId(7)));
    assert!(foo.scope_id.is_some(), "HIR import facts should preserve scope context");
    assert!(foo.span_start_byte.is_some(), "HIR import facts should preserve directive order");

    let empty = find_import_spec(&specs, "Empty")?;
    assert_eq!(empty.kind, ImportKind::UseEmpty);
    assert_eq!(empty.symbols, ImportSymbols::None);

    let explicit = find_import_spec(&specs, "Explicit")?;
    assert_eq!(explicit.kind, ImportKind::UseExplicitList);
    assert_eq!(
        explicit.symbols,
        ImportSymbols::Explicit(vec!["alpha".to_string(), "beta".to_string()])
    );

    let brackets = find_import_spec(&specs, "Brackets")?;
    assert_eq!(brackets.kind, ImportKind::UseExplicitList);
    assert_eq!(
        brackets.symbols,
        ImportSymbols::Explicit(vec!["one".to_string(), "two".to_string()])
    );

    let braces = find_import_spec(&specs, "Braces")?;
    assert_eq!(braces.kind, ImportKind::UseExplicitList);
    assert_eq!(
        braces.symbols,
        ImportSymbols::Mixed { tags: vec!["all".to_string()], names: vec!["delta".to_string()] }
    );

    let tags = find_import_spec(&specs, "Tags")?;
    assert_eq!(tags.kind, ImportKind::UseExplicitList);
    assert_eq!(
        tags.symbols,
        ImportSymbols::Mixed { tags: vec!["all".to_string()], names: vec!["gamma".to_string()] }
    );

    let constant = find_import_spec(&specs, "constant")?;
    assert_eq!(constant.kind, ImportKind::UseConstant);
    assert_eq!(constant.symbols, ImportSymbols::Explicit(vec!["PI".to_string()]));

    let runtime = find_import_spec(&specs, "Runtime")?;
    assert_eq!(runtime.kind, ImportKind::UseExplicitList);
    assert_eq!(runtime.symbols, ImportSymbols::Dynamic);
    assert_eq!(runtime.provenance, Provenance::DynamicBoundary);

    let code_import = find_import_spec(&specs, "CodeImport")?;
    assert_eq!(code_import.symbols, ImportSymbols::Dynamic);
    assert_eq!(code_import.provenance, Provenance::DynamicBoundary);

    let glob_import = find_import_spec(&specs, "GlobImport")?;
    assert_eq!(glob_import.symbols, ImportSymbols::Dynamic);
    assert_eq!(glob_import.provenance, Provenance::DynamicBoundary);

    let require = find_import_spec(&specs, "Foo::Bar")?;
    assert_eq!(require.kind, ImportKind::Require);
    assert_eq!(require.symbols, ImportSymbols::Default);

    let dynamic_require = specs
        .iter()
        .find(|spec| spec.kind == ImportKind::DynamicRequire)
        .ok_or("expected dynamic require ImportSpec")?;
    assert_eq!(dynamic_require.module, "");
    assert_eq!(dynamic_require.symbols, ImportSymbols::Dynamic);
    assert_eq!(dynamic_require.provenance, Provenance::DynamicBoundary);

    let starts = specs
        .iter()
        .map(|spec| spec.span_start_byte.ok_or("expected span_start_byte"))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(
        starts.windows(2).all(|window| window[0] <= window[1]),
        "import facts must keep source order"
    );

    Ok(())
}

#[test]
fn hir_compile_environment_skips_version_and_no_directives_for_import_specs()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "use 5.036;\n\
         no strict 'refs';\n\
         use Real::Module;\n",
    );

    let specs = file.compile_environment.import_specs(FileId(11));
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].module, "Real::Module");
    assert_eq!(specs[0].kind, ImportKind::Use);

    Ok(())
}

#[test]
fn hir_stash_graph_projects_export_sets_from_static_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Export::One;\n\
         use Exporter 'import';\n\
         our @EXPORT = qw(default_one default_two);\n\
         our @EXPORT_OK = ('optional_one', \"optional_two\", '$scalar_name');\n\
         our %EXPORT_TAGS = (\n\
             all => [qw(default_one default_two optional_one optional_two)],\n\
             vars => [qw($scalar_name)],\n\
         );\n\
         package Export::Two;\n\
         @EXPORT = qw(second_default);\n\
         @EXPORT_OK = qw(second_optional);\n\
         %EXPORT_TAGS = (only => [qw(second_default second_optional)]);\n",
    );

    let export_sets = file.stash_graph.export_sets();
    assert_eq!(export_sets.len(), 2);

    let first = find_export_set(&export_sets, "Export::One")?;
    assert_eq!(first.default_exports, vec!["default_one".to_string(), "default_two".to_string()]);
    assert_eq!(
        first.optional_exports,
        vec!["$scalar_name".to_string(), "optional_one".to_string(), "optional_two".to_string()]
    );
    assert_eq!(
        first.tags,
        vec![
            ExportTag {
                name: "all".to_string(),
                members: vec![
                    "default_one".to_string(),
                    "default_two".to_string(),
                    "optional_one".to_string(),
                    "optional_two".to_string()
                ],
            },
            ExportTag { name: "vars".to_string(), members: vec!["$scalar_name".to_string()] },
        ]
    );
    assert_eq!(first.provenance, Provenance::ExactAst);
    assert_eq!(first.confidence, Confidence::High);
    assert!(first.anchor_id.is_some(), "export facts should preserve a source anchor");

    let second = find_export_set(&export_sets, "Export::Two")?;
    assert_eq!(second.default_exports, vec!["second_default".to_string()]);
    assert_eq!(second.optional_exports, vec!["second_optional".to_string()]);
    assert_eq!(
        second.tags,
        vec![ExportTag {
            name: "only".to_string(),
            members: vec!["second_default".to_string(), "second_optional".to_string()],
        }]
    );
    assert_eq!(second.provenance, Provenance::ExactAst);
    assert_eq!(second.confidence, Confidence::High);

    Ok(())
}

#[test]
fn hir_framework_adapter_registry_projects_exporter_family_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Export::One;\n\
         use Exporter ();\n\
         our @EXPORT = qw(default_one default_two);\n\
         our @EXPORT_OK = ('optional_one', \"optional_two\");\n\
         our %EXPORT_TAGS = (\n\
             all => [qw(default_one default_two optional_one optional_two)],\n\
         );\n\
         package Export::TinyDemo;\n\
         use Exporter::Tiny qw(import);\n\
         our @EXPORT_OK = qw(tiny_optional);\n",
    );

    let registry = FrameworkAdapterRegistry::default();
    assert_eq!(registry.adapters(), &[FrameworkAdapterKind::ExporterFamily]);

    assert_eq!(
        render_framework_facts(&file),
        "[exports]\n\
         ExporterFamily Export::One::default_one Default tag=<none> visible=DefaultExport ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::default_two Default tag=<none> visible=DefaultExport ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::optional_one Optional tag=<none> visible=ExplicitImport ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::optional_two Optional tag=<none> visible=ExplicitImport ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::default_one TagMember tag=all visible=ExportTag ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::default_two TagMember tag=all visible=ExportTag ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::optional_one TagMember tag=all visible=ExportTag ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::One::optional_two TagMember tag=all visible=ExportTag ctx=Export::One source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         ExporterFamily Export::TinyDemo::tiny_optional Optional tag=<none> visible=ExplicitImport ctx=Export::TinyDemo source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         [boundaries]"
    );

    let graph = registry.project_file(&file);
    let optional = graph
        .exported_symbols
        .iter()
        .find(|fact| fact.name == "optional_one")
        .ok_or("expected optional_one framework fact")?;
    assert_eq!(optional.kind, FrameworkExportedSymbolKind::Optional);
    assert!(optional.declaration_item.is_some(), "framework fact should preserve HIR source item");
    let context =
        optional.visible_symbol.context.as_ref().ok_or("expected visible symbol context")?;
    assert_eq!(context.source_module.as_deref(), Some("Export::One"));
    assert!(
        context.source_export_anchor_id.is_some(),
        "visible symbol should preserve export anchor"
    );

    Ok(())
}

#[test]
fn hir_stash_graph_fails_closed_for_dynamic_export_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Dynamic::Exports;\n\
         our @EXPORT = qw(stable_default);\n\
         our @EXPORT_OK = @runtime;\n\
         our %EXPORT_TAGS = (all => $runtime);\n",
    );

    let export_sets = file.stash_graph.export_sets();
    let exports = find_export_set(&export_sets, "Dynamic::Exports")?;
    assert_eq!(exports.default_exports, vec!["stable_default".to_string()]);
    assert!(exports.optional_exports.is_empty());
    assert!(exports.tags.is_empty());

    let dynamic_export_boundaries = file
        .stash_graph
        .dynamic_boundaries
        .iter()
        .filter(|boundary| {
            boundary.package.as_deref() == Some("Dynamic::Exports")
                && boundary.reason.contains("non-static")
        })
        .count();
    assert_eq!(dynamic_export_boundaries, 2);

    Ok(())
}

#[test]
fn hir_framework_adapter_registry_fails_closed_for_dynamic_exporter_shapes()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Dynamic::Exports;\n\
         use Exporter 'import';\n\
         our @EXPORT = @runtime;\n\
         our @EXPORT_OK = qw(stable_optional);\n\
         our %EXPORT_TAGS = (all => $runtime);\n",
    );

    assert_eq!(
        render_framework_facts(&file),
        "[exports]\n\
         ExporterFamily Dynamic::Exports::stable_optional Optional tag=<none> visible=ExplicitImport ctx=Dynamic::Exports source=ExactAst/High fact=FrameworkSynthesis/Medium\n\
         [boundaries]\n\
         ExporterFamily Dynamic::Exports::EXPORT DynamicExportDeclaration DynamicBoundary/Low reason=export declaration has non-static members\n\
         ExporterFamily Dynamic::Exports::EXPORT_TAGS DynamicExportDeclaration DynamicBoundary/Low reason=export tag declaration has non-static members"
    );

    let graph = file.framework_facts();
    assert_eq!(graph.exported_symbols.len(), 1);
    assert_eq!(graph.dynamic_boundaries.len(), 2);
    assert!(
        graph.dynamic_boundaries.iter().all(|boundary| {
            boundary.provenance == Provenance::DynamicBoundary
                && boundary.confidence == Confidence::Low
        }),
        "dynamic Exporter shapes should stay fail-closed"
    );

    Ok(())
}

#[test]
fn hir_compile_environment_records_directives_without_provider_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Env::Demo;\n\
         use strict;\n\
         use warnings qw(all);\n\
         use feature 'signatures';\n\
         no warnings 'once';\n\
         use lib qw(lib t/lib);\n\
         no lib 'legacy/lib';\n\
         use parent 'Base::Class';\n\
         use constant ANSWER => 42;\n\
         require Other::Module;\n\
         require $dynamic;\n\
         BEGIN { require Runtime::Thing; }\n",
    );

    assert_eq!(
        render_compile_environment(&file.compile_environment),
        "[directives]\n\
         Use strict Strict args= scope=1 pkg=Env::Demo ExactAst High\n\
         Use warnings Warnings args=qw(all) scope=1 pkg=Env::Demo ExactAst High\n\
         Use feature Feature args='signatures' scope=1 pkg=Env::Demo ExactAst High\n\
         No warnings Warnings args='once' scope=1 pkg=Env::Demo ExactAst High\n\
         Use lib Lib args=qw(lib t/lib) scope=1 pkg=Env::Demo ExactAst High\n\
         No lib Lib args='legacy/lib' scope=1 pkg=Env::Demo ExactAst High\n\
         Use parent Inheritance args='Base::Class' scope=1 pkg=Env::Demo ExactAst High\n\
         Use constant Constant args=ANSWER,42 scope=1 pkg=Env::Demo ExactAst High\n\
         Require Other::Module Module args= scope=1 pkg=Env::Demo ExactAst High\n\
         Require <dynamic> Dynamic args= scope=1 pkg=Env::Demo ExactAst High\n\
         Require Runtime::Thing Module args= scope=3 pkg=Env::Demo ExactAst High\n\
         [pragmas]\n\
         strict enabled=true kind=Broad args= pkg=Env::Demo ExactAst High\n\
         warnings enabled=true kind=Categories args=all pkg=Env::Demo ExactAst High\n\
         feature enabled=true kind=Categories args=signatures pkg=Env::Demo ExactAst High\n\
         warnings enabled=false kind=Categories args=once pkg=Env::Demo ExactAst High\n\
         [inc]\n\
         lib Add UseLib pkg=Env::Demo ExactAst High\n\
         t/lib Add UseLib pkg=Env::Demo ExactAst High\n\
         legacy/lib Remove UseLib pkg=Env::Demo ExactAst High\n\
         [modules]\n\
         parent Use Deferred pkg=Env::Demo ExactAst High\n\
         Base::Class Parent Deferred pkg=Env::Demo ExactAst High\n\
         constant Use Deferred pkg=Env::Demo ExactAst High\n\
         Other::Module Require Deferred pkg=Env::Demo ExactAst High\n\
         <dynamic> Require Dynamic pkg=Env::Demo ExactAst Low\n\
         Runtime::Thing Require Deferred pkg=Env::Demo ExactAst High\n\
         [phase-blocks]\n\
         Begin pkg=Env::Demo ExactAst High\n\
         [boundaries]\n\
         DynamicRequire pkg=Env::Demo DynamicBoundary Low reason=require target is not statically known\n\
         PhaseBlockExecution pkg=Env::Demo DynamicBoundary Low reason=phase block compile-time execution is recorded but not evaluated"
    );

    assert!(
        !file.items.iter().any(
            |item| matches!(&item.kind, HirKind::CallExpr(expr) if expr.name == "provider_cutover")
        ),
        "compile-environment facts must not imply live provider cutover"
    );

    Ok(())
}

#[test]
fn hir_compile_effect_log_links_source_mutations_facts_and_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "package Effect::Demo;\n\
                  use strict;\n\
                  use lib 'lib';\n\
                  use parent 'Base::Class';\n\
                  use constant ANSWER => 42;\n\
                  require Foo::Bar;\n\
                  require $dynamic;\n\
                  sub proto ($) { 1; }\n\
                  sub literal () { 1; }\n\
                  method run { 1; }\n\
                  our @ISA = qw(Local::Base);\n\
                  *alias = \\&proto;\n\
                  *dynamic = $target;\n\
                  BEGIN { require Runtime::Thing; }\n";
    let file = lower_source(source);

    let effects = file.compile_effects_with_source_hash(Some("fixture-source-sha".to_string()));
    assert_eq!(file.prototype_table.facts.len(), 2, "named sub prototypes should be table-backed");
    let proto_fact = file
        .prototype_table
        .facts
        .iter()
        .find(|fact| fact.sub_name == "proto")
        .ok_or("expected source-backed proto prototype fact")?;
    assert_eq!(proto_fact.package_context.as_deref(), Some("Effect::Demo"));
    assert_eq!(proto_fact.content, "$");
    assert_eq!(&source[proto_fact.range.start..proto_fact.range.end], "($)");
    assert_eq!(
        proto_fact.declaration_range,
        file.items
            .iter()
            .find(|item| item.id == proto_fact.declaration_item)
            .ok_or("expected prototype declaration item")?
            .range
    );
    assert_eq!(proto_fact.scope_id.map(|scope| scope.index()), Some(2));
    assert_eq!(proto_fact.anchor_id, AnchorId(proto_fact.range.start as u64));
    assert_eq!(proto_fact.provenance, CompileProvenance::ExactAst);
    assert_eq!(proto_fact.confidence, CompileConfidence::High);
    let literal_fact = file
        .prototype_table
        .facts
        .iter()
        .find(|fact| fact.sub_name == "literal")
        .ok_or("expected source-backed empty prototype fact")?;
    assert_eq!(literal_fact.content, "");
    assert_eq!(&source[literal_fact.range.start..literal_fact.range.end], "()");
    assert!(
        effects.iter().all(|effect| {
            effect.model_version == COMPILE_EFFECT_MODEL_VERSION
                && effect.source_hash.as_deref() == Some("fixture-source-sha")
        }),
        "effect log should carry model version and caller source hash"
    );
    assert!(
        effects.windows(2).all(|pair| pair[0].ordinal < pair[1].ordinal),
        "compile effects should have stable sorted ordinals"
    );

    let has_effect = |kind: CompileEffectKind,
                      source_kind: CompileEffectSourceKind,
                      fact_kind: CompileEffectFactKind,
                      name: &str| {
        effects.iter().any(|effect| {
            effect.kind == kind
                && effect.source_kind == source_kind
                && effect.fact_kind == fact_kind
                && effect.fact_name.as_deref() == Some(name)
        })
    };

    assert!(has_effect(
        CompileEffectKind::DeclarePackage,
        CompileEffectSourceKind::PackageDecl,
        CompileEffectFactKind::Package,
        "Effect::Demo"
    ));
    assert!(has_effect(
        CompileEffectKind::SetPragmaState,
        CompileEffectSourceKind::CompileEnvironment,
        CompileEffectFactKind::PragmaState,
        "strict/warnings/feature"
    ));
    assert!(has_effect(
        CompileEffectKind::AddIncludePath,
        CompileEffectSourceKind::UseDirective,
        CompileEffectFactKind::IncludeRoot,
        "lib"
    ));
    assert!(has_effect(
        CompileEffectKind::RequestModule,
        CompileEffectSourceKind::UseDirective,
        CompileEffectFactKind::ModuleRequest,
        "Base::Class"
    ));
    assert!(has_effect(
        CompileEffectKind::ImportSymbols,
        CompileEffectSourceKind::UseDirective,
        CompileEffectFactKind::ImportSpec,
        "constant"
    ));
    assert!(has_effect(
        CompileEffectKind::AssignInheritance,
        CompileEffectSourceKind::StashGraph,
        CompileEffectFactKind::InheritanceEdge,
        "Effect::Demo->Base::Class"
    ));
    assert!(has_effect(
        CompileEffectKind::AssignInheritance,
        CompileEffectSourceKind::StashGraph,
        CompileEffectFactKind::InheritanceEdge,
        "Effect::Demo->Local::Base"
    ));
    assert!(has_effect(
        CompileEffectKind::DefineConstant,
        CompileEffectSourceKind::StashGraph,
        CompileEffectFactKind::Constant,
        "Effect::Demo::ANSWER"
    ));
    assert!(has_effect(
        CompileEffectKind::DefineConstant,
        CompileEffectSourceKind::StashGraph,
        CompileEffectFactKind::Constant,
        "Effect::Demo::literal"
    ));
    assert!(has_effect(
        CompileEffectKind::RegisterPrototype,
        CompileEffectSourceKind::SubDecl,
        CompileEffectFactKind::Prototype,
        "proto"
    ));
    let proto_effect = effects
        .iter()
        .find(|effect| {
            effect.kind == CompileEffectKind::RegisterPrototype
                && effect.fact_name.as_deref() == Some("proto")
        })
        .ok_or("expected prototype compile effect")?;
    assert_eq!(proto_effect.range, proto_fact.range);
    assert_eq!(proto_effect.source_item, Some(proto_fact.declaration_item));
    assert_eq!(proto_effect.fact_anchor_id, Some(proto_fact.anchor_id));
    assert!(has_effect(
        CompileEffectKind::AssignGlobAlias,
        CompileEffectSourceKind::TypeglobAssignment,
        CompileEffectFactKind::GlobSlot,
        "Effect::Demo::alias"
    ));

    let boundary_reasons = effects
        .iter()
        .filter(|effect| effect.kind == CompileEffectKind::EmitDynamicBoundary)
        .filter_map(|effect| effect.dynamic_reason.as_deref())
        .collect::<Vec<_>>();
    assert!(boundary_reasons.contains(&"require target is not statically known"));
    assert!(boundary_reasons.contains(&"typeglob assignment has a non-static RHS"));
    assert!(
        boundary_reasons
            .contains(&"phase block compile-time execution is recorded but not evaluated")
    );
    assert!(
        effects.iter().filter(|effect| effect.kind == CompileEffectKind::EmitDynamicBoundary).all(
            |effect| {
                effect.provenance == CompileProvenance::DynamicBoundary
                    && effect.confidence == CompileConfidence::Low
            }
        ),
        "unsupported compile-time behavior should stay fail-closed"
    );

    let rendered = render_compile_effects(&file);
    assert!(
        rendered.contains("EmitDynamicBoundary"),
        "rendered effect proof should expose dynamic-boundary rows"
    );

    Ok(())
}

#[test]
fn hir_symbolic_refs_emit_dynamic_boundaries_only_when_strict_refs_disabled()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"package Symbolic::Refs;
use strict;
${$strict_scalar} = 1;
{
    no strict 'refs';
    ${$loose_scalar} = 1;
    @{$loose_array} = ();
    %{$loose_hash} = ();
    &{$loose_code}();
}
${$strict_again} = 2;
*alias = \&target;
"#;
    let file = lower_source(source);

    let strict_scalar = source.find("${$strict_scalar}").ok_or("expected strict scalar deref")?;
    let strict_again = source.find("${$strict_again}").ok_or("expected restored strict deref")?;
    let loose_scalar = source.find("${$loose_scalar}").ok_or("expected loose scalar deref")?;
    let loose_array = source.find("@{$loose_array}").ok_or("expected loose array deref")?;
    let loose_hash = source.find("%{$loose_hash}").ok_or("expected loose hash deref")?;
    let loose_code = source.find("&{$loose_code}").ok_or("expected loose code deref")?;

    let symbolic_items = file
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::DynamicBoundary(boundary)
                if boundary.kind == DynamicBoundaryKind::SymbolicReferenceDeref =>
            {
                Some((item.range.start, boundary.reason.as_str()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        symbolic_items,
        vec![
            (loose_scalar, "symbolic reference dereference is not statically known"),
            (loose_array, "symbolic reference dereference is not statically known"),
            (loose_hash, "symbolic reference dereference is not statically known"),
            (loose_code, "symbolic reference dereference is not statically known"),
        ],
        "symbolic-reference HIR boundaries should follow the scoped no strict refs region"
    );

    let symbolic_boundaries = file
        .compile_environment
        .dynamic_boundaries
        .iter()
        .filter(|boundary| boundary.kind == CompileEnvironmentBoundaryKind::SymbolicReferenceDeref)
        .collect::<Vec<_>>();
    let boundary_starts =
        symbolic_boundaries.iter().map(|boundary| boundary.range.start).collect::<Vec<_>>();
    assert_eq!(boundary_starts, vec![loose_scalar, loose_array, loose_hash, loose_code]);
    assert!(
        symbolic_boundaries.iter().all(|boundary| {
            boundary.provenance == CompileProvenance::DynamicBoundary
                && boundary.confidence == CompileConfidence::Low
                && boundary.package_context.as_deref() == Some("Symbolic::Refs")
                && boundary.reason == "symbolic reference dereference is not statically known"
        }),
        "symbolic-reference facts should stay fail-closed with package context"
    );
    assert!(
        !boundary_starts.contains(&strict_scalar) && !boundary_starts.contains(&strict_again),
        "strict refs regions should not be marked as symbolic-reference boundaries"
    );

    let effects = file.compile_effects();
    assert!(
        effects.iter().any(|effect| {
            effect.kind == CompileEffectKind::EmitDynamicBoundary
                && effect.source_kind == CompileEffectSourceKind::SymbolicReferenceDeref
                && effect.fact_kind == CompileEffectFactKind::DynamicBoundary
                && effect.fact_name.as_deref() == Some("SymbolicReferenceDeref")
                && effect.dynamic_reason.as_deref()
                    == Some("symbolic reference dereference is not statically known")
                && effect.provenance == CompileProvenance::DynamicBoundary
                && effect.confidence == CompileConfidence::Low
        }),
        "compile-effect log should expose symbolic-reference boundary rows"
    );
    assert!(
        effects.iter().any(|effect| {
            effect.kind == CompileEffectKind::AssignGlobAlias
                && effect.fact_name.as_deref() == Some("Symbolic::Refs::alias")
        }),
        "static typeglob aliases should remain on the existing precise path"
    );
    assert!(
        !file.items.iter().any(
            |item| matches!(&item.kind, HirKind::CallExpr(expr) if expr.name == "provider_cutover")
        ),
        "symbolic-reference facts must not imply live provider cutover"
    );

    Ok(())
}

#[test]
fn hir_compile_environment_proves_scoped_pragma_state_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "package Env::Scoped;\n\
         use strict;\n\
         use warnings;\n\
         use feature 'say';\n\
         {\n\
             no strict 'refs';\n\
             no warnings 'once';\n\
             no feature 'say';\n\
             my $inside = 1;\n\
         }\n\
         my $outside = 1;\n\
         use strict $dynamic;\n";
    let file = lower_source(source);
    let environment = &file.compile_environment;

    let strict_offset = source.find("use strict;").ok_or("expected outer strict directive")?;
    let no_strict_offset = source.find("no strict 'refs';").ok_or("expected no strict")?;
    let inside_offset = source.find("my $inside").ok_or("expected inner marker")?;
    let outside_offset = source.find("my $outside").ok_or("expected outer marker")?;
    let dynamic_offset =
        source.find("use strict $dynamic").ok_or("expected dynamic strict directive")?;

    let strict_effect = environment
        .pragma_effects
        .iter()
        .find(|effect| effect.range.start == strict_offset && effect.pragma == "strict")
        .ok_or("expected broad strict effect")?;
    assert_eq!(strict_effect.argument_kind, PragmaArgumentKind::Broad);
    assert!(strict_effect.args.is_empty());
    assert_eq!(strict_effect.provenance, CompileProvenance::ExactAst);
    assert_eq!(strict_effect.confidence, CompileConfidence::High);

    let no_strict_effect = environment
        .pragma_effects
        .iter()
        .find(|effect| effect.range.start == no_strict_offset && effect.pragma == "strict")
        .ok_or("expected category strict effect")?;
    assert_eq!(no_strict_effect.argument_kind, PragmaArgumentKind::Categories);
    assert_eq!(no_strict_effect.args, vec!["refs".to_string()]);
    assert_eq!(no_strict_effect.range.start, no_strict_offset);
    assert!(strict_effect.range.start < no_strict_effect.range.start);

    let after_strict = environment
        .pragma_state_at(strict_offset)
        .ok_or("expected state after strict directive")?;
    assert_eq!(after_strict.anchor_id, AnchorId(strict_offset as u64));
    assert_eq!(after_strict.package_context.as_deref(), Some("Env::Scoped"));
    assert!(after_strict.scope_id.is_some());
    assert!(after_strict.strict_enabled());
    assert_eq!(after_strict.provenance, CompileProvenance::ExactAst);
    assert_eq!(after_strict.confidence, CompileConfidence::High);

    let inside_state =
        environment.pragma_state_at(inside_offset).ok_or("expected inner scoped pragma state")?;
    assert!(inside_state.strict_vars);
    assert!(inside_state.strict_subs);
    assert!(!inside_state.strict_refs);
    assert!(inside_state.warnings);
    assert!(!inside_state.warning_active("once"));
    assert!(!inside_state.has_feature("say"));
    assert!(
        inside_state.disabled_warning_categories.iter().any(|category| category == "once"),
        "category-specific no warnings should be visible in the state fact"
    );

    let outside_state = environment
        .pragma_state_at(outside_offset)
        .ok_or("expected restored outer pragma state")?;
    assert!(outside_state.strict_enabled());
    assert!(outside_state.warning_active("once"));
    assert!(outside_state.has_feature("say"));

    assert!(
        !environment.pragma_state_facts().iter().any(|fact| fact.range.start == dynamic_offset),
        "dynamic pragma arguments must not produce a confirmed state fact"
    );
    let dynamic_boundary = environment
        .dynamic_boundaries
        .iter()
        .find(|boundary| boundary.kind == CompileEnvironmentBoundaryKind::DynamicPragmaArgs)
        .ok_or("expected dynamic pragma boundary")?;
    assert_eq!(dynamic_boundary.range.start, dynamic_offset);
    assert_eq!(dynamic_boundary.package_context.as_deref(), Some("Env::Scoped"));
    assert_eq!(dynamic_boundary.provenance, CompileProvenance::DynamicBoundary);
    assert_eq!(dynamic_boundary.confidence, CompileConfidence::Low);

    assert!(
        !file.items.iter().any(
            |item| matches!(&item.kind, HirKind::CallExpr(expr) if expr.name == "provider_cutover")
        ),
        "compile-environment state facts must not imply live provider cutover"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_candidate_facts_preserve_root_sources_and_dynamic_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "package Env::Demo;\n\
         use lib 'lib';\n\
         use Foo::Bar;\n\
         no lib 'lib';\n\
         require Other::Thing;\n\
         require $dynamic;\n",
    );
    let supplied_roots = vec![
        ModuleResolutionRoot::new(
            "configured/lib",
            IncRootKind::Configured,
            "workspace-include-paths",
        ),
        ModuleResolutionRoot::new("env/lib", IncRootKind::Perl5Lib, "perl5lib-env"),
        ModuleResolutionRoot::new(
            "/usr/share/perl5",
            IncRootKind::SystemInc,
            "interpreter-startup-inc",
        ),
    ];

    assert_eq!(
        render_module_resolution_candidates(&file.compile_environment, &supplied_roots),
        "[module-candidates]\n\
         request=0 target=Foo::Bar kind=Use rel=Foo/Bar.pm status=CandidateBuilt roots=4 pkg=Env::Demo ExactAst High\n\
           root=lib UseLib source=use-lib-lexical candidate=lib/Foo/Bar.pm precedence=0\n\
           root=configured/lib Configured source=workspace-include-paths candidate=configured/lib/Foo/Bar.pm precedence=1\n\
           root=env/lib Perl5Lib source=perl5lib-env candidate=env/lib/Foo/Bar.pm precedence=2\n\
           root=/usr/share/perl5 SystemInc source=interpreter-startup-inc candidate=/usr/share/perl5/Foo/Bar.pm precedence=3\n\
         request=1 target=Other::Thing kind=Require rel=Other/Thing.pm status=CandidateBuilt roots=3 pkg=Env::Demo ExactAst High\n\
           root=configured/lib Configured source=workspace-include-paths candidate=configured/lib/Other/Thing.pm precedence=0\n\
           root=env/lib Perl5Lib source=perl5lib-env candidate=env/lib/Other/Thing.pm precedence=1\n\
           root=/usr/share/perl5 SystemInc source=interpreter-startup-inc candidate=/usr/share/perl5/Other/Thing.pm precedence=2"
    );

    assert_eq!(file.compile_environment.module_requests.len(), 3);
    assert_eq!(
        file.compile_environment.module_resolution_candidates(&supplied_roots).len(),
        2,
        "dynamic require must not produce fake candidate paths"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_candidate_facts_mark_static_requests_without_roots_as_not_found()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source("use Missing::Module;\n");

    assert_eq!(
        render_module_resolution_candidates(&file.compile_environment, &[]),
        "[module-candidates]\n\
         request=0 target=Missing::Module kind=Use rel=Missing/Module.pm status=NotFound roots=0 pkg=<none> ExactAst High"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_candidate_facts_preserve_path_like_require_targets()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source("require 'Local/Path.pm';\n");
    let supplied_roots =
        [ModuleResolutionRoot::new("lib", IncRootKind::Configured, "workspace-include-paths")];

    assert_eq!(
        render_module_resolution_candidates(&file.compile_environment, &supplied_roots),
        "[module-candidates]\n\
         request=0 target=Local/Path.pm kind=Require rel=Local/Path.pm status=CandidateBuilt roots=1 pkg=<none> ExactAst High\n\
         root=lib Configured source=workspace-include-paths candidate=lib/Local/Path.pm precedence=0"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_candidate_facts_match_use_lib_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source(
        "use lib 'older';\n\
         use lib qw(first second);\n\
         use Foo::Bar;\n",
    );

    assert_eq!(
        render_module_resolution_candidates(&file.compile_environment, &[]),
        "[module-candidates]\n\
         request=0 target=Foo::Bar kind=Use rel=Foo/Bar.pm status=CandidateBuilt roots=3 pkg=<none> ExactAst High\n\
         root=first UseLib source=use-lib-lexical candidate=first/Foo/Bar.pm precedence=0\n\
         root=second UseLib source=use-lib-lexical candidate=second/Foo/Bar.pm precedence=1\n\
         root=older UseLib source=use-lib-lexical candidate=older/Foo/Bar.pm precedence=2"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_candidate_facts_reject_path_traversal_targets()
-> Result<(), Box<dyn std::error::Error>> {
    let file = lower_source("require '../Secret.pm';\n");
    let supplied_roots =
        [ModuleResolutionRoot::new("lib", IncRootKind::Configured, "workspace-include-paths")];

    assert_eq!(
        render_module_resolution_candidates(&file.compile_environment, &supplied_roots),
        "[module-candidates]"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_outcomes_resolve_against_explicit_roots()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let workspace_root = dir.path().join("workspace");
    let env_root = dir.path().join("envlib");
    let system_root = dir.path().join("system");
    let lexical_module = workspace_root.join("lib/Foo/Bar.pm");
    let env_module = env_root.join("Env/Only.pm");

    let lexical_parent =
        lexical_module.parent().ok_or("expected lexical module parent directory")?;
    fs::create_dir_all(lexical_parent)?;
    fs::write(&lexical_module, "package Foo::Bar;\n1;\n")?;

    let env_parent = env_module.parent().ok_or("expected env module parent directory")?;
    fs::create_dir_all(env_parent)?;
    fs::write(&env_module, "package Env::Only;\n1;\n")?;
    fs::create_dir_all(&system_root)?;
    let workspace_root = workspace_root.to_string_lossy().replace('\\', "/");
    let env_root = env_root.to_string_lossy().replace('\\', "/");
    let system_root = system_root.to_string_lossy().replace('\\', "/");

    let file = lower_source(
        "package Env::Demo;\n\
         use lib 'lib';\n\
         use Foo::Bar;\n\
         require Env::Only;\n\
         require Missing::Thing;\n\
         require $dynamic;\n",
    );
    let supplied_roots = vec![
        ModuleResolutionRoot::new(
            workspace_root.clone(),
            IncRootKind::Configured,
            "workspace-include-paths",
        ),
        ModuleResolutionRoot::new(env_root.clone(), IncRootKind::Perl5Lib, "perl5lib-env"),
        ModuleResolutionRoot::new(
            system_root.clone(),
            IncRootKind::SystemInc,
            "interpreter-startup-inc",
        ),
    ];

    assert_eq!(
        render_resolved_module_resolution_candidates(
            &file.compile_environment,
            &supplied_roots,
            |candidate| {
                let path = if let Some(rest) = candidate.strip_prefix("lib/") {
                    std::path::PathBuf::from(&workspace_root).join("lib").join(rest)
                } else {
                    std::path::PathBuf::from(candidate)
                };
                path.is_file()
            },
        ),
        format!(
            "[module-candidates]\n\
             request=0 target=Foo::Bar kind=Use rel=Foo/Bar.pm status=Resolved resolved=lib/Foo/Bar.pm roots=4 pkg=Env::Demo ExactAst High\n\
             root=lib UseLib source=use-lib-lexical candidate=lib/Foo/Bar.pm precedence=0\n\
             root={workspace} Configured source=workspace-include-paths candidate={workspace}/Foo/Bar.pm precedence=1\n\
             root={env} Perl5Lib source=perl5lib-env candidate={env}/Foo/Bar.pm precedence=2\n\
             root={system} SystemInc source=interpreter-startup-inc candidate={system}/Foo/Bar.pm precedence=3\n\
             request=1 target=Env::Only kind=Require rel=Env/Only.pm status=Resolved resolved={env}/Env/Only.pm roots=4 pkg=Env::Demo ExactAst High\n\
             root=lib UseLib source=use-lib-lexical candidate=lib/Env/Only.pm precedence=0\n\
             root={workspace} Configured source=workspace-include-paths candidate={workspace}/Env/Only.pm precedence=1\n\
             root={env} Perl5Lib source=perl5lib-env candidate={env}/Env/Only.pm precedence=2\n\
             root={system} SystemInc source=interpreter-startup-inc candidate={system}/Env/Only.pm precedence=3\n\
             request=2 target=Missing::Thing kind=Require rel=Missing/Thing.pm status=NotFound resolved=<none> roots=4 pkg=Env::Demo ExactAst High\n\
             root=lib UseLib source=use-lib-lexical candidate=lib/Missing/Thing.pm precedence=0\n\
             root={workspace} Configured source=workspace-include-paths candidate={workspace}/Missing/Thing.pm precedence=1\n\
             root={env} Perl5Lib source=perl5lib-env candidate={env}/Missing/Thing.pm precedence=2\n\
             root={system} SystemInc source=interpreter-startup-inc candidate={system}/Missing/Thing.pm precedence=3",
            workspace = workspace_root,
            env = env_root,
            system = system_root
        )
    );

    assert_eq!(file.compile_environment.module_requests.len(), 4);
    let outcome_facts =
        file.compile_environment.resolved_module_resolution_candidates(&supplied_roots, |_| false);
    assert!(
        outcome_facts.iter().all(|candidate| {
            candidate.directive_item.is_some() && candidate.range.end >= candidate.range.start
        }),
        "resolved/not-found outcome facts should preserve source anchors"
    );
    assert_eq!(
        file.compile_environment
            .resolved_module_resolution_candidates(&supplied_roots, |_| true)
            .len(),
        3,
        "dynamic require must not produce fake resolution outcomes"
    );

    Ok(())
}

#[test]
fn hir_module_resolution_cache_key_tracks_roots_and_epoch() -> Result<(), Box<dyn std::error::Error>>
{
    let file = lower_source("package Cache::Demo;\nuse Foo::Bar;\n");
    let roots = vec![
        ModuleResolutionRoot::new("workspace/lib", IncRootKind::Configured, "configured"),
        ModuleResolutionRoot::new("env/lib", IncRootKind::Perl5Lib, "perl5lib"),
        ModuleResolutionRoot::new("system/lib", IncRootKind::SystemInc, "system-inc"),
    ];

    let candidate = file
        .compile_environment
        .module_resolution_candidates(&roots)
        .into_iter()
        .next()
        .ok_or("expected a static module-resolution candidate")?;
    let same_candidate = file
        .compile_environment
        .module_resolution_candidates(&roots)
        .into_iter()
        .next()
        .ok_or("expected a stable static module-resolution candidate")?;

    assert_eq!(candidate.cache_key(7), same_candidate.cache_key(7));
    assert_ne!(candidate.cache_key(7), candidate.cache_key(8));

    let reversed_roots = vec![
        ModuleResolutionRoot::new("system/lib", IncRootKind::SystemInc, "system-inc"),
        ModuleResolutionRoot::new("env/lib", IncRootKind::Perl5Lib, "perl5lib"),
        ModuleResolutionRoot::new("workspace/lib", IncRootKind::Configured, "configured"),
    ];
    let reversed_candidate = file
        .compile_environment
        .module_resolution_candidates(&reversed_roots)
        .into_iter()
        .next()
        .ok_or("expected a candidate for reordered roots")?;
    assert_ne!(candidate.cache_key(7), reversed_candidate.cache_key(7));

    let relabeled_roots = vec![
        ModuleResolutionRoot::new("workspace/lib", IncRootKind::Perl5Lib, "configured"),
        ModuleResolutionRoot::new("env/lib", IncRootKind::Perl5Lib, "perl5lib"),
        ModuleResolutionRoot::new("system/lib", IncRootKind::SystemInc, "system-inc"),
    ];
    let relabeled_candidate = file
        .compile_environment
        .module_resolution_candidates(&relabeled_roots)
        .into_iter()
        .next()
        .ok_or("expected a candidate for relabeled roots")?;
    assert_ne!(candidate.cache_key(7), relabeled_candidate.cache_key(7));

    let key = candidate.cache_key(7);
    assert_eq!(key.resolver_epoch, 7);
    let first_root = key.roots.first().ok_or("expected first root in cache key")?;
    assert_eq!(first_root.path, "workspace/lib");
    assert_eq!(first_root.kind, IncRootKind::Configured);
    assert_eq!(first_root.source, "configured");
    assert_eq!(first_root.candidate_path, "workspace/lib/Foo/Bar.pm");
    assert_eq!(first_root.precedence, 0);

    Ok(())
}

#[test]
fn hir_module_resolution_cache_invalidation_tracks_candidate_file_state()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("lib");
    fs::create_dir_all(&root)?;
    let root = root.to_string_lossy().replace('\\', "/");
    let module_path = std::path::PathBuf::from(&root).join("Foo/Bar.pm");

    let file = lower_source("package Cache::Demo;\nuse Foo::Bar;\n");
    let roots =
        vec![ModuleResolutionRoot::new(root.clone(), IncRootKind::Configured, "configured")];
    let candidate = file
        .compile_environment
        .module_resolution_candidates(&roots)
        .into_iter()
        .next()
        .ok_or("expected a static module-resolution candidate")?;

    let missing_state =
        candidate.cache_invalidation(11, |path| std::path::PathBuf::from(path).is_file());

    let module_parent = module_path.parent().ok_or("expected module parent directory")?;
    fs::create_dir_all(module_parent)?;
    fs::write(&module_path, "package Foo::Bar;\n1;\n")?;

    let resolved_state =
        candidate.cache_invalidation(11, |path| std::path::PathBuf::from(path).is_file());
    assert_eq!(missing_state.key, resolved_state.key);
    assert_ne!(missing_state.path_states, resolved_state.path_states);
    let resolved_path_state =
        resolved_state.path_states.first().ok_or("expected resolved candidate path state")?;
    assert!(resolved_path_state.exists);

    fs::remove_file(&module_path)?;

    let removed_state =
        candidate.cache_invalidation(11, |path| std::path::PathBuf::from(path).is_file());
    assert_eq!(resolved_state.key, removed_state.key);
    assert_ne!(resolved_state.path_states, removed_state.path_states);
    let removed_path_state =
        removed_state.path_states.first().ok_or("expected removed candidate path state")?;
    assert!(!removed_path_state.exists);

    Ok(())
}

#[test]
fn hir_module_resolution_cache_key_excludes_dynamic_requires() {
    let file = lower_source("package Cache::Demo;\nrequire $dynamic;\n");
    let roots =
        vec![ModuleResolutionRoot::new("workspace/lib", IncRootKind::Configured, "configured")];

    assert!(
        file.compile_environment.module_resolution_candidates(&roots).is_empty(),
        "dynamic require should not produce a static cache key candidate"
    );
    assert!(
        file.compile_environment.resolved_module_resolution_candidates(&roots, |_| true).is_empty(),
        "dynamic require should not produce a resolved cacheable outcome"
    );
}

#[test]
fn hir_marks_items_lowered_from_error_partials_as_recovered()
-> Result<(), Box<dyn std::error::Error>> {
    let loc = SourceLocation { start: 0, end: 12 };
    let partial = Node::new(
        NodeKind::Subroutine {
            name: Some("broken".to_string()),
            name_span: Some(SourceLocation { start: 4, end: 10 }),
            prototype: None,
            signature: None,
            attributes: Vec::new(),
            body: Box::new(Node::new(
                NodeKind::MissingBlock,
                SourceLocation { start: 11, end: 11 },
            )),
        },
        loc,
    );
    let ast = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Error {
                    message: "missing block".to_string(),
                    expected: Vec::new(),
                    found: None,
                    partial: Some(Box::new(partial)),
                },
                loc,
            )],
        },
        loc,
    );

    let file = lower_ast(&ast);
    let Some(item) = file.items.first() else {
        return Err("expected recovered HIR item".into());
    };

    assert_eq!(item.recovery_confidence, RecoveryConfidence::Recovered);
    assert_eq!(
        render_hir(&file),
        "0 SubDecl broken proto=false sig=false attrs=0 pkg=<none> recovery=Recovered anchor=Subroutine via=name scope=1"
    );

    Ok(())
}
