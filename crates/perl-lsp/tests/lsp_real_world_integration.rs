/// Real-world integration tests for LSP server
/// Tests against actual Perl codebases and common patterns
use serde_json::json;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Test with a real CPAN module structure
#[test]
fn test_cpan_module_structure() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_cpan_module_structure (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Typical CPAN module structure
    let module_code = r#"
package My::Module;
use strict;
use warnings;
use Exporter 'import';

our $VERSION = '1.00';
our @EXPORT_OK = qw(function1 function2);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

use Carp;
use Data::Dumper;

sub new {
    my ($class, %args) = @_;
    my $self = {
        name => $args{name} || 'default',
        debug => $args{debug} || 0,
    };
    return bless $self, $class;
}

sub function1 {
    my ($self, $param) = @_;
    croak "Missing parameter" unless defined $param;
    return $self->{name} . ": " . $param;
}

sub function2 {
    my $self = shift;
    return Data::Dumper::Dumper($self);
}

sub DESTROY {
    my $self = shift;
    print "Destroying $self->{name}\n" if $self->{debug};
}

1;

__END__

=head1 NAME

My::Module - A sample module

=head1 SYNOPSIS

    use My::Module;
    my $obj = My::Module->new(name => 'test');
    print $obj->function1('hello');

=head1 DESCRIPTION

This is a sample CPAN-style module for testing.

=cut
"#;

    let uri = "file:///Module.pm";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": module_code
                }
            }
        }),
    );

    // Request document symbols
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    assert!(response["result"].is_array());

    let symbols = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Verify expected symbols are found
    let symbol_names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(symbol_names.contains(&"My::Module"));
    assert!(symbol_names.contains(&"new"));
    assert!(symbol_names.contains(&"function1"));
    assert!(symbol_names.contains(&"function2"));
    assert!(symbol_names.contains(&"DESTROY"));

    Ok(())
}

/// Test with Mojolicious web application
#[test]
fn test_mojolicious_app() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_mojolicious_app (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let mojo_app = r#"
#!/usr/bin/env perl
use Mojolicious::Lite;
use Mojo::JSON qw(decode_json encode_json);

# Route handlers
get '/' => sub {
    my $c = shift;
    $c->render(template => 'index');
};

get '/api/users' => sub {
    my $c = shift;
    my @users = (
        { id => 1, name => 'Alice' },
        { id => 2, name => 'Bob' },
    );
    $c->render(json => \@users);
};

post '/api/users' => sub {
    my $c = shift;
    my $user = decode_json($c->req->body);
    
    # Validate input
    unless ($user->{name}) {
        return $c->render(
            json => { error => 'Name required' },
            status => 400
        );
    }
    
    # Save user (mock)
    $user->{id} = int(rand(1000));
    
    $c->render(json => $user, status => 201);
};

# WebSocket endpoint
websocket '/ws' => sub {
    my $c = shift;
    
    $c->on(message => sub {
        my ($c, $msg) = @_;
        $c->send("Echo: $msg");
    });
};

app->start;

__DATA__

@@ index.html.ep
<!DOCTYPE html>
<html>
<head><title>Mojolicious App</title></head>
<body>
    <h1>Welcome to Mojolicious!</h1>
</body>
</html>
"#;

    let uri = "file:///app.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": mojo_app
                }
            }
        }),
    );

    // Test diagnostics - should have no errors
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let items = response["result"]["items"].as_array().ok_or("Expected items to be an array")?;

    // Should parse without errors
    assert_eq!(items.len(), 0, "Mojolicious app should parse without errors");

    Ok(())
}

/// Test with DBI database code
#[test]
fn test_dbi_database_code() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_dbi_database_code (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let dbi_code = r#"
#!/usr/bin/perl
use strict;
use warnings;
use DBI;
use Try::Tiny;

# Database connection
my $dsn = "DBI:mysql:database=testdb;host=localhost";
my $username = "user";
my $password = "pass";

my $dbh;

try {
    $dbh = DBI->connect($dsn, $username, $password, {
        RaiseError => 1,
        AutoCommit => 1,
        PrintError => 0,
    });
} catch {
    die "Cannot connect to database: $_";
};

# Prepared statements
my $insert_sth = $dbh->prepare(q{
    INSERT INTO users (name, email, created_at)
    VALUES (?, ?, NOW())
});

my $select_sth = $dbh->prepare(q{
    SELECT id, name, email
    FROM users
    WHERE created_at > ?
    ORDER BY name
});

# Transaction example
sub add_user_with_profile {
    my ($name, $email, $bio) = @_;
    
    $dbh->begin_work;
    
    try {
        # Insert user
        $insert_sth->execute($name, $email);
        my $user_id = $dbh->last_insert_id(undef, undef, 'users', 'id');
        
        # Insert profile
        my $profile_sth = $dbh->prepare(q{
            INSERT INTO profiles (user_id, bio)
            VALUES (?, ?)
        });
        $profile_sth->execute($user_id, $bio);
        
        $dbh->commit;
        return $user_id;
    } catch {
        $dbh->rollback;
        die "Transaction failed: $_";
    };
}

# Fetch and process results
sub get_recent_users {
    my $days = shift || 7;
    my $date = DateTime->now->subtract(days => $days);
    
    $select_sth->execute($date);
    
    my @users;
    while (my $row = $select_sth->fetchrow_hashref) {
        push @users, {
            id => $row->{id},
            name => $row->{name},
            email => $row->{email},
        };
    }
    
    return \@users;
}

# Cleanup
END {
    $dbh->disconnect if $dbh;
}

1;
"#;

    let uri = "file:///database.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": dbi_code
                }
            }
        }),
    );

    // Request symbols
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let symbols = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Verify subroutines are detected
    let sub_names: Vec<&str> = symbols
        .iter()
        .filter(|s| s["kind"] == 12) // Function
        .filter_map(|s| s["name"].as_str())
        .collect();

    assert!(sub_names.contains(&"add_user_with_profile"));
    assert!(sub_names.contains(&"get_recent_users"));

    Ok(())
}

/// Test with Test::More test file
#[test]
fn test_perl_test_file() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_perl_test_file (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let test_code = r#"
#!/usr/bin/perl
use strict;
use warnings;
use Test::More tests => 10;
use Test::Exception;
use Test::Deep;
use lib 'lib';

BEGIN {
    use_ok('My::Module');
}

# Test object creation
my $obj = My::Module->new(name => 'test');
isa_ok($obj, 'My::Module', 'Object creation');

# Test methods
is($obj->get_name, 'test', 'get_name returns correct value');
ok($obj->set_name('new'), 'set_name returns true');
is($obj->get_name, 'new', 'name was updated');

# Test exceptions
throws_ok {
    $obj->divide(10, 0);
} qr/Division by zero/, 'divide by zero throws exception';

lives_ok {
    $obj->divide(10, 2);
} 'normal division works';

# Test deep structures
my $result = $obj->get_data;
cmp_deeply($result, {
    name => 'new',
    items => bag(1, 2, 3, 4, 5),
    metadata => superhashof({
        version => re(qr/^\d+\.\d+$/),
    }),
}, 'complex data structure matches');

# Subtest
subtest 'Edge cases' => sub {
    plan tests => 3;
    
    ok(!$obj->process(undef), 'undef handled');
    ok(!$obj->process(''), 'empty string handled');
    ok($obj->process('valid'), 'valid input processed');
};

# Skip and TODO
SKIP: {
    skip "Database not configured", 2 unless $ENV{DB_TEST};
    
    ok($obj->connect_db, 'Database connection');
    ok($obj->query('SELECT 1'), 'Simple query');
}

TODO: {
    local $TODO = "Feature not implemented yet";

    ok($obj->new_feature, 'New feature works');
}

done_testing();
"#;

    let uri = "file:///test.t";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": test_code
                }
            }
        }),
    );

    // Verify parsing succeeds
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let items = response["result"]["items"].as_array().ok_or("Expected items to be an array")?;
    assert_eq!(items.len(), 0, "Test file should parse without errors");

    Ok(())
}

/// Test with Catalyst MVC controller
#[test]
fn test_catalyst_controller() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_catalyst_controller (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let catalyst_code = r#"
package MyApp::Controller::API;
use Moose;
use namespace::autoclean;

BEGIN { extends 'Catalyst::Controller::REST' }

__PACKAGE__->config(
    default => 'application/json',
    map => {
        'application/json' => 'JSON',
        'text/html' => 'YAML::HTML',
    },
);

sub users : Local : ActionClass('REST') {}

sub users_GET {
    my ($self, $c) = @_;
    
    my @users = $c->model('DB::User')->search(
        { active => 1 },
        { order_by => 'name' }
    )->all;
    
    $self->status_ok(
        $c,
        entity => [ map { $_->TO_JSON } @users ]
    );
}

sub users_POST {
    my ($self, $c) = @_;
    
    my $params = $c->req->data;
    
    # Validate
    unless ($params->{email} && $params->{name}) {
        return $self->status_bad_request(
            $c,
            message => "Missing required fields"
        );
    }
    
    # Create user
    my $user = eval {
        $c->model('DB::User')->create($params);
    };
    
    if ($@) {
        return $self->status_bad_request(
            $c,
            message => "Failed to create user: $@"
        );
    }
    
    $self->status_created(
        $c,
        location => $c->uri_for('/api/users', $user->id),
        entity => $user->TO_JSON
    );
}

sub user : Path('users') : Args(1) : ActionClass('REST') {}

sub user_GET {
    my ($self, $c, $id) = @_;
    
    my $user = $c->model('DB::User')->find($id);
    
    unless ($user) {
        return $self->status_not_found(
            $c,
            message => "User not found"
        );
    }
    
    $self->status_ok(
        $c,
        entity => $user->TO_JSON
    );
}

sub user_PUT {
    my ($self, $c, $id) = @_;
    
    my $user = $c->model('DB::User')->find($id);
    
    unless ($user) {
        return $self->status_not_found(
            $c,
            message => "User not found"
        );
    }
    
    $user->update($c->req->data);
    
    $self->status_ok(
        $c,
        entity => $user->TO_JSON
    );
}

sub user_DELETE {
    my ($self, $c, $id) = @_;
    
    my $user = $c->model('DB::User')->find($id);
    
    unless ($user) {
        return $self->status_not_found(
            $c,
            message => "User not found"
        );
    }
    
    $user->delete;
    
    $self->status_no_content($c);
}

__PACKAGE__->meta->make_immutable;

1;
"#;

    let uri = "file:///Controller.pm";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": catalyst_code
                }
            }
        }),
    );

    // Request symbols - should find all REST methods
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let symbols = response["result"].as_array().ok_or("Expected result to be an array")?;

    let method_names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // Verify REST methods are found
    assert!(method_names.contains(&"users_GET"));
    assert!(method_names.contains(&"users_POST"));
    assert!(method_names.contains(&"user_GET"));
    assert!(method_names.contains(&"user_PUT"));
    assert!(method_names.contains(&"user_DELETE"));

    Ok(())
}

/// Test with complex regex patterns
#[test]
fn test_complex_regex_patterns() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_complex_regex_patterns (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let regex_code = r#"
#!/usr/bin/perl
use strict;
use warnings;

# Email validation
my $email_regex = qr{
    \A                      # Start of string
    [a-zA-Z0-9._%+-]+       # Local part
    @                       # At sign
    [a-zA-Z0-9.-]+          # Domain name
    \.                      # Dot
    [a-zA-Z]{2,}            # TLD
    \z                      # End of string
}x;

# URL parsing
my $url_regex = qr{
    \A
    (?<protocol>https?|ftp)://   # Protocol
    (?<host>[^:/]+)               # Host
    (?::(?<port>\d+))?            # Optional port
    (?<path>/[^?#]*)?             # Optional path
    (?:\?(?<query>[^#]*))?        # Optional query
    (?:\#(?<fragment>.*))?        # Optional fragment
    \z
}x;

# Log parsing
sub parse_log_line {
    my $line = shift;
    
    if ($line =~ m{
        ^
        (?<timestamp>\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})  # Timestamp
        \s+
        \[(?<level>\w+)\]                                      # Log level
        \s+
        (?<message>.+)                                         # Message
        $
    }x) {
        return {
            timestamp => $+{timestamp},
            level => $+{level},
            message => $+{message},
        };
    }
    
    return undef;
}

# HTML tag extraction
sub extract_tags {
    my $html = shift;
    my @tags;
    
    while ($html =~ m{
        <
        (?<tag>[a-zA-Z][a-zA-Z0-9]*)   # Tag name
        (?:\s+[^>]*)?                   # Attributes
        (?:/>|>.*?</\g{tag}>)           # Self-closing or with content
    }gx) {
        push @tags, $+{tag};
    }
    
    return @tags;
}

# Substitution patterns
sub clean_text {
    my $text = shift;
    
    # Remove extra whitespace
    $text =~ s/\s+/ /g;
    
    # Remove HTML tags
    $text =~ s/<[^>]+>//g;
    
    # Convert entities
    $text =~ s/&amp;/&/g;
    $text =~ s/&lt;/</g;
    $text =~ s/&gt;/>/g;
    
    # Trim
    $text =~ s/^\s+|\s+$//g;
    
    return $text;
}

# Transliteration
sub normalize_text {
    my $text = shift;
    
    # Normalize quotes
    $text =~ tr/""''/""''/;
    
    # Remove accents (simplified)
    $text =~ tr/àáäâèéëêìíïîòóöôùúüû/aaaaeeeeiiiioooouuuu/;
    
    return $text;
}

1;
"#;

    let uri = "file:///regex.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": regex_code
                }
            }
        }),
    );

    // Should parse complex regex patterns without errors
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let items = response["result"]["items"].as_array().ok_or("Expected items to be an array")?;
    assert_eq!(items.len(), 0, "Complex regex patterns should parse correctly");

    Ok(())
}

/// Test with modern Perl features
#[test]
fn test_modern_perl_features() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_modern_perl_features (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let modern_perl = r#"
#!/usr/bin/perl
use v5.36;
use feature 'class';
use experimental 'try';
use experimental 'defer';

# Class with field variables
class Point {
    field $x :param = 0;
    field $y :param = 0;
    
    method move($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
    
    method distance($other) {
        my $dx = $other->x - $x;
        my $dy = $other->y - $y;
        return sqrt($dx * $dx + $dy * $dy);
    }
    
    method x() { $x }
    method y() { $y }
}

# Try/catch with finally
sub risky_operation {
    my $resource;
    
    try {
        $resource = acquire_resource();
        process($resource);
    }
    catch ($e) {
        log_error("Operation failed: $e");
        die $e;
    }
    finally {
        release_resource($resource) if $resource;
    }
}

# Defer for cleanup
sub with_defer {
    open my $fh, '<', 'file.txt' or die $!;
    defer { close $fh }
    
    my $handle = get_handle();
    defer { $handle->cleanup() }
    
    # Do work...
    process_file($fh);
    process_handle($handle);
}

# Signatures with types
sub typed_function (Str $name, Int $age, ArrayRef $data) {
    say "Name: $name, Age: $age";
    for my $item (@$data) {
        say "Item: $item";
    }
}

# Match operator
sub process_value($value) {
    given ($value) {
        when (undef) { return 'undefined' }
        when (/^\d+$/) { return 'number' }
        when ([1..10]) { return 'small number' }
        when (\&is_valid) { return 'valid' }
        default { return 'other' }
    }
}

# Postfix dereferencing
sub array_operations {
    my $aref = [1, 2, 3, 4, 5];
    my @array = $aref->@*;
    my $first = $aref->@[0];
    my @slice = $aref->@[0..2];
    
    my $href = { a => 1, b => 2 };
    my %hash = $href->%*;
    my @keys = $href->%{qw(a b)};
}

1;
"#;

    let uri = "file:///modern.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": modern_perl
                }
            }
        }),
    );

    // Request symbols to verify modern constructs are recognized
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let symbols = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Look for class and methods
    let symbol_names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(symbol_names.contains(&"Point"));
    assert!(symbol_names.contains(&"risky_operation"));
    assert!(symbol_names.contains(&"with_defer"));

    Ok(())
}

/// Test multi-file project with modules
#[test]
fn test_multi_file_project() -> Result<(), Box<dyn std::error::Error>> {
    // Opt-in for slow/flake-prone integration test
    if std::env::var("RUN_REAL_WORLD").is_err() {
        eprintln!("skipping test_multi_file_project (set RUN_REAL_WORLD=1 to run)");
        return Ok(());
    }

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Main script
    let main_script = r#"
#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use MyApp::Config;
use MyApp::Database;
use MyApp::Logger qw(log_info log_error);

my $config = MyApp::Config->new('config.yaml');
my $db = MyApp::Database->connect($config->database);
my $logger = MyApp::Logger->instance;

log_info("Application started");

eval {
    my $users = $db->get_users;
    for my $user (@$users) {
        process_user($user);
    }
};

if ($@) {
    log_error("Error processing users: $@");
    exit 1;
}

sub process_user {
    my $user = shift;
    log_info("Processing user: " . $user->{name});
    # Process...
}

log_info("Application completed");
"#;

    // Config module
    let config_module = r#"
package MyApp::Config;
use strict;
use warnings;
use YAML::XS qw(LoadFile);

sub new {
    my ($class, $file) = @_;
    my $self = {
        data => LoadFile($file),
    };
    return bless $self, $class;
}

sub database {
    my $self = shift;
    return $self->{data}->{database};
}

sub get {
    my ($self, $key) = @_;
    return $self->{data}->{$key};
}

1;
"#;

    // Open both files
    let main_uri = "file:///main.pl";
    let config_uri = "file:///lib/MyApp/Config.pm";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": main_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": main_script
                }
            }
        }),
    );

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": config_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": config_module
                }
            }
        }),
    );

    // Test cross-file references (would need actual implementation)
    // For now, just verify both files parse correctly

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": main_uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let items = response["result"]["items"].as_array().ok_or("Expected items to be an array")?;
    assert_eq!(items.len(), 0, "Main script should parse without errors");

    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": config_uri }
            }
        }),
    );

    let response = read_response(&mut server);
    let items = response["result"]["items"].as_array().ok_or("Expected items to be an array")?;
    assert_eq!(items.len(), 0, "Config module should parse without errors");

    Ok(())
}
