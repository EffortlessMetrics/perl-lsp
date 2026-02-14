//! Comprehensive tests for data structure combinations
//!
//! These tests validate complex interactions between data structures
//! including array/hash operations, references, dereferencing, variable lists,
//! tie/untie operations, and package/module interactions.

use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};

/// Test complex array/hash operations with references and dereferencing
#[test]
fn test_complex_array_hash_operations() {
    let code = r#"
# Complex nested data structures
my $data = {
    users => [
        {
            id => 1,
            name => 'Alice',
            profile => {
                email => 'alice@example.com',
                settings => {
                    theme => 'dark',
                    notifications => 1,
                    privacy => {
                        show_email => 0,
                        show_name => 1
                    }
                }
            }
        },
        {
            id => 2,
            name => 'Bob',
            profile => {
                email => 'bob@example.com',
                settings => {
                    theme => 'light',
                    notifications => 0,
                    privacy => {
                        show_email => 1,
                        show_name => 0
                    }
                }
            }
        }
    ],
    metadata => {
        total => 2,
        last_updated => time(),
        version => '1.0'
    }
};

# Complex dereferencing operations
my $first_user = $data->{users}[0];
my $alice_email = $data->{users}[0]{profile}{email};
my $bob_theme = $data->{users}[1]{profile}{settings}{theme};
my $alice_show_email = $data->{users}[0]{profile}{settings}{privacy}{show_email};

# Array operations on complex structures
my @user_names = map { $_->{name} } @{$data->{users}};
my @user_emails = map { $_->{profile}{email} } @{$data->{users}};
my @themes = map { $_->{profile}{settings}{theme} } @{$data->{users}};

# Hash operations on complex structures
my %user_by_name = map { $_->{name} => $_ } @{$data->{users}};
my %email_by_name = map { $_->{name} => $_->{profile}{email} } @{$data->{users}};
my %settings_by_name = map { $_->{name} => $_->{profile}{settings} } @{$data->{users}};

# Complex reference operations
my $users_ref = $data->{users};
my $profiles_ref = [map { $_->{profile} } @$users_ref];
my $settings_ref = {map { $_->{name} => $_->{profile}{settings} } @$users_ref};

# Dereferencing with complex expressions
my $complex_deref = $data->{users}[ $data->{metadata}{total} - 1 ]{profile}{settings}{privacy};
my $nested_deref = ${${${$data->{users}[0]}{profile}}{settings}}{privacy};

# Array slice operations
my @first_users = @{$data->{users}}[0..1];
my @user_profiles = @{$data->{users}}[0,1]{profile};
my @privacy_settings = @{$data->{users}}[0,1]{profile}{settings}{privacy};

# Hash slice operations
my @metadata_values = @{$data->{metadata}}{qw(total last_updated version)};
my @user_ids = @{$data->{users}}[0,1]{id};

# Complex operations with references
sub process_user_data {
    my ($users_ref) = @_;
    
    my @processed;
    for my $user (@$users_ref) {
        my $processed_user = {
            %$user, # Copy original
            processed => 1,
            timestamp => time()
        };
        
        # Modify nested structure
        $processed_user->{profile}{settings}{privacy}{processed} = 1;
        
        push @processed, $processed_user;
    }
    
    return \@processed;
}

my $processed_users = process_user_data($data->{users});

# Complex array/hash chaining
my $chained_access = $data->{users}[0]{profile}{settings}{privacy}{show_email} ? 
    $data->{users}[0]{profile}{email} : 
    'hidden';

# Complex reference manipulation
sub deep_clone {
    my ($ref) = @_;
    
    if (ref $ref eq 'ARRAY') {
        return [map { deep_clone($_) } @$ref];
    } elsif (ref $ref eq 'HASH') {
        return {map { $_ => deep_clone($ref->{$_}) } keys %$ref};
    } else {
        return $ref;
    }
}

my $cloned_data = deep_clone($data);
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify hash literals
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));
    assert!(!hash_literals.is_empty(), "Should have hash literals");

    // Verify array literals
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    assert!(!array_literals.is_empty(), "Should have array literals");

    // Verify dereferencing operations (binary operations with {} and [])
    let binary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Binary { .. }));
    assert!(!binary_ops.is_empty(), "Should have binary operations for dereferencing");

    // Verify reference operations (unary with \)
    let unary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Unary { .. }));
    assert!(!unary_ops.is_empty(), "Should have unary operations for references");

    // Verify variable declarations
    let var_decls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::VariableDeclaration { .. }));
    assert!(!var_decls.is_empty(), "Should have variable declarations");
}

/// Test VariableListDeclaration with nested structures and function calls
#[test]
fn test_variable_list_declaration_nested_structures() {
    let code = r#"
# Simple variable list declarations
my ($scalar1, $scalar2, $scalar3);
my ($array_ref, $hash_ref, $code_ref);
my ($object, $method, $result);

# Variable list with initializers
my ($x, $y, $z) = (1, 2, 3);
my ($name, $age, $email) = get_user_info();
my ($config, $settings, $options) = load_config('config.json');

# Complex nested structures in variable lists
my ($users_ref, $metadata_ref) = load_complex_data();
my ($user_profiles, $privacy_settings) = extract_profiles($users_ref);

# Variable list with array/hash operations
my ($first_user, $second_user) = @{$users_ref}[0,1];
my ($total_users, $last_updated) = @{$metadata_ref}{qw(total last_updated)};

# Variable list with function calls and method calls
my ($connection, $database, $schema) = connect_database($config);
my ($table, $columns, $indexes) = $database->get_table_info($table_name);
my ($query_result, $affected_rows, $error) = $connection->execute($sql);

# Variable list with complex expressions
my ($min_val, $max_val, $avg_val) = calculate_stats(@numbers);
my ($success, $message, $data) = validate_and_process($input, $options);
my ($status, $headers, $body) = parse_http_response($response);

# Nested variable list declarations
{
    my ($local_var1, $local_var2) = get_local_values();
    my ($nested_ref, $deep_ref) = process_local($local_var1, $local_var2);
    
    # More complex nested declarations
    my ($array_slice, $hash_slice, $mixed_slice) = extract_slices(
        $nested_ref,
        [0, 2, 4],
        [qw(name email status)]
    );
}

# Variable list with dereferencing
my ($user_name, $user_email, $user_theme) = @{$users_ref}[0]{qw(name email theme)};
my ($setting1, $setting2, $setting3) = @{$config->{settings}}{qw(theme notifications privacy)};

# Variable list with typeglob operations
my ($read_fh, $write_fh, $error_fh);
my ($old_stdout, $old_stderr);
my ($temp_stdout, $temp_stderr);

# Variable list with complex expressions
my ($processed_count, $error_count, $warnings) = process_batch(
    \@input_data,
    sub {
        my ($item) = @_;
        return $item->{status} eq 'processed';
    },
    sub {
        my ($item) = @_;
        return $item->{error};
    }
);

# Variable list with regex operations
my ($matched, $captured1, $captured2) = $input =~ /^(pattern)\s+(capture1)\s+(capture2)$/;
my ($cleaned, $validated, $formatted) = process_text($raw_text);

sub get_user_info {
    return ('John Doe', 30, 'john@example.com');
}

sub load_config {
    my ($filename) = @_;
    return ({}, {}, {});
}

sub load_complex_data {
    return ([], {});
}

sub extract_profiles {
    my ($users) = @_;
    return ([], {});
}

sub connect_database {
    my ($config) = @_;
    return (undef, undef, undef);
}

sub calculate_stats {
    my (@numbers) = @_;
    return (0, 0, 0);
}

sub validate_and_process {
    my ($input, $options) = @_;
    return (0, '', {});
}

sub parse_http_response {
    my ($response) = @_;
    return (0, {}, '');
}

sub get_local_values {
    return (1, 2);
}

sub process_local {
    my ($val1, $val2) = @_;
    return ([], {});
}

sub extract_slices {
    my ($ref, $indices, $keys) = @_;
    return ([], {}, []);
}

sub process_batch {
    my ($data, $filter, $error_handler) = @_;
    return (0, 0, 0);
}

sub process_text {
    my ($text) = @_;
    return ('', '', '');
}
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify variable list declarations
    let var_list_decls =
        find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::VariableListDeclaration { .. }));
    assert!(!var_list_decls.is_empty(), "Should have variable list declarations");

    // Verify function calls in initializers
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!function_calls.is_empty(), "Should have function calls");

    // Verify method calls
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    assert!(!method_calls.is_empty(), "Should have method calls");

    // Verify array literals in initializers
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    assert!(!array_literals.is_empty(), "Should have array literals");

    // Verify hash literals in initializers
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));
    assert!(!hash_literals.is_empty(), "Should have hash literals");

    // Verify regex operations
    let match_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Match { .. }));
    assert!(!match_ops.is_empty(), "Should have match operations");
}

/// Test tie/untie operations with complex data structures
#[test]
fn test_tie_untie_complex_structures() {
    let code = r#"
# Tie a simple scalar
my $tied_scalar;
tie $tied_scalar, 'TiedScalar', 'initial_value';

# Tie an array with complex initialization
my @tied_array;
tie @tied_array, 'TiedArray', 
    initial_data => [1, 2, 3, 4, 5],
    readonly => 0,
    max_size => 100;

# Tie a hash with complex configuration
my %tied_hash;
tie %tied_hash, 'TiedHash',
    backend => 'DB_File',
    filename => 'data.db',
    flags => O_CREAT | O_RDWR,
    mode => 0644;

# Tie with complex data structures
my $complex_tied;
tie $complex_tied, 'ComplexTie',
    data => {
        users => [
            {name => 'Alice', id => 1},
            {name => 'Bob', id => 2}
        ],
        metadata => {
            created => time(),
            version => '1.0'
        }
    },
    serializer => sub {
        my ($data) = @_;
        return JSON::encode($data);
    },
    deserializer => sub {
        my ($string) = @_;
        return JSON::decode($string);
    };

# Multiple ties with different types
my ($scalar_tie, $array_tie, $hash_tie);
tie $scalar_tie, 'LoggingScalar', 'important_value';
tie @$array_tie, 'ValidatedArray', validator => sub { $_[0] =~ /^\d+$/ };
tie %$hash_tie, 'PersistentHash', file => 'persistent.dat';

# Tie with method calls and object orientation
my $object_tie;
tie $object_tie, 'ObjectTie',
    object => DataProcessor->new(
        config => load_config(),
        logger => Logger->new(level => 'INFO')
    ),
    methods => {
        FETCH => sub {
            my ($self) = @_;
            return $self->{object}->get_data();
        },
        STORE => sub {
            my ($self, $value) = @_;
            $self->{object}->set_data($value);
        }
    };

# Complex tie operations with error handling
eval {
    my $risky_tie;
    tie $risky_tie, 'RiskyTie', 
        auto_retry => 1,
        timeout => 30,
        on_error => sub {
            my ($error) = @_;
            log_error("Tie error: $error");
        };
    
    # Use the tied variable
    $risky_tie = 'test value';
    my $result = $risky_tie;
};

if ($@) {
    warn "Tie operation failed: $@";
}

# Untie operations with cleanup
untie $tied_scalar;
untie @tied_array;
untie %tied_hash;

# Conditional untie with validation
if (validate_untie($complex_tied)) {
    untie $complex_tie;
}

# Complex untie with error handling
eval {
    untie $risky_tie if tied $risky_tie;
};

if ($@) {
    warn "Untie failed: $@";
}

# Tie with filehandles and I/O
my $tied_fh;
tie *$tied_fh, 'TiedFileHandle',
    filename => 'tied_output.txt',
    mode => '>>',
    buffering => 1;

# Use tied filehandle
print $tied_fh "This goes to tied filehandle\n";

# Complex tie with network operations
my $network_tie;
tie $network_tie, 'NetworkTie',
    host => 'example.com',
    port => 80,
    protocol => 'http',
    timeout => 10,
    retry_count => 3;

# Tie with caching layer
my $cached_tie;
tie $cached_tie, 'CachedTie',
    backend => tie({}, 'PersistentHash', file => 'cache.db'),
    cache_size => 1000,
    ttl => 3600;

sub validate_untie {
    my ($tied_var) = @_;
    return 1; # Simplified validation
}

sub log_error {
    my ($message) = @_;
    print STDERR "$message\n";
}

sub load_config {
    return {};
}
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify tie operations
    let tie_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Tie { .. }));
    assert!(!tie_nodes.is_empty(), "Should have tie operations");

    // Verify untie operations
    let untie_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Untie { .. }));
    assert!(!untie_nodes.is_empty(), "Should have untie operations");

    // Verify typeglob operations (for filehandle ties)
    let typeglob_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Typeglob { .. }));
    assert!(!typeglob_nodes.is_empty(), "Should have typeglob operations");

    // Verify complex data structures in tie arguments
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));

    assert!(!hash_literals.is_empty(), "Should have hash literals in tie arguments");
    assert!(!array_literals.is_empty(), "Should have array literals in tie arguments");

    // Verify eval blocks for error handling
    let eval_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Eval { .. }));
    assert!(!eval_nodes.is_empty(), "Should have eval blocks for error handling");
}

/// Test package/module interactions with symbol tables and exports
#[test]
fn test_package_module_interactions() {
    let code = r#"
# Package declaration with inline block
package BaseModule {
    our $VERSION = '1.0.0';
    our @EXPORT = qw(function1 function2);
    our @EXPORT_OK = qw(optional_function advanced_function);
    
    sub function1 {
        return "Base function 1";
    }
    
    sub function2 {
        return "Base function 2";
    }
    
    sub optional_function {
        return "Optional function";
    }
    
    sub advanced_function {
        return "Advanced function";
    }
}

# Separate package with inheritance
package DerivedModule;
use base 'BaseModule';
our @ISA = ('BaseModule');

our $MODULE_VAR = 'derived_value';
our @MODULE_EXPORT = qw(derived_function);

sub derived_function {
    return "Derived function calling base: " . function1();
}

sub override_function {
    return "Override in derived";
}

# Package with complex symbol table manipulation
package SymbolTable;
use strict 'refs';
use warnings 'all';

our %SYMBOL_TABLE = (
    scalar_symbols => {},
    array_symbols => {},
    hash_symbols => {},
    code_symbols => {},
);

sub import_symbol {
    my ($type, $name, $value) = @_;
    
    if ($type eq 'scalar') {
        $SYMBOL_TABLE{scalar_symbols}{$name} = $value;
    } elsif ($type eq 'array') {
        $SYMBOL_TABLE{array_symbols}{$name} = $value;
    } elsif ($type eq 'hash') {
        $SYMBOL_TABLE{hash_symbols}{$name} = $value;
    } elsif ($type eq 'code') {
        $SYMBOL_TABLE{code_symbols}{$name} = $value;
    }
}

sub export_symbol {
    my ($type, $name) = @_;
    
    if ($type eq 'scalar' && exists $SYMBOL_TABLE{scalar_symbols}{$name}) {
        no strict 'refs';
        *{"main::$name"} = \$SYMBOL_TABLE{scalar_symbols}{$name};
        use strict 'refs';
    } elsif ($type eq 'array' && exists $SYMBOL_TABLE{array_symbols}{$name}) {
        no strict 'refs';
        *{"main::$name"} = \@{$SYMBOL_TABLE{array_symbols}{$name}};
        use strict 'refs';
    } elsif ($type eq 'hash' && exists $SYMBOL_TABLE{hash_symbols}{$name}) {
        no strict 'refs';
        *{"main::$name"} = \%{$SYMBOL_TABLE{hash_symbols}{$name}};
        use strict 'refs';
    } elsif ($type eq 'code' && exists $SYMBOL_TABLE{code_symbols}{$name}) {
        no strict 'refs';
        *{"main::$name"} = \&{$SYMBOL_TABLE{code_symbols}{$name}};
        use strict 'refs';
    }
}

# Package with complex exports and imports
package Exporter;
our @EXPORT = qw(export_symbols import_symbols);
our @EXPORT_OK = qw(dynamic_export conditional_export);

sub export_symbols {
    my (@symbols) = @_;
    
    for my $symbol (@symbols) {
        if (ref $symbol eq 'CODE') {
            # Export code reference
            no strict 'refs';
            my $name = generate_symbol_name();
            *{"Exporter::$name"} = $symbol;
            push @EXPORT, $name;
            use strict 'refs';
        }
    }
}

sub import_symbols {
    my ($package, @symbols) = @_;
    
    for my $symbol (@symbols) {
        no strict 'refs';
        if (defined &{"${package}::$symbol"}) {
            *{"main::$symbol"} = \&{"${package}::$symbol"};
        }
        use strict 'refs';
    }
}

sub dynamic_export {
    return "Dynamically exported function";
}

sub conditional_export {
    return "Conditionally exported function";
}

sub generate_symbol_name {
    return 'generated_' . int(rand(1000));
}

# Package with typeglob manipulation
package TypeGlobManipulator;
use Symbol qw(gensym);

sub create_anon_handle {
    my $handle = gensym;
    return $handle;
}

sub alias_subroutine {
    my ($old_name, $new_name) = @_;
    
    no strict 'refs';
    *{$new_name} = \&{$old_name};
    use strict 'refs';
}

sub alias_variable {
    my ($old_name, $new_name, $type) = @_;
    
    no strict 'refs';
    if ($type eq 'scalar') {
        *{$new_name} = \${$old_name};
    } elsif ($type eq 'array') {
        *{$new_name} = \@{$old_name};
    } elsif ($type eq 'hash') {
        *{$new_name} = \%{$old_name};
    }
    use strict 'refs';
}

# Back to main package
package main;

# Use statements with complex imports
use BaseModule qw(function1 optional_function);
use Exporter qw(export_symbols import_symbols);
use SymbolTable qw(import_symbol export_symbol);

# Complex symbol table operations
import_symbol('scalar', 'config_value', 'test_config');
import_symbol('array', 'data_array', [1, 2, 3, 4, 5]);
import_symbol('hash', 'config_hash', {key1 => 'value1', key2 => 'value2'});
import_symbol('code', 'process_func', sub { return $_[0] * 2 });

# Export symbols to main namespace
export_symbol('scalar', 'config_value');
export_symbol('array', 'data_array');
export_symbol('hash', 'config_hash');
export_symbol('code', 'process_func');

# Typeglob operations
my $anon_handle = TypeGlobManipulator::create_anon_handle();
TypeGlobManipulator::alias_subroutine('process_func', 'alias_process');
TypeGlobManipulator::alias_variable('config_value', 'config_alias', 'scalar');

# Complex package interactions
my $result = function1(); # From BaseModule
my $derived = DerivedModule::derived_function();
my $processed = process_func($result);

# Dynamic symbol resolution
my $dynamic_func = 'dynamic_export';
my $dynamic_result = Exporter::$dynamic_func();

# Complex namespace manipulation
{
    package TempNamespace;
    
    our $temp_var = 'temporary';
    sub temp_sub {
        return "temp subroutine";
    }
    
    # Import symbols into temp namespace
    import_symbol('scalar', 'imported_temp', 'imported_value');
}

# Access symbols from temp namespace
my $temp_result = TempNamespace::temp_sub();
my $temp_var_value = $TempNamespace::temp_var;
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify package declarations
    let package_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Package { .. }));
    assert!(!package_nodes.is_empty(), "Should have package declarations");

    // Verify our variable declarations
    let var_decls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::VariableDeclaration { .. }));
    assert!(!var_decls.is_empty(), "Should have variable declarations");

    // Verify subroutine declarations
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");

    // Verify use statements
    let use_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Use { .. }));
    assert!(!use_nodes.is_empty(), "Should have use statements");

    // Verify typeglob operations
    let typeglob_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Typeglob { .. }));
    assert!(!typeglob_nodes.is_empty(), "Should have typeglob operations");

    // Verify function calls
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!function_calls.is_empty(), "Should have function calls");

    // Verify method calls
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    assert!(!method_calls.is_empty(), "Should have method calls");
}

/// Test complex reference and dereferencing patterns
#[test]
fn test_complex_reference_dereferencing() {
    let code = r#"
# Basic references
my $scalar_ref = \$scalar;
my $array_ref = \@array;
my $hash_ref = \%hash;
my $code_ref = \&subroutine;
my $glob_ref = \*FILEHANDLE;

# References to references
my $ref_to_ref = \$scalar_ref;
my $array_ref_ref = \$array_ref;
my $hash_ref_ref = \%hash_ref;

# Complex dereferencing
my $scalar_value = $$scalar_ref;
my $array_element = $$array_ref[0];
my $hash_value = $$hash_ref{key};
my $sub_result = &$code_ref();
my $glob_handle = *$glob_ref;

# Nested dereferencing
my $deep_scalar = $$$ref_to_ref;
my $deep_array = $$array_ref_ref[0];
my $deep_hash = $$hash_ref_ref{key};

# Array and hash slice dereferencing
my @array_slice = @$array_ref[1, 3, 5];
my @hash_slice = @$hash_ref{qw(key1 key2 key3)};
my @complex_slice = @{$$ref_to_ref}[0, 2, 4];

# Complex reference expressions
my $complex_ref = \${$hash_ref{nested}{array}}[0];
my $very_complex = \${${$complex_ref}{deep}{structure}}[1]{value};

# References to anonymous structures
my $anon_array_ref = [1, 2, 3, 4, 5];
my $anon_hash_ref = {key1 => 'value1', key2 => 'value2'};
my $anon_code_ref = sub { return $_[0] * 2 };
my $anon_glob_ref = *{ANON_HANDLE};

# Complex anonymous structures
my $complex_anon = {
    users => [
        {
            name => 'Alice',
            data => {
                scores => [95, 87, 92],
                metadata => {
                    active => 1,
                    level => 'advanced'
                }
            }
        },
        {
            name => 'Bob',
            data => {
                scores => [88, 91, 85],
                metadata => {
                    active => 0,
                    level => 'intermediate'
                }
            }
        }
    ],
    config => {
        version => '1.0',
        features => [qw(scoring reporting export)]
    }
};

# Dereferencing complex anonymous structures
my $alice_scores = $complex_anon->{users}[0]{data}{scores};
my $bob_level = $complex_anon->{users}[1]{data}{metadata}{level};
my $features = $complex_anon->{config}{features};

# Reference counting and sharing
my $shared_ref = $complex_anon;
my $copy_ref = { %$complex_anon }; # Shallow copy
my $deep_copy = deep_clone($complex_anon);

# References in subroutines
sub process_references {
    my ($scalar_ref, $array_ref, $hash_ref) = @_;
    
    my $scalar_result = process_scalar($$scalar_ref);
    my $array_result = process_array(@$array_ref);
    my $hash_result = process_hash(%$hash_ref);
    
    return {
        scalar => $scalar_result,
        array => $array_result,
        hash => $hash_result
    };
}

sub deep_clone {
    my ($ref) = @_;
    
    if (ref $ref eq 'ARRAY') {
        return [map { deep_clone($_) } @$ref];
    } elsif (ref $ref eq 'HASH') {
        return {map { $_ => deep_clone($ref->{$_}) } keys %$ref};
    } elsif (ref $ref eq 'CODE') {
        return $ref; # Can't clone code refs
    } else {
        return $ref;
    }
}

sub process_scalar {
    my ($value) = @_;
    return uc $value;
}

sub process_array {
    my (@array) = @_;
    return scalar @array;
}

sub process_hash {
    my (%hash) = @_;
    return scalar keys %hash;
}

# Complex reference operations
my $result = process_references(\$scalar, \@array, \%hash);

# References with typeglob manipulation
my $code_ref_from_glob = *{process_references}{CODE};
my $array_ref_from_glob = *{array}{ARRAY};
my $hash_ref_from_glob = *{hash}{HASH};

# Dynamic dereferencing
my $ref_type = 'HASH';
my $ref_name = 'complex_anon';
my $dynamic_deref = ${"${ref_name}"}{users}[0]{name};

# Complex reference chains
my $ref_chain = \$complex_anon;
my $chain_result = $$$ref_chain->{users}[0]{data}{metadata}{level};
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify unary operations (reference creation)
    let unary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Unary { .. }));
    assert!(!unary_ops.is_empty(), "Should have unary operations for references");

    // Verify binary operations (dereferencing)
    let binary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Binary { .. }));
    assert!(!binary_ops.is_empty(), "Should have binary operations for dereferencing");

    // Verify anonymous structures
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));

    assert!(!array_literals.is_empty(), "Should have array literals");
    assert!(!hash_literals.is_empty(), "Should have hash literals");

    // Verify subroutine declarations
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");

    // Verify typeglob operations
    let typeglob_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Typeglob { .. }));
    assert!(!typeglob_nodes.is_empty(), "Should have typeglob operations");
}

/// Helper function to find nodes of specific kinds
fn find_nodes_of_kind<F>(node: &Node, predicate: F) -> Vec<&Node>
where
    F: Fn(&NodeKind) -> bool,
{
    let mut results = Vec::new();
    find_nodes_recursive(node, &predicate, &mut results);
    results
}

/// Recursive helper to find nodes matching predicate
fn find_nodes_recursive<'a, F>(node: &'a Node, predicate: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&NodeKind) -> bool,
{
    if predicate(&node.kind) {
        results.push(node);
    }

    // Recurse into child nodes based on node type
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            find_nodes_recursive(expression, predicate, results);
        }
        NodeKind::VariableDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            find_nodes_recursive(lhs, predicate, results);
            find_nodes_recursive(rhs, predicate, results);
        }
        NodeKind::Binary { left, right, .. } => {
            find_nodes_recursive(left, predicate, results);
            find_nodes_recursive(right, predicate, results);
        }
        NodeKind::Unary { operand, .. } => {
            find_nodes_recursive(operand, predicate, results);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_expr, predicate, results);
            find_nodes_recursive(else_expr, predicate, results);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_branch, predicate, results);
            for (_, branch) in elsif_branches {
                find_nodes_recursive(branch, predicate, results);
            }
            if let Some(else_branch) = else_branch {
                find_nodes_recursive(else_branch, predicate, results);
            }
        }
        NodeKind::While { condition, body, continue_block } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            if let Some(init) = init {
                find_nodes_recursive(init, predicate, results);
            }
            if let Some(cond) = condition {
                find_nodes_recursive(cond, predicate, results);
            }
            if let Some(upd) = update {
                find_nodes_recursive(upd, predicate, results);
            }
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(list, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            find_nodes_recursive(body, predicate, results);
            for (_, catch_body) in catch_blocks {
                find_nodes_recursive(catch_body, predicate, results);
            }
            if let Some(final_body) = finally_block {
                find_nodes_recursive(final_body, predicate, results);
            }
        }
        NodeKind::Given { expr, body } => {
            find_nodes_recursive(expr, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::When { condition, body } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Default { body } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Subroutine { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Method { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Class { body, name: _ } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::FunctionCall { args, name: _ } => {
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for element in elements {
                find_nodes_recursive(element, predicate, results);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, value) in pairs {
                find_nodes_recursive(key, predicate, results);
                find_nodes_recursive(value, predicate, results);
            }
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            find_nodes_recursive(statement, predicate, results);
            find_nodes_recursive(condition, predicate, results);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            find_nodes_recursive(statement, predicate, results);
        }
        NodeKind::Eval { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Do { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Return { value } => {
            if let Some(val) = value {
                find_nodes_recursive(val, predicate, results);
            }
        }
        NodeKind::LoopControl { .. } => {} // No children
        NodeKind::Tie { variable, package, args } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(package, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Untie { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Readline { .. } => {} // No complex children
        NodeKind::Diamond => {}         // No children
        NodeKind::Glob { .. } => {}     // No children
        NodeKind::Typeglob { .. } => {} // No children
        NodeKind::Number { .. } => {}   // No children
        NodeKind::String { .. } => {}   // No children
        NodeKind::Heredoc { .. } => {}  // No children
        NodeKind::Undef => {}           // No children
        NodeKind::Ellipsis => {}        // No children
        NodeKind::Regex { .. } => {}    // No children
        NodeKind::Match { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Substitution { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Transliteration { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Package { block, .. } => {
            if let Some(b) = block {
                find_nodes_recursive(b, predicate, results);
            }
        }
        NodeKind::Use { .. } => {} // No complex children
        NodeKind::No { .. } => {}  // No complex children
        NodeKind::PhaseBlock { block, .. } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::DataSection { .. } => {} // No children
        NodeKind::Format { .. } => {}      // No children
        NodeKind::Identifier { .. } => {}  // No children
        NodeKind::Variable { .. } => {}    // No children
        NodeKind::VariableWithAttributes { variable, .. } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Prototype { .. } => {} // No children
        NodeKind::Signature { parameters } => {
            for param in parameters {
                find_nodes_recursive(param, predicate, results);
            }
        }
        NodeKind::MandatoryParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(default_value, predicate, results);
        }
        NodeKind::SlurpyParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::NamedParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::IndirectCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Error { partial, .. } => {
            if let Some(p) = partial {
                find_nodes_recursive(p, predicate, results);
            }
        }
        NodeKind::MissingExpression
        | NodeKind::MissingStatement
        | NodeKind::MissingIdentifier
        | NodeKind::MissingBlock => {} // No children
        NodeKind::UnknownRest => {} // No children
    }
}
