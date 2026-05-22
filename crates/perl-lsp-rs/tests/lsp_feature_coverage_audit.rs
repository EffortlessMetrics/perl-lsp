//! LSP Feature Coverage Audit Tests
//!
//! Targeted tests that fill identified gaps in LSP feature coverage.
//! Each section addresses a specific feature area found to have thin
//! or missing test coverage during the audit.
//!
//! Coverage areas:
//! - Semantic tokens: empty files, multi-line Perl, Moose class patterns
//! - Inlay hints: empty files, multiple builtins, user-defined functions
//! - Document links: real LSP requests for use/require statements
//! - Selection ranges: nested scopes, multiple positions
//! - Real-world Perl patterns: Moose classes, DBI usage across features

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_server() -> LspServer {
    let server = LspServer::new();

    let init = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {
                "textDocument": {
                    "inlayHint": {}
                }
            }
        })),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
    };
    server.handle_request(init);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized);

    server
}

fn open_document(server: &LspServer, uri: &str, content: &str) {
    let notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
        id: None,
    };
    server.handle_request(notification);
}

/// Send a request and return the result field, or an error.
fn send_request(
    server: &LspServer,
    id: i64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((id) as i64)),
        method: method.to_string(),
        params: Some(params),
    };
    let response = server.handle_request(req).ok_or_else(|| format!("no response for {method}"))?;
    response.result.ok_or_else(|| format!("no result in response for {method}"))
}

// ===========================================================================
// Semantic Tokens
// ===========================================================================

#[test]
fn semantic_tokens_empty_document() -> TestResult {
    let server = setup_server();
    open_document(&server, "file:///empty.pl", "");

    let result = send_request(
        &server,
        10,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": "file:///empty.pl"}}),
    )?;

    let data = result["data"].as_array().ok_or("semantic tokens should have data array")?;
    assert!(
        data.is_empty(),
        "empty document should produce no semantic tokens, got {} tokens",
        data.len() / 5
    );

    Ok(())
}

#[test]
fn semantic_tokens_multiline_subroutine() -> TestResult {
    let server = setup_server();
    let content = r#"sub greet {
    my ($name) = @_;
    print "Hello, $name!\n";
    return 1;
}
"#;
    open_document(&server, "file:///multi.pl", content);

    let result = send_request(
        &server,
        10,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": "file:///multi.pl"}}),
    )?;

    let data = result["data"].as_array().ok_or("semantic tokens should have data array")?;

    // Must be valid 5-tuples
    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples, got {} elements", data.len());
    // Multi-line subroutine should produce several tokens
    assert!(
        data.len() / 5 >= 3,
        "multiline subroutine should produce at least 3 tokens, got {}",
        data.len() / 5
    );

    Ok(())
}

#[test]
fn semantic_tokens_moose_class() -> TestResult {
    let server = setup_server();
    let content = r#"package Animal;
use Moose;

has 'name' => (is => 'ro', isa => 'Str', required => 1);
has 'age'  => (is => 'rw', isa => 'Int', default  => 0);

sub speak {
    my ($self) = @_;
    return "...";
}

no Moose;
__PACKAGE__->meta->make_immutable;
1;
"#;
    open_document(&server, "file:///moose.pm", content);

    let result = send_request(
        &server,
        10,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": "file:///moose.pm"}}),
    )?;

    let data = result["data"].as_array().ok_or("semantic tokens should have data array")?;

    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples");
    // A Moose class with package, attributes, and subroutine should produce many tokens
    assert!(
        data.len() / 5 >= 5,
        "Moose class should produce at least 5 tokens, got {}",
        data.len() / 5
    );

    Ok(())
}

#[test]
fn semantic_tokens_invalid_perl_does_not_crash() -> TestResult {
    let server = setup_server();
    // Deliberately malformed Perl
    let content = "sub { { { my $x = ; } } }\n@#$%^&\n";
    open_document(&server, "file:///bad.pl", content);

    let result = send_request(
        &server,
        10,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": "file:///bad.pl"}}),
    )?;

    // Should return a valid response (possibly with empty data) rather than crashing
    let data = result["data"]
        .as_array()
        .ok_or("semantic tokens should have data array even for invalid input")?;
    // Validate 5-tuple encoding if any tokens are returned
    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples");

    Ok(())
}

// ===========================================================================
// Inlay Hints
// ===========================================================================

#[test]
fn inlay_hints_empty_document() -> TestResult {
    let server = setup_server();
    open_document(&server, "file:///empty_hints.pl", "");

    let result = send_request(
        &server,
        20,
        "textDocument/inlayHint",
        json!({
            "textDocument": {"uri": "file:///empty_hints.pl"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            }
        }),
    )?;

    let hints = result.as_array().ok_or("inlay hints should return an array")?;
    assert!(hints.is_empty(), "empty document should produce no inlay hints");

    Ok(())
}

#[test]
fn inlay_hints_multiple_builtins() -> TestResult {
    let server = setup_server();
    let content = r#"my @data = (3, 1, 4, 1, 5);
push(@data, 9, 2, 6);
my $joined = join(",", @data);
my $sub = substr("hello world", 0, 5);
splice(@data, 1, 2, 99);
"#;
    open_document(&server, "file:///builtins.pl", content);

    let result = send_request(
        &server,
        20,
        "textDocument/inlayHint",
        json!({
            "textDocument": {"uri": "file:///builtins.pl"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 10, "character": 0}
            }
        }),
    )?;

    let hints = result.as_array().ok_or("inlay hints should return an array")?;

    // Multiple builtin calls with named parameters should produce hints
    assert!(!hints.is_empty(), "builtins with multiple arguments should produce inlay hints");

    // Validate structure of each hint
    for hint in hints {
        assert!(hint.get("position").is_some(), "each hint must have a position");
        assert!(hint.get("label").is_some(), "each hint must have a label");
    }

    Ok(())
}

#[test]
fn inlay_hints_invalid_perl_does_not_crash() -> TestResult {
    let server = setup_server();
    let content = "sub broken {{ my $x = ;\n@! invalid perl }}\n";
    open_document(&server, "file:///bad_hints.pl", content);

    let result = send_request(
        &server,
        20,
        "textDocument/inlayHint",
        json!({
            "textDocument": {"uri": "file:///bad_hints.pl"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 10, "character": 0}
            }
        }),
    )?;

    // Should return valid response without crashing
    assert!(result.is_array(), "inlay hints should return an array even for invalid input");

    Ok(())
}

// ===========================================================================
// Document Links
// ===========================================================================

#[test]
fn document_links_use_statements() -> TestResult {
    let server = setup_server();
    let content = r#"use strict;
use warnings;
use File::Path qw(make_path);
use Data::Dumper;
require JSON::XS;
"#;
    open_document(&server, "file:///links.pl", content);

    let result = send_request(
        &server,
        30,
        "textDocument/documentLink",
        json!({"textDocument": {"uri": "file:///links.pl"}}),
    )?;

    let links = result.as_array().ok_or("document links should return an array")?;

    // use and require statements should generate links
    assert!(!links.is_empty(), "use/require statements should produce document links");

    // Validate link structure
    for link in links {
        assert!(link.get("range").is_some(), "each document link must have a range");
        // target may be resolved lazily, so it's optional here
    }

    Ok(())
}

#[test]
fn document_links_empty_document() -> TestResult {
    let server = setup_server();
    open_document(&server, "file:///empty_links.pl", "");

    let result = send_request(
        &server,
        30,
        "textDocument/documentLink",
        json!({"textDocument": {"uri": "file:///empty_links.pl"}}),
    )?;

    let links = result.as_array().ok_or("document links should return an array")?;
    assert!(links.is_empty(), "empty document should produce no document links");

    Ok(())
}

#[test]
fn document_links_multiple_module_forms() -> TestResult {
    let server = setup_server();
    let content = r#"use parent 'Exporter';
use base qw(Class::Accessor);
use Moose;
use Moo;
require Carp;
use Scalar::Util 'blessed';
"#;
    open_document(&server, "file:///modules.pl", content);

    let result = send_request(
        &server,
        30,
        "textDocument/documentLink",
        json!({"textDocument": {"uri": "file:///modules.pl"}}),
    )?;

    let links = result.as_array().ok_or("document links should return an array")?;

    // Various module forms should all produce links
    assert!(
        links.len() >= 3,
        "multiple use/require forms should produce at least 3 links, got {}",
        links.len()
    );

    Ok(())
}

// ===========================================================================
// Selection Ranges
// ===========================================================================

#[test]
fn selection_range_nested_scopes() -> TestResult {
    let server = setup_server();
    let content = r#"package MyApp;

sub process {
    my ($self, $data) = @_;
    if ($data) {
        while (my $item = shift @$data) {
            print $item;
        }
    }
    return 1;
}
"#;
    open_document(&server, "file:///scopes.pl", content);

    let result = send_request(
        &server,
        40,
        "textDocument/selectionRange",
        json!({
            "textDocument": {"uri": "file:///scopes.pl"},
            "positions": [
                {"line": 6, "character": 18}  // inside while body, deeply nested
            ]
        }),
    )?;

    let ranges = result.as_array().ok_or("selection ranges should return an array")?;

    assert!(!ranges.is_empty(), "nested scope position should produce selection ranges");

    // The innermost range should have a parent chain
    let first = &ranges[0];
    assert!(first.get("range").is_some(), "selection range must have a range field");

    // Walk the parent chain and count nesting depth
    let mut depth = 1;
    let mut current = first.get("parent");
    while let Some(parent) = current {
        if parent.is_object() {
            depth += 1;
            current = parent.get("parent");
        } else {
            break;
        }
    }

    // Deeply nested position should have at least 2 levels (body + enclosing scope)
    assert!(
        depth >= 2,
        "deeply nested position should have parent chain depth >= 2, got {}",
        depth
    );

    Ok(())
}

#[test]
fn selection_range_multiple_positions() -> TestResult {
    let server = setup_server();
    let content = r#"my $x = 1;
my $y = 2;
sub add { return $_[0] + $_[1] }
my $sum = add($x, $y);
"#;
    open_document(&server, "file:///multi_sel.pl", content);

    let result = send_request(
        &server,
        40,
        "textDocument/selectionRange",
        json!({
            "textDocument": {"uri": "file:///multi_sel.pl"},
            "positions": [
                {"line": 0, "character": 4},  // on $x
                {"line": 2, "character": 5},  // on 'add' in sub definition
                {"line": 3, "character": 15}  // on $x in add($x, $y)
            ]
        }),
    )?;

    let ranges = result.as_array().ok_or("selection ranges should return an array")?;

    // Should return one selection range per position
    assert_eq!(
        ranges.len(),
        3,
        "should return one selection range per position, got {}",
        ranges.len()
    );

    for (i, range) in ranges.iter().enumerate() {
        assert!(range.get("range").is_some(), "selection range {} must have a range field", i);
    }

    Ok(())
}

#[test]
fn selection_range_empty_document() -> TestResult {
    let server = setup_server();
    open_document(&server, "file:///empty_sel.pl", "");

    let result = send_request(
        &server,
        40,
        "textDocument/selectionRange",
        json!({
            "textDocument": {"uri": "file:///empty_sel.pl"},
            "positions": [
                {"line": 0, "character": 0}
            ]
        }),
    )?;

    let ranges = result.as_array().ok_or("selection ranges should return an array")?;

    // Even for empty document, should return one result per position
    assert_eq!(
        ranges.len(),
        1,
        "should return one selection range per position even for empty doc"
    );

    Ok(())
}

// ===========================================================================
// Folding Ranges - edge cases
// ===========================================================================

#[test]
fn folding_ranges_heredoc() -> TestResult {
    let server = setup_server();
    let content = r#"my $text = <<'END_TEXT';
This is a heredoc
that spans multiple
lines of text.
END_TEXT

my $html = <<"HTML";
<html>
<body>
<p>Hello</p>
</body>
</html>
HTML
"#;
    open_document(&server, "file:///heredoc.pl", content);

    let result = send_request(
        &server,
        50,
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": "file:///heredoc.pl"}}),
    )?;

    let ranges = result.as_array().ok_or("folding ranges should return an array")?;

    // Heredocs may or may not produce folding ranges depending on the parser.
    // This test verifies the server handles the request without errors.
    // If ranges are returned, validate their structure.
    for range in ranges {
        assert!(range.get("startLine").is_some(), "folding range must have startLine");
        assert!(range.get("endLine").is_some(), "folding range must have endLine");
    }

    Ok(())
}

#[test]
fn folding_ranges_pod_comments() -> TestResult {
    let server = setup_server();
    let content = r#"=head1 NAME

MyModule - A test module

=head1 SYNOPSIS

    use MyModule;
    my $obj = MyModule->new();

=head1 DESCRIPTION

This is a long description
that spans multiple lines.

=cut

sub new {
    my $class = shift;
    return bless {}, $class;
}
"#;
    open_document(&server, "file:///pod.pm", content);

    let result = send_request(
        &server,
        50,
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": "file:///pod.pm"}}),
    )?;

    let ranges = result.as_array().ok_or("folding ranges should return an array")?;

    // POD documentation and sub should produce folding ranges
    assert!(!ranges.is_empty(), "POD and subroutine should produce folding ranges");

    // Check for comment-kind folding ranges (POD)
    let has_comment_range =
        ranges.iter().any(|r| r.get("kind").and_then(|k| k.as_str()) == Some("comment"));
    // POD is typically folded as comments; if not, at least ensure region ranges exist
    if !has_comment_range {
        // At minimum, the subroutine should fold
        assert!(!ranges.is_empty(), "should have at least one folding range for subroutine");
    }

    Ok(())
}

// ===========================================================================
// Real-world Perl Patterns - cross-feature integration
// ===========================================================================

#[test]
fn moose_class_document_symbols() -> TestResult {
    let server = setup_server();
    let content = r#"package Animal;
use Moose;

has 'name'  => (is => 'ro', isa => 'Str', required => 1);
has 'sound' => (is => 'ro', isa => 'Str', default  => '...');

sub speak {
    my ($self) = @_;
    return $self->name . " says " . $self->sound;
}

sub describe {
    my ($self) = @_;
    return sprintf("Animal: %s", $self->name);
}

no Moose;
__PACKAGE__->meta->make_immutable;
1;
"#;
    open_document(&server, "file:///animal.pm", content);

    let result = send_request(
        &server,
        60,
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": "file:///animal.pm"}}),
    )?;

    let symbols = result.as_array().ok_or("document symbols should return an array")?;

    assert!(!symbols.is_empty(), "Moose class should produce document symbols");

    // Collect all symbol names
    let names: Vec<&str> =
        symbols.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    // Should find the package
    assert!(
        names.iter().any(|n| n.contains("Animal")),
        "should find Animal package symbol, found: {:?}",
        names
    );

    let package = symbols
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Animal"))
        .ok_or("Animal package symbol not found")?;
    let children =
        package["children"].as_array().ok_or("Animal package symbol should have children")?;
    let child_names: Vec<&str> =
        children.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();
    assert!(
        child_names.first() == Some(&"name") && child_names.get(1) == Some(&"sound"),
        "attribute symbols should lead the Animal outline children, found: {:?}",
        child_names
    );
    assert!(
        !child_names.iter().any(|name| *name == "Animal"),
        "package self symbol should not appear as a child, found: {:?}",
        child_names
    );
    assert!(
        children.iter().any(|s| s.get("name").and_then(|n| n.as_str()) == Some("name")
            && s.get("kind").and_then(|k| k.as_i64()) == Some(7)),
        "name attribute should appear as a Property symbol"
    );
    assert!(
        children.iter().any(|s| s.get("name").and_then(|n| n.as_str()) == Some("sound")
            && s.get("kind").and_then(|k| k.as_i64()) == Some(7)),
        "sound attribute should appear as a Property symbol"
    );

    // Should find subroutines
    assert!(
        names.iter().any(|n| n.contains("speak")),
        "should find speak subroutine symbol, found: {:?}",
        names
    );

    Ok(())
}

#[test]
fn dbi_usage_completion() -> TestResult {
    let server = setup_server();
    let content = r#"use DBI;

my $dbh = DBI->connect("dbi:SQLite:dbname=test.db", "", "");
my $sth = $dbh->prepare("SELECT * FROM users WHERE id = ?");
$sth->execute(42);

while (my $row = $sth->fetchrow_hashref) {
    print $row->{name};
}

$sth->finish;
$dbh->disconnect;

sub get_user {
    my ($self, $id) = @_;
    my $sth = $self->{dbh}->prepare("SELECT * FROM users WHERE id = ?");
    $sth->execute($id);
    return $sth->fetchrow_hashref;
}

get_
"#;
    open_document(&server, "file:///dbi.pl", content);

    // Request completion for the partial function name "get_"
    let result = send_request(
        &server,
        60,
        "textDocument/completion",
        json!({
            "textDocument": {"uri": "file:///dbi.pl"},
            "position": {"line": 20, "character": 4}
        }),
    )?;

    // Check that completion includes the user-defined function
    let items = result["items"]
        .as_array()
        .or_else(|| result.as_array())
        .ok_or("completion should return items")?;

    let labels: Vec<&str> =
        items.iter().filter_map(|i| i.get("label").and_then(|l| l.as_str())).collect();

    assert!(
        labels.contains(&"get_user"),
        "completion should include get_user, found: {:?}",
        labels
    );

    Ok(())
}

#[test]
fn dbi_usage_document_symbols() -> TestResult {
    let server = setup_server();
    let content = r#"package MyApp::DB;

use DBI;

sub new {
    my ($class, %args) = @_;
    my $self = bless { dbh => undef }, $class;
    return $self;
}

sub connect {
    my ($self, $dsn) = @_;
    $self->{dbh} = DBI->connect($dsn, "", "");
    return $self;
}

sub query {
    my ($self, $sql, @params) = @_;
    my $sth = $self->{dbh}->prepare($sql);
    $sth->execute(@params);
    return $sth->fetchall_arrayref({});
}

1;
"#;
    open_document(&server, "file:///db.pm", content);

    let result = send_request(
        &server,
        60,
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": "file:///db.pm"}}),
    )?;

    let symbols = result.as_array().ok_or("document symbols should return an array")?;

    let names: Vec<&str> =
        symbols.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    // Should find the package and its subroutines
    assert!(
        names.iter().any(|n| n.contains("MyApp::DB") || n.contains("MyApp")),
        "should find MyApp::DB package, found: {:?}",
        names
    );
    assert!(names.contains(&"new"), "should find new subroutine, found: {:?}", names);
    assert!(names.contains(&"connect"), "should find connect subroutine, found: {:?}", names);
    assert!(names.contains(&"query"), "should find query subroutine, found: {:?}", names);

    Ok(())
}

#[test]
fn moose_class_folding_ranges() -> TestResult {
    let server = setup_server();
    let content = r#"package Animal;
use Moose;

has 'name' => (
    is       => 'ro',
    isa      => 'Str',
    required => 1,
);

has 'sound' => (
    is      => 'ro',
    isa     => 'Str',
    default => '...',
);

sub speak {
    my ($self) = @_;
    return $self->name . " says " . $self->sound;
}

no Moose;
__PACKAGE__->meta->make_immutable;
1;
"#;
    open_document(&server, "file:///moose_fold.pm", content);

    let result = send_request(
        &server,
        50,
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": "file:///moose_fold.pm"}}),
    )?;

    let ranges = result.as_array().ok_or("folding ranges should return an array")?;

    // Should fold the multi-line has() calls and the subroutine
    assert!(
        ranges.len() >= 2,
        "Moose class with multi-line attributes and sub should have >= 2 folding ranges, got {}",
        ranges.len()
    );

    Ok(())
}

// ===========================================================================
// Diagnostics for real-world patterns
// ===========================================================================

#[test]
fn diagnostics_strict_undeclared_variable() -> TestResult {
    let server = setup_server();
    let content = r#"use strict;
use warnings;

my $declared = 42;
print $undeclared;
"#;
    open_document(&server, "file:///strict.pl", content);

    // Diagnostics are typically published as notifications, but we can also
    // test via the pull diagnostics endpoint if available, or verify the
    // document opens without crashing the server and that the server remains
    // responsive afterwards.

    // Verify server is still responsive after opening a file with issues
    let result = send_request(
        &server,
        70,
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": "file:///strict.pl"}}),
    )?;

    // Should return symbols even for files with diagnostics
    assert!(
        result.is_array(),
        "server should remain responsive after opening file with undeclared variable"
    );

    Ok(())
}

#[test]
fn diagnostics_syntax_error_does_not_crash_server() -> TestResult {
    let server = setup_server();
    let content = r#"sub broken {
    my $x = ;
    if ( {
    }
}
"#;
    open_document(&server, "file:///broken.pl", content);

    // Server must remain responsive after opening a file with syntax errors
    let result = send_request(
        &server,
        70,
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": "file:///broken.pl"}}),
    )?;

    assert!(
        result.is_array(),
        "server should remain responsive after opening file with syntax errors"
    );

    // Also verify hover still works
    let hover_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((71) as i64)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///broken.pl"},
            "position": {"line": 0, "character": 5}
        })),
    };
    let hover_resp = server.handle_request(hover_req);
    // Should return Some response (either hover data or null result), not crash
    assert!(hover_resp.is_some(), "hover should return a response for file with syntax errors");

    Ok(())
}

// ===========================================================================
// Hover - additional patterns
// ===========================================================================

#[test]
fn hover_on_package_name() -> TestResult {
    let server = setup_server();
    let content = r#"package MyApp::Controller;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

1;
"#;
    open_document(&server, "file:///pkg_hover.pm", content);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((80) as i64)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///pkg_hover.pm"},
            "position": {"line": 0, "character": 10}  // on "MyApp::Controller"
        })),
    };
    let response = server.handle_request(req);
    // Should return a response (hover info or null), not crash
    assert!(response.is_some(), "hover on package name should return a response");

    Ok(())
}

#[test]
fn hover_on_use_statement_module() -> TestResult {
    let server = setup_server();
    let content = "use List::Util qw(sum max min);\nmy $total = sum(1, 2, 3);\n";
    open_document(&server, "file:///use_hover.pl", content);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((81) as i64)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///use_hover.pl"},
            "position": {"line": 0, "character": 6}  // on "List::Util"
        })),
    };
    let response = server.handle_request(req);
    assert!(response.is_some(), "hover on module name in use statement should return a response");

    Ok(())
}

// ===========================================================================
// Code Actions - additional patterns
// ===========================================================================

#[test]
fn code_actions_empty_range() -> TestResult {
    let server = setup_server();
    let content = "my $x = 42;\nprint $x;\n";
    open_document(&server, "file:///actions.pl", content);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((90) as i64)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": {"uri": "file:///actions.pl"},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "context": {
                "diagnostics": []
            }
        })),
    };
    let response =
        server.handle_request(req).ok_or("code action request should return a response")?;

    // Response should have a result (possibly empty array)
    let result = response.result.ok_or("code action should have result")?;
    assert!(result.is_array(), "code actions should return an array, got: {:?}", result);

    Ok(())
}

// ===========================================================================
// Document Symbols - edge cases
// ===========================================================================

#[test]
fn document_symbols_empty_file() -> TestResult {
    let server = setup_server();
    open_document(&server, "file:///empty_sym.pl", "");

    let result = send_request(
        &server,
        100,
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": "file:///empty_sym.pl"}}),
    )?;

    let symbols = result.as_array().ok_or("document symbols should return an array")?;
    assert!(symbols.is_empty(), "empty document should produce no document symbols");

    Ok(())
}

#[test]
fn document_symbols_multiple_packages() -> TestResult {
    let server = setup_server();
    let content = r#"package Foo;

sub foo_method { return 1 }

package Bar;

sub bar_method { return 2 }

package Baz;

sub baz_method { return 3 }

1;
"#;
    open_document(&server, "file:///multi_pkg.pm", content);

    let result = send_request(
        &server,
        100,
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": "file:///multi_pkg.pm"}}),
    )?;

    let symbols = result.as_array().ok_or("document symbols should return an array")?;

    let names: Vec<&str> =
        symbols.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    // Should find all three packages and their methods
    assert!(names.iter().any(|n| n.contains("Foo")), "should find Foo package, found: {:?}", names);
    assert!(names.iter().any(|n| n.contains("Bar")), "should find Bar package, found: {:?}", names);
    assert!(names.iter().any(|n| n.contains("Baz")), "should find Baz package, found: {:?}", names);

    Ok(())
}
