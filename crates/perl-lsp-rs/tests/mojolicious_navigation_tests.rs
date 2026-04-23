//! Focused go-to-definition tests for Mojolicious route targets.

mod support;

use serde_json::json;
use support::lsp_harness::{LspHarness, TempWorkspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_valid_location(location: &serde_json::Value) {
    assert!(location.get("uri").is_some(), "Location must have 'uri' field, got: {:?}", location);
    let range = location.get("range");
    assert!(range.is_some(), "Location must have 'range' field, got: {:?}", location);
    let range = range.unwrap_or(&json!(null));
    assert!(range.get("start").is_some(), "Range must have 'start' position");
    assert!(range.get("end").is_some(), "Range must have 'end' position");
}

fn first_location(result: &serde_json::Value) -> Option<&serde_json::Value> {
    if let Some(locations) = result.as_array() {
        locations.first()
    } else if result.is_object() {
        Some(result)
    } else {
        None
    }
}

fn position_of(text: &str, needle: &str) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(char_idx) = line.find(needle) {
            return Ok((line_idx, char_idx));
        }
    }

    Err(format!("needle `{needle}` not found in test text").into())
}

#[test]
fn mojolicious_string_route_target_definitions_to_controller_method() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/User.pm",
        r#"package MyApp::Controller::User;
use Mojo::Base 'Mojolicious::Controller';

sub list {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    workspace.write(
        "lib/MyApp/App.pm",
        r#"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/')->to('user#list');
}

1;
"#,
    )?;

    let app_text = r##"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/')->to('user#list');
}

1;
"##;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/User.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/User.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "user#list")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/User.pm"),
        "definition should point to controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_kv_route_target_definitions_to_controller_method() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Admin.pm",
        r#"package MyApp::Controller::Admin;
use Mojo::Base 'Mojolicious::Controller';

sub dashboard {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    workspace.write(
        "lib/MyApp/App.pm",
        r#"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/admin')->to(controller => 'admin', action => 'dashboard');
}

1;
"#,
    )?;

    let app_text = r#"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/admin')->to(controller => 'admin', action => 'dashboard');
}

1;
"#;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Admin.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/Admin.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "dashboard")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Admin.pm"),
        "definition should point to controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_string_route_with_double_quotes_definitions_to_controller_method() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Health.pm",
        r#"package MyApp::Controller::Health;
use Mojo::Base 'Mojolicious::Controller';

sub check {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r##"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/health')->to("health#check");
}

1;
"##;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Health.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/Health.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "health#check")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Health.pm"),
        "definition should point to controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_string_route_with_dashed_controller_definitions_to_nested_controller_method()
-> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Admin/User.pm",
        r#"package MyApp::Controller::Admin::User;
use Mojo::Base 'Mojolicious::Controller';

sub list {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r##"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/admin/users')->to('admin-user#list');
}

1;
"##;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Admin/User.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/Admin/User.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "admin-user#list")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Admin/User.pm"),
        "definition should point to nested controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_kv_route_target_action_first_definitions_to_controller_method() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Audit.pm",
        r#"package MyApp::Controller::Audit;
use Mojo::Base 'Mojolicious::Controller';

sub list {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r#"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/audit')->to(action => "list", controller => "audit");
}

1;
"#;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Audit.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/Audit.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "list")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Audit.pm"),
        "definition should point to controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_kv_route_target_double_quotes_definitions_to_controller_method() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Profile.pm",
        r#"package MyApp::Controller::Profile;
use Mojo::Base 'Mojolicious::Controller';

sub show {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r#"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/profile')->to(controller => "profile", action => "show");
}

1;
"#;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Profile.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/Profile.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "show")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Profile.pm"),
        "definition should point to controller file, got: {uri}"
    );

    Ok(())
}
#[test]
fn mojolicious_string_route_snake_case_controller_resolves_to_camelized_package() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/AdminUser.pm",
        r#"package MyApp::Controller::AdminUser;
use Mojo::Base 'Mojolicious::Controller';

sub list {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r##"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/admin-users')->to('admin_user#list');
}

1;
"##;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/AdminUser.pm"),
        &std::fs::read_to_string(workspace.dir.path().join("lib/MyApp/Controller/AdminUser.pm"))?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "admin_user#list")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/AdminUser.pm"),
        "definition should point to camelized controller file, got: {uri}"
    );

    Ok(())
}

#[test]
fn mojolicious_string_route_space_separated_controller_resolves_to_nested_package() -> TestResult {
    let workspace = TempWorkspace::new()?;
    workspace.write(
        "lib/MyApp/Controller/Google/Antigravity.pm",
        r#"package MyApp::Controller::Google::Antigravity;
use Mojo::Base 'Mojolicious::Controller';

sub launch {
    my $self = shift;
    return "ok";
}

1;
"#,
    )?;
    let app_text = r##"package MyApp::App;
use Mojo::Base 'Mojolicious';

sub startup {
    my $self = shift;
    my $r = $self->routes;
    $r->get('/google-antigravity')->to('google antigravity#launch');
}

1;
"##;
    workspace.write("lib/MyApp/App.pm", app_text)?;

    let mut harness = LspHarness::new();
    harness.initialize_with_root(&workspace.root_uri, None)?;
    harness.open(
        &workspace.uri("lib/MyApp/Controller/Google/Antigravity.pm"),
        &std::fs::read_to_string(
            workspace.dir.path().join("lib/MyApp/Controller/Google/Antigravity.pm"),
        )?,
    )?;
    harness.open(&workspace.uri("lib/MyApp/App.pm"), app_text)?;
    harness.barrier();

    let (line, character) = position_of(app_text, "google antigravity#launch")?;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": workspace.uri("lib/MyApp/App.pm")},
            "position": {"line": line, "character": character}
        }),
    )?;

    let location = first_location(&result).ok_or("expected a definition location")?;
    assert_valid_location(location);
    let uri = location["uri"].as_str().ok_or("expected definition URI")?;
    assert!(
        uri.contains("MyApp/Controller/Google/Antigravity.pm"),
        "definition should point to nested controller file, got: {uri}"
    );

    Ok(())
}
