use perl_parser_core::{builtin_signatures, builtin_signatures_phf};

#[test]
fn builtin_signatures_contains_common_functions() -> Result<(), Box<dyn std::error::Error>> {
    let sigs = builtin_signatures::create_builtin_signatures();
    // Some well-known Perl builtins should be present
    assert!(sigs.contains_key("print"), "missing 'print'");
    assert!(sigs.contains_key("push"), "missing 'push'");
    assert!(sigs.contains_key("pop"), "missing 'pop'");
    assert!(sigs.contains_key("chomp"), "missing 'chomp'");
    assert!(sigs.contains_key("open"), "missing 'open'");
    assert!(sigs.contains_key("close"), "missing 'close'");
    Ok(())
}

#[test]
fn builtin_signatures_phf_contains_common_functions() -> Result<(), Box<dyn std::error::Error>> {
    let phf = &builtin_signatures_phf::BUILTIN_SIGS;
    assert!(phf.contains_key("print"), "missing 'print' in phf");
    assert!(phf.contains_key("push"), "missing 'push' in phf");
    assert!(phf.contains_key("chomp"), "missing 'chomp' in phf");
    Ok(())
}

#[test]
fn builtin_signatures_not_empty() -> Result<(), Box<dyn std::error::Error>> {
    let sigs = builtin_signatures::create_builtin_signatures();
    assert!(sigs.len() > 50, "expected >50 builtins, got {}", sigs.len());
    Ok(())
}

#[test]
fn phf_map_not_empty() -> Result<(), Box<dyn std::error::Error>> {
    let phf = &builtin_signatures_phf::BUILTIN_SIGS;
    assert!(!phf.is_empty());
    Ok(())
}
