use perl_lsp_rs_core::providers::missing_module::ModuleSearchPathDisplay;
use perl_module::resolution::IncRoot;

pub(super) fn search_display_paths(roots: &[IncRoot]) -> Vec<ModuleSearchPathDisplay> {
    roots
        .iter()
        .map(|root| {
            ModuleSearchPathDisplay::new(
                root.path.to_string_lossy().into_owned(),
                root_source_label(&root.source),
            )
        })
        .collect()
}

fn root_source_label(source: &str) -> &'static str {
    match source {
        "use-lib-lexical" => "use lib",
        "workspace-include-paths" => "workspace includePaths",
        "perl5lib-env" => "PERL5LIB",
        "interpreter-startup-inc" => "interpreter startup @INC",
        _ => "unknown @INC source",
    }
}
