#!/usr/bin/env perl
# Test: Enhanced Given/When/Default Production Scenarios
# Impact: Comprehensive testing of switch-like constructs and smart matching
# NodeKinds: Given, When, Default, Match
# 
# This file tests the parser's ability to handle:
# 1. Complex given/when/default patterns with real-world data
# 2. Advanced smart matching scenarios
# 3. Nested given/when structures
# 4. Performance-optimized switch patterns
# 5. Business logic with conditional branching
# 6. Type-aware matching and dispatch
# 7. Error handling and fallback patterns
# 8. Cross-context given/when usage

use strict;
use warnings;
use feature 'switch';
no warnings 'experimental::smartmatch';

# Helper function to simulate smart matching for older Perl versions
sub smart_match {
    my ($left, $right) = @_;
    
    # String ~~ String (equality)
    if (!ref($left) && !ref($right)) {
        return $left eq $right;
    }
    
    # String ~~ Regex (pattern match)
    if (!ref($left) && ref($right) eq 'Regexp') {
        return $left =~ $right;
    }
    
    # Number ~~ Array (contains)
    if (!ref($left) && ref($right) eq 'ARRAY') {
        return grep { $left == $_ } @$right;
    }
    
    # String ~~ Array (contains)
    if (!ref($left) && ref($right) eq 'ARRAY') {
        return grep { $left eq $_ } @$right;
    }
    
    # Array ~~ Array (array equality)
    if (ref($left) eq 'ARRAY' && ref($right) eq 'ARRAY') {
        return 0 if @$left != @$right;
        for my $i (0..$#$left) {
            return 0 if $left->[$i] ne $right->[$i];
        }
        return 1;
    }
    
    # Hash ~~ Hash (hash equality)
    if (ref($left) eq 'HASH' && ref($right) eq 'HASH') {
        my @left_keys = sort keys %$left;
        my @right_keys = sort keys %$right;
        return 0 if @left_keys != @right_keys;
        for my $key (@left_keys) {
            return 0 unless exists $right->{$key};
            return 0 if $left->{$key} ne $right->{$key};
        }
        return 1;
    }
    
    # String ~~ Hash (key exists)
    if (!ref($left) && ref($right) eq 'HASH') {
        return exists $right->{$left};
    }
    
    # Code ref ~~ Any (coderef test)
    if (ref($right) eq 'CODE') {
        return $right->($left);
    }
    
    # Type matching
    if (!ref($left) && ref($right) eq '') {
        if ($right eq 'ARRAY') { return ref($left) eq 'ARRAY'; }
        if ($right eq 'HASH') { return ref($left) eq 'HASH'; }
        if ($right eq 'CODE') { return ref($left) eq 'CODE'; }
        if ($right eq 'SCALAR') { return ref($left) eq 'SCALAR'; }
    }
    
    return 0;
}

# Simulate given/when/default for compatibility
sub given_when_default {
    my ($value, $cases) = @_;
    
    for my $case (@$cases) {
        my ($when_condition, $when_code, $is_default) = @$case;
        
        if ($is_default || smart_match($value, $when_condition)) {
            return $when_code->($value);
        }
    }
    
    return undef;
}

print "=== Enhanced Given/When/Default Production Tests ===\n\n";

# Test 1: Business logic dispatch system
print "=== Business Logic Dispatch ===\n";

sub process_business_request {
    my ($request) = @_;
    
    return given_when_default($request->{type}, [
        [qr/^user_/ => sub {
            my ($req) = @_;
            return "Processing user request: " . $req->{action};
        }],
        [qr/^admin_/ => sub {
            my ($req) = @_;
            return "Processing admin request: " . $req->{action};
        }],
        [qr/^system_/ => sub {
            my ($req) = @_;
            return "Processing system request: " . $req->{action};
        }],
        ['default' => sub {
            my ($req) = @_;
            return "Unknown request type: " . $req->{type};
        }, 1]  # Default case
    ]);
}

my @requests = (
    { type => 'user_create', action => 'create_user' },
    { type => 'admin_delete', action => 'delete_user' },
    { type => 'system_backup', action => 'backup_database' },
    { type => 'unknown_type', action => 'mystery_action' }
);

for my $req (@requests) {
    my $result = process_business_request($req);
    print "Request $req->{type}: $result\n";
}

print "\n";

# Test 2: Data type validation and processing
print "=== Data Type Validation ===\n";

sub validate_and_process {
    my ($data) = @_;
    
    return given_when_default($data, [
        ['ARRAY' => sub {
            my ($arr) = @_;
            return "Array with " . scalar(@$arr) . " elements: " . join(', ', @$arr);
        }],
        ['HASH' => sub {
            my ($hash) = @_;
            my $keys = join(', ', sort keys %$hash);
            return "Hash with keys: $keys";
        }],
        ['CODE' => sub {
            my ($code) = @_;
            my $result = eval { $code->() };
            return "Code ref executed, result: " . (defined $result ? $result : 'undef');
        }],
        [qr/^\d+$/ => sub {
            my ($num) = @_;
            return "Integer: $num (squared: " . ($num * $num) . ")";
        }],
        [qr/^[a-zA-Z]+$/ => sub {
            my ($str) = @_;
            return "String: $str (length: " . length($str) . ", upper: " . uc($str) . ")";
        }],
        ['default' => sub {
            my ($value) = @_;
            return "Other type: " . (ref($value) || 'scalar') . " = " . (defined $value ? $value : 'undef');
        }, 1]
    ]);
}

my @test_data = (
    [1, 2, 3, 4],
    { name => 'John', age => 30 },
    sub { return "Hello from code ref"; },
    42,
    "hello",
    \*STDOUT,
    undef
);

for my $data (@test_data) {
    my $result = validate_and_process($data);
    print "Data processing: $result\n";
}

print "\n";

# Test 3: Configuration management with smart matching
print "=== Configuration Management ===\n";

my %config = (
    debug => 1,
    verbose => 0,
    log_level => 'INFO',
    features => [qw(auth logging backup)],
    database => { driver => 'mysql', host => 'localhost' }
);

sub get_config_value {
    my ($key) = @_;
    
    return given_when_default($key, [
        # Direct hash key access
        sub { exists $config{$_[0]} ? $config{$_[0]} : undef } => sub {
            my ($k) = @_;
            return "Direct config: $k = " . $config{$k};
        },
        
        # Feature flag checking
        [qr/^feature_(.+)$/ => sub {
            my ($k) = @_;
            my $feature = $1;
            return "Feature $feature: " . (grep { $_ eq $feature } @{$config{features}} ? "ENABLED" : "DISABLED");
        }],
        
        # Database configuration
        [qr/^db_(.+)$/ => sub {
            my ($k) = @_;
            my $db_key = $1;
            return "Database $db_key: " . ($config{database}{$db_key} || 'not set');
        }],
        
        # Default case
        ['default' => sub {
            my ($k) = @_;
            return "Unknown config key: $k";
        }, 1]
    ]);
}

my @config_keys = qw(debug verbose log_level feature_auth feature_nonexistent db_driver db_host unknown_key);

for my $key (@config_keys) {
    my $result = get_config_value($key);
    print "Config query '$key': $result\n";
}

print "\n";

# Test 4: HTTP request routing
print "=== HTTP Request Routing ===\n";

sub route_http_request {
    my ($method, $path) = @_;
    
    return given_when_default($path, [
        [qr/^\/api\/users\/(\d+)$/ => sub {
            my ($p) = @_;
            return "GET user by ID: $1 (method: $method)";
        }],
        [qr'^/api/users$' => sub {
            my ($p) = @_;
            return "$method users collection";
        }],
        [qr'^/api/posts/(\d+)/comments$' => sub {
            my ($p) = @_;
            return "GET comments for post $1 (method: $method)";
        }],
        [qr'^/health$' => sub {
            my ($p) = @_;
            return "Health check endpoint (method: $method)";
        }],
        [qr'^/admin/' => sub {
            my ($p) = @_;
            return "Admin area access (method: $method)";
        }],
        ['default' => sub {
            my ($p) = @_;
            return "404 Not Found: $method $p";
        }, 1]
    ]);
}

my @routes = (
    ['GET', '/api/users/123'],
    ['POST', '/api/users'],
    ['GET', '/api/posts/456/comments'],
    ['GET', '/health'],
    ['DELETE', '/admin/users'],
    ['GET', '/nonexistent/path']
);

for my $route (@routes) {
    my ($method, $path) = @$route;
    my $result = route_http_request($method, $path);
    print "Route $method $path: $result\n";
}

print "\n";

# Test 5: Error classification and handling
print "=== Error Classification ===\n";

sub classify_error {
    my ($error_message) = @_;
    
    return given_when_default($error_message, [
        [qr/permission denied|access denied|unauthorized/i => sub {
            return "AUTHENTICATION_ERROR";
        }],
        [qr/not found|missing|does not exist/i => sub {
            return "NOT_FOUND_ERROR";
        }],
        [qr/connection|network|timeout/i => sub {
            return "NETWORK_ERROR";
        }],
        [qr/database|sql|query/i => sub {
            return "DATABASE_ERROR";
        }],
        [qr/syntax|parse|unexpected/i => sub {
            return "SYNTAX_ERROR";
        }],
        [qr/invalid|bad|malformed/i => sub {
            return "VALIDATION_ERROR";
        }],
        ['default' => sub {
            return "UNKNOWN_ERROR";
        }, 1]
    ]);
}

my @error_messages = (
    "Permission denied for user 'guest'",
    "File not found: /path/to/file.txt",
    "Network connection timeout after 30 seconds",
    "Database query failed: syntax error near 'SELECT'",
    "Invalid input: malformed JSON",
    "Some mysterious error occurred"
);

for my $error (@error_messages) {
    my $classification = classify_error($error);
    print "Error: '$error' -> $classification\n";
}

print "\n";

# Test 6: Complex nested data structure matching
print "=== Complex Data Structure Matching ===\n";

sub analyze_data_structure {
    my ($data) = @_;
    
    return given_when_default($data, [
        # Array of specific pattern
        [[1, 2, 3] => sub {
            return "Exact array match: [1, 2, 3]";
        }],
        
        # Array with specific length
        sub { ref($_[0]) eq 'ARRAY' && @{$_[0]} == 3 } => sub {
            my ($arr) = @_;
            return "Array with 3 elements: [" . join(', ', @$arr) . "]";
        },
        
        # Hash with specific keys
        sub { ref($_[0]) eq 'HASH' && exists $_[0]->{name} && exists $_[0]->{age} } => sub {
            my ($hash) = @_;
            return "Person hash: name=$hash->{name}, age=$hash->{age}";
        },
        
        # Nested structure
        sub { ref($_[0]) eq 'HASH' && ref($_[0]->{nested}) eq 'ARRAY' } => sub {
            my ($hash) = @_;
            return "Hash with nested array: " . scalar(@{$hash->{nested}}) . " items";
        },
        
        # String patterns
        [qr/^\d{4}-\d{2}-\d{2}$/ => sub {
            my ($date) = @_;
            return "Date string: $date";
        }],
        
        [qr/^\w+@\w+\.\w+$/ => sub {
            my ($email) = @_;
            return "Email address: $email";
        }],
        
        ['default' => sub {
            my ($value) = @_;
            return "Other structure: " . (ref($value) || 'scalar');
        }, 1]
    ]);
}

my @complex_data = (
    [1, 2, 3],
    [4, 5, 6],
    { name => 'Alice', age => 30 },
    { nested => [1, 2, 3, 4] },
    '2023-12-25',
    'user@example.com',
    { other => 'data' }
);

for my $data (@complex_data) {
    my $analysis = analyze_data_structure($data);
    print "Data analysis: $analysis\n";
}

print "\n";

# Test 7: Performance-optimized dispatch table
print "=== Performance-Optimized Dispatch ===\n";

# Create a dispatch table for common operations
my %dispatch_table = (
    'CREATE' => sub { my ($data) = @_; return "Created: " . join(', ', @$data); },
    'READ'   => sub { my ($data) = @_; return "Read: " . join(', ', @$data); },
    'UPDATE' => sub { my ($data) = @_; return "Updated: " . join(', ', @$data); },
    'DELETE' => sub { my ($data) = @_; return "Deleted: " . join(', ', @$data); }
);

sub crud_operation {
    my ($operation, $data) = @_;
    
    return given_when_default(uc($operation), [
        # Fast path for common operations
        sub { exists $dispatch_table{$_[0]} } => sub {
            my ($op) = @_;
            return $dispatch_table{$op}->($data);
        },
        
        # Batch operations
        [qr/^BATCH_(CREATE|READ|UPDATE|DELETE)$/ => sub {
            my ($op) = @_;
            my $base_op = $1;
            return "Batch $base_op: " . scalar(@$data) . " items";
        }],
        
        # Default case
        ['default' => sub {
            my ($op) = @_;
            return "Unknown operation: $op";
        }, 1]
    ]);
}

my @crud_tests = (
    ['create', ['user1', 'user2']],
    ['READ', ['item1', 'item2', 'item3']],
    ['Update', ['record1']],
    ['delete', ['old_item']],
    ['batch_create', ['item1', 'item2', 'item3', 'item4']],
    ['unknown', ['data']]
);

for my $test (@crud_tests) {
    my ($op, $data) = @$test;
    my $result = crud_operation($op, $data);
    print "CRUD $op: $result\n";
}

print "\n";

# Test 8: Multi-level conditional logic
print "=== Multi-Level Conditional Logic ===\n";

sub complex_decision {
    my ($input) = @_;
    
    # First level: Type classification
    my $type = given_when_default($input, [
        sub { ref($_[0]) eq 'HASH' } => sub { return 'hash'; },
        sub { ref($_[0]) eq 'ARRAY' } => sub { return 'array'; },
        sub { !ref($_[0]) && $_[0] =~ /^\d+$/ } => sub { return 'number'; },
        sub { !ref($_[0]) } => sub { return 'string'; },
        ['default' => sub { return 'other'; }, 1]
    ]);
    
    # Second level: Type-specific processing
    return given_when_default($type, [
        ['hash' => sub {
            my $hash = $input;
            my $size = scalar keys %$hash;
            return given_when_default($size, [
                [0 => sub { return "Empty hash"; }],
                [1 => sub { return "Single key hash"; }],
                [2..5 => sub { return "Small hash ($size keys)"; }],
                ['default' => sub { return "Large hash ($size keys)"; }, 1]
            ]);
        }],
        ['array' => sub {
            my $array = $input;
            my $size = scalar @$array;
            return given_when_default($size, [
                [0 => sub { return "Empty array"; }],
                [1 => sub { return "Single element array"; }],
                [2..3 => sub { return "Small array ($size elements)"; }],
                ['default' => sub { return "Large array ($size elements)"; }, 1]
            ]);
        }],
        ['number' => sub {
            my $num = $input;
            return given_when_default($num, [
                sub { $_[0] < 0 } => sub { return "Negative number: $num"; },
                sub { $_[0] == 0 } => sub { return "Zero"; },
                sub { $_[0] < 10 } => sub { return "Single digit: $num"; },
                sub { $_[0] < 100 } => sub { return "Two digit: $num"; },
                ['default' => sub { return "Large number: $num"; }, 1]
            ]);
        }],
        ['string' => sub {
            my $str = $input;
            my $len = length($str);
            return given_when_default($len, [
                [0 => sub { return "Empty string"; }],
                [1 => sub { return "Single character: '$str'"; }],
                [2..10 => sub { return "Short string ($len chars): '$str'"; }],
                ['default' => sub { return "Long string ($len chars): " . substr($str, 0, 20) . "..."; }, 1]
            ]);
        }],
        ['default' => sub {
            return "Other type: " . ref($input);
        }, 1]
    ]);
}

my @complex_inputs = (
    {},
    { a => 1, b => 2, c => 3, d => 4, e => 5 },
    [],
    [1],
    [1, 2],
    -5,
    0,
    7,
    123,
    '',
    'x',
    'hello world',
    \*STDOUT
);

for my $input (@complex_inputs) {
    my $result = complex_decision($input);
    my $input_desc = ref($input) ? ref($input) : (defined $input ? $input : 'undef');
    print "Input '$input_desc' -> $result\n";
}

print "\n=== Enhanced Given/When/Default Production Tests Completed ===\n";
print "This file demonstrates comprehensive given/when/default patterns\n";
print "with smart matching for production Perl applications.\n";
print "All examples use compatibility functions for broader Perl version support.\n";