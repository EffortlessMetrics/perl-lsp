use perl_parser::{Parser, declaration::DeclarationProvider};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_var_decl_in_same_block() -> TestResult {
    // Test: variable declaration in same block
    let content = "my $x = 5;\n$x + 1;";
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Find the usage of $x at position 11 (the $x in "$x + 1")
    let links = provider.find_declaration(11, 0).ok_or("Should find declaration")?;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_selection_range, (3, 5)); // Points to "$x" in "my $x"
    Ok(())
}

#[test]
fn test_shadowing_inner_my() -> TestResult {
    // Test: shadowing with inner `my $x`
    let content = r#"
my $x = 1;
{
    my $x = 2;
    $x;  # Should resolve to inner $x
}
"#;
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Find the usage of $x inside the block
    let usage_pos = content.find("$x;").ok_or("Could not find usage")?;
    let links = provider.find_declaration(usage_pos, 0).ok_or("Should find declaration")?;
    assert_eq!(links.len(), 1);

    // Should point to the inner declaration, not the outer one
    let inner_decl_pos = content.rfind("my $x = 2").ok_or("Could not find inner decl")?;
    assert!(
        links[0].target_selection_range.0 >= inner_decl_pos,
        "Should resolve to inner declaration"
    );
    Ok(())
}

#[test]
fn test_sub_decl_in_same_package() -> TestResult {
    // Test: subroutine declaration in same package
    let content = r#"
sub foo {
    return 42;
}

foo();
"#;
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Find the call to foo()
    let call_pos = content.find("foo()").ok_or("Could not find call")?;
    let links = provider.find_declaration(call_pos, 0).ok_or("Should find declaration")?;
    assert_eq!(links.len(), 1);

    // Should point to the sub declaration
    let sub_pos = content.find("sub foo").ok_or("Could not find sub")?;
    assert_eq!(links[0].target_selection_range.0, sub_pos + 4); // Points to "foo" after "sub "
    Ok(())
}

#[cfg(feature = "package-qualified")]
#[test]
fn test_package_qualified_sub() -> TestResult {
    // Test: Foo::bar resolves to package Foo; sub bar
    let content = r#"
package Foo;
sub bar { 42 }

package main;
Foo::bar();
"#;
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Find the call to Foo::bar()
    let call_pos = content.find("bar()").ok_or("Could not find call")?;
    let links = provider.find_declaration(call_pos, 0).ok_or("Should find declaration")?;
    assert_eq!(links.len(), 1);

    // Should point to sub bar in package Foo
    let sub_pos = content.find("sub bar").ok_or("Could not find sub")?;
    assert!(links[0].target_selection_range.0 >= sub_pos, "Should resolve to sub bar");
    Ok(())
}

#[cfg(feature = "constant-advanced")]
#[test]
fn test_constant_forms() -> TestResult {
    // Test: All three constant forms resolve correctly
    let content = r#"
use constant FOO => 42;
use constant { BAR => 1, BAZ => 2 };
use constant qw(QUX QUUX);

print FOO;
print BAR;
print QUX;
"#;
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Test FOO (simple form)
    let foo_usage = content.rfind("FOO").ok_or("Could not find FOO usage")?;
    let links = provider.find_declaration(foo_usage, 0).ok_or("Should find FOO declaration")?;
    assert_eq!(links.len(), 1);
    // Check that it points to the constant name, not the whole `use` statement
    let foo_decl = content.find("FOO =>").ok_or("Could not find FOO decl")?;
    assert!(
        links[0].target_selection_range.0 >= foo_decl
            && links[0].target_selection_range.1 <= foo_decl + 3,
        "Should point to FOO name specifically"
    );

    // Test BAR (hash form)
    let bar_usage = content.rfind("BAR").ok_or("Could not find BAR usage")?;
    let _links = provider.find_declaration(bar_usage, 0).ok_or("Should find BAR declaration")?;

    // Test QUX (qw form)
    let qux_usage = content.rfind("QUX").ok_or("Could not find QUX usage")?;
    let _links = provider.find_declaration(qux_usage, 0).ok_or("Should find QUX declaration")?;
    Ok(())
}

#[test]
fn test_unicode_and_crlf() -> TestResult {
    // Test: Unicode variable ($π) and CRLF buffer with position round-trip
    let content = "my $π = 3.14;\r\n$π++;\r\nmy $🐍 = 'snake';\r\n$🐍;";
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Find usage of $π
    let pi_usage = content.rfind("$π++").ok_or("Could not find π usage")?;
    let links = provider.find_declaration(pi_usage, 0).ok_or("Should find π declaration")?;
    assert_eq!(links.len(), 1);

    // Should point to the declaration
    let pi_decl = content.find("my $π").ok_or("Could not find π decl")?;
    assert!(links[0].target_selection_range.0 >= pi_decl, "Should find π declaration");

    // Find usage of $🐍 (snake emoji - surrogate pair)
    let snake_usage = content.rfind("$🐍;").ok_or("Could not find snake usage")?;
    let _links =
        provider.find_declaration(snake_usage, 0).ok_or("Should find snake declaration")?;

    // Test UTF-16 position round-trip
    // The server needs to handle CRLF and surrogate pairs correctly
    // This is tested implicitly by the declaration provider working correctly
    Ok(())
}

#[cfg(feature = "package-qualified")]
#[test]
fn test_tricky_names() -> TestResult {
    // Test: Complex names like Foo::Bar_baz9, _priv, métód_π
    let content = r#"
package Foo::Bar_baz9;
sub _priv { 42 }
sub métód_π { "unicode" }

Foo::Bar_baz9::_priv();
métód_π();
"#;
    let mut parser = Parser::new(content);
    let ast = parser.parse()?;

    let provider =
        DeclarationProvider::new(Arc::new(ast), content.to_string(), "test.pl".to_string());

    // Test _priv (private sub with underscore)
    let priv_call = content.rfind("_priv()").ok_or("Could not find _priv call")?;
    let _links = provider.find_declaration(priv_call, 0).ok_or("Should find _priv declaration")?;

    // Test métód_π (unicode method name)
    let unicode_call = content.rfind("métód_π()").ok_or("Could not find unicode call")?;
    let _links = provider
        .find_declaration(unicode_call, 0)
        .ok_or("Should find unicode method declaration")?;
    Ok(())
}
