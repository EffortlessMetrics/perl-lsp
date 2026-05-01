use super::LspServer;
use perl_tdd_support::must_some;

#[test]
fn test_internal_pl_sv_yes_hover_from_sigiled_token() {
    let text = "print $PL_sv_yes;\n";
    let offset = must_some(text.find('$'));

    assert_eq!(LspServer::extract_special_variable(text, offset).as_deref(), Some("$PL_sv_yes"));

    let hover = must_some(LspServer::get_special_variable_hover("$PL_sv_yes"));
    let value = must_some(hover["contents"]["value"].as_str());
    assert!(value.contains("true scalar"), "hover should describe the shared true scalar: {value}");
}
