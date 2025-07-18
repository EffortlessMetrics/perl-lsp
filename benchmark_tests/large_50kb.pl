#!/usr/bin/perl
use strict;
use warnings;
use feature qw(say state);

# Large 50KB test file simulating a complex Perl application
# Tests parser performance on larger codebases with diverse constructs

#############################################################################
# Package: Database::Schema
#############################################################################
package Database::Schema;

use constant {
    SCHEMA_VERSION => '2.1.0',
    MAX_CONNECTIONS => 50,
    DEFAULT_TIMEOUT => 30,
};

our @TABLES = qw(
    users roles permissions
    products orders order_items
    categories tags product_tags
    sessions audit_logs
    configurations cache_entries
);

sub new {
    my ($class, %args) = @_;
    my $self = {
        dbh => undef,
        schema => {},
        indexes => {},
        constraints => {},
        %args,
    };
    bless $self, $class;
    $self->_initialize_schema();
    return $self;
}

sub _initialize_schema {
    my $self = shift;
    
    # Users table
    $self->{schema}{users} = {
        columns => {
            id => 'INTEGER PRIMARY KEY AUTOINCREMENT',
            username => 'VARCHAR(50) UNIQUE NOT NULL',
            email => 'VARCHAR(100) UNIQUE NOT NULL',
            password_hash => 'VARCHAR(255) NOT NULL',
            first_name => 'VARCHAR(50)',
            last_name => 'VARCHAR(50)',
            created_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
            updated_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
            last_login => 'TIMESTAMP',
            is_active => 'BOOLEAN DEFAULT 1',
            failed_attempts => 'INTEGER DEFAULT 0',
        },
        indexes => [
            'CREATE INDEX idx_users_email ON users(email)',
            'CREATE INDEX idx_users_username ON users(username)',
            'CREATE INDEX idx_users_active ON users(is_active)',
        ],
    };
    
    # Products table
    $self->{schema}{products} = {
        columns => {
            id => 'INTEGER PRIMARY KEY AUTOINCREMENT',
            sku => 'VARCHAR(50) UNIQUE NOT NULL',
            name => 'VARCHAR(200) NOT NULL',
            description => 'TEXT',
            price => 'DECIMAL(10,2) NOT NULL',
            cost => 'DECIMAL(10,2)',
            weight => 'DECIMAL(8,3)',
            dimensions => 'JSON',
            category_id => 'INTEGER REFERENCES categories(id)',
            stock_quantity => 'INTEGER DEFAULT 0',
            is_active => 'BOOLEAN DEFAULT 1',
            created_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
            updated_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
        },
        indexes => [
            'CREATE INDEX idx_products_sku ON products(sku)',
            'CREATE INDEX idx_products_category ON products(category_id)',
            'CREATE INDEX idx_products_active ON products(is_active)',
            'CREATE INDEX idx_products_price ON products(price)',
        ],
    };
    
    # Add more tables...
    foreach my $table (@TABLES) {
        next if exists $self->{schema}{$table};
        $self->{schema}{$table} = $self->_generate_default_schema($table);
    }
}

sub _generate_default_schema {
    my ($self, $table_name) = @_;
    return {
        columns => {
            id => 'INTEGER PRIMARY KEY AUTOINCREMENT',
            name => 'VARCHAR(100)',
            created_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
            updated_at => 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
        },
        indexes => [],
    };
}

#############################################################################
# Package: Web::Application
#############################################################################
package Web::Application;

use base qw(Web::Framework);

our $VERSION = '3.14.159';

# Route definitions
our %ROUTES = (
    'GET /' => 'home',
    'GET /about' => 'about',
    'GET /products' => 'list_products',
    'GET /products/:id' => 'show_product',
    'POST /products' => 'create_product',
    'PUT /products/:id' => 'update_product',
    'DELETE /products/:id' => 'delete_product',
    'GET /api/v1/users' => 'api_list_users',
    'POST /api/v1/auth/login' => 'api_login',
    'POST /api/v1/auth/logout' => 'api_logout',
);

# Middleware stack
our @MIDDLEWARE = qw(
    Logger
    ErrorHandler
    Session
    Authentication
    CSRF
    RateLimit
    Compression
);

sub new {
    my ($class, %config) = @_;
    
    my $self = $class->SUPER::new(%config);
    
    $self->{template_engine} = $config{template_engine} || 'TT';
    $self->{static_path} = $config{static_path} || './public';
    $self->{upload_path} = $config{upload_path} || './uploads';
    $self->{session_config} = $config{session} || {
        expires => 3600,
        secure => 1,
        httponly => 1,
    };
    
    $self->_setup_routes();
    $self->_setup_middleware();
    
    return $self;
}

sub _setup_routes {
    my $self = shift;
    
    while (my ($route, $handler) = each %ROUTES) {
        my ($method, $path) = split /\s+/, $route, 2;
        
        $self->add_route($method, $path, sub {
            my $req = shift;
            my $method_name = "handle_$handler";
            
            if ($self->can($method_name)) {
                return $self->$method_name($req);
            }
            else {
                return $self->error_404($req);
            }
        });
    }
}

sub handle_home {
    my ($self, $req) = @_;
    
    my $data = {
        title => 'Welcome',
        user => $req->user,
        featured_products => $self->get_featured_products(),
        recent_posts => $self->get_recent_posts(5),
    };
    
    return $self->render('home', $data);
}

sub handle_list_products {
    my ($self, $req) = @_;
    
    my $page = $req->param('page') || 1;
    my $per_page = $req->param('per_page') || 20;
    my $sort = $req->param('sort') || 'name';
    my $order = $req->param('order') || 'asc';
    
    # Validate parameters
    $page = 1 if $page !~ /^\d+$/ || $page < 1;
    $per_page = 20 if $per_page !~ /^\d+$/ || $per_page < 1 || $per_page > 100;
    $sort = 'name' unless $sort =~ /^(name|price|created_at)$/;
    $order = 'asc' unless $order =~ /^(asc|desc)$/;
    
    my $offset = ($page - 1) * $per_page;
    
    my $products = $self->db->select(
        'products',
        ['*'],
        { is_active => 1 },
        {
            order_by => "$sort $order",
            limit => $per_page,
            offset => $offset,
        }
    );
    
    my $total = $self->db->count('products', { is_active => 1 });
    my $total_pages = int(($total + $per_page - 1) / $per_page);
    
    return $self->render('products/list', {
        products => $products,
        pagination => {
            current_page => $page,
            total_pages => $total_pages,
            per_page => $per_page,
            total_items => $total,
        },
    });
}

#############################################################################
# Package: Data::Validator
#############################################################################
package Data::Validator;

use Email::Valid;
use Scalar::Util qw(looks_like_number);

our %RULES = (
    required => sub {
        my ($value) = @_;
        return defined $value && length($value) > 0;
    },
    
    email => sub {
        my ($value) = @_;
        return Email::Valid->address($value) ? 1 : 0;
    },
    
    min_length => sub {
        my ($value, $min) = @_;
        return length($value) >= $min;
    },
    
    max_length => sub {
        my ($value, $max) = @_;
        return length($value) <= $max;
    },
    
    numeric => sub {
        my ($value) = @_;
        return looks_like_number($value);
    },
    
    integer => sub {
        my ($value) = @_;
        return $value =~ /^-?\d+$/;
    },
    
    positive => sub {
        my ($value) = @_;
        return looks_like_number($value) && $value > 0;
    },
    
    regex => sub {
        my ($value, $pattern) = @_;
        return $value =~ /$pattern/;
    },
    
    in => sub {
        my ($value, $allowed) = @_;
        return grep { $_ eq $value } @$allowed;
    },
    
    date => sub {
        my ($value) = @_;
        return $value =~ /^\d{4}-\d{2}-\d{2}$/;
    },
    
    datetime => sub {
        my ($value) = @_;
        return $value =~ /^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}$/;
    },
);

sub new {
    my ($class, %args) = @_;
    bless {
        rules => { %RULES, %{$args{custom_rules} || {}} },
        errors => {},
    }, $class;
}

sub validate {
    my ($self, $data, $schema) = @_;
    
    $self->{errors} = {};
    my $is_valid = 1;
    
    foreach my $field (keys %$schema) {
        my $rules = $schema->{$field};
        my $value = $data->{$field};
        
        # Convert single rule to array
        $rules = [$rules] unless ref $rules eq 'ARRAY';
        
        foreach my $rule (@$rules) {
            my ($rule_name, @args) = ref $rule eq 'ARRAY' ? @$rule : ($rule);
            
            if (my $validator = $self->{rules}{$rule_name}) {
                unless ($validator->($value, @args)) {
                    push @{$self->{errors}{$field}}, $self->_format_error($rule_name, $field, @args);
                    $is_valid = 0;
                }
            }
            else {
                warn "Unknown validation rule: $rule_name";
            }
        }
    }
    
    return $is_valid;
}

sub _format_error {
    my ($self, $rule, $field, @args) = @_;
    
    my %messages = (
        required => "$field is required",
        email => "$field must be a valid email address",
        min_length => "$field must be at least $args[0] characters",
        max_length => "$field must not exceed $args[0] characters",
        numeric => "$field must be a number",
        integer => "$field must be an integer",
        positive => "$field must be positive",
        date => "$field must be a valid date (YYYY-MM-DD)",
        datetime => "$field must be a valid datetime",
    );
    
    return $messages{$rule} || "$field failed validation rule: $rule";
}

#############################################################################
# Package: Utils::Text
#############################################################################
package Utils::Text;

use Exporter 'import';
use Encode qw(encode decode);
use HTML::Entities;
use Text::Wrap;
use Unicode::Normalize;

our @EXPORT_OK = qw(
    trim ltrim rtrim
    slugify truncate
    escape_html unescape_html
    word_wrap remove_accents
    camel_case snake_case
    pluralize singularize
);

sub trim {
    my $text = shift;
    return unless defined $text;
    $text =~ s/^\s+|\s+$//g;
    return $text;
}

sub ltrim {
    my $text = shift;
    return unless defined $text;
    $text =~ s/^\s+//;
    return $text;
}

sub rtrim {
    my $text = shift;
    return unless defined $text;
    $text =~ s/\s+$//;
    return $text;
}

sub slugify {
    my ($text, $separator) = @_;
    $separator //= '-';
    
    return '' unless defined $text;
    
    # Convert to lowercase
    $text = lc($text);
    
    # Remove accents
    $text = remove_accents($text);
    
    # Replace non-alphanumeric characters with separator
    $text =~ s/[^a-z0-9]+/$separator/g;
    
    # Remove leading/trailing separators
    $text =~ s/^$separator+|$separator+$//g;
    
    return $text;
}

sub truncate {
    my ($text, $length, $suffix) = @_;
    $suffix //= '...';
    
    return '' unless defined $text;
    return $text if length($text) <= $length;
    
    my $truncated = substr($text, 0, $length - length($suffix));
    
    # Try to break at word boundary
    if ($truncated =~ s/\s+\S*$//) {
        return $truncated . $suffix;
    }
    
    return $truncated . $suffix;
}

sub escape_html {
    my $text = shift;
    return encode_entities($text, '<>&"\'');
}

sub unescape_html {
    my $text = shift;
    return decode_entities($text);
}

sub word_wrap {
    my ($text, $columns) = @_;
    $columns ||= 72;
    
    local $Text::Wrap::columns = $columns;
    return wrap('', '', $text);
}

sub remove_accents {
    my $text = shift;
    return '' unless defined $text;
    
    # Normalize to NFD (decomposed form)
    $text = NFD($text);
    
    # Remove combining characters
    $text =~ s/\p{Mn}//g;
    
    return $text;
}

sub camel_case {
    my $text = shift;
    return '' unless defined $text;
    
    # Split on underscores or spaces
    my @parts = split /[_\s]+/, $text;
    
    # Capitalize first letter of each part
    return join '', map { ucfirst(lc($_)) } @parts;
}

sub snake_case {
    my $text = shift;
    return '' unless defined $text;
    
    # Insert underscore before uppercase letters
    $text =~ s/([A-Z])/_$1/g;
    
    # Convert to lowercase and remove leading underscore
    $text = lc($text);
    $text =~ s/^_//;
    
    # Replace spaces with underscores
    $text =~ s/\s+/_/g;
    
    return $text;
}

#############################################################################
# Package: Test::Framework
#############################################################################
package Test::Framework;

use Time::HiRes qw(time);
use Data::Dumper;

our $TEST_COUNT = 0;
our $FAILED_COUNT = 0;
our @TEST_RESULTS = ();

sub new {
    my ($class, %args) = @_;
    bless {
        verbose => $args{verbose} || 0,
        bail_on_fail => $args{bail_on_fail} || 0,
        current_suite => undef,
    }, $class;
}

sub describe {
    my ($self, $suite_name, $code) = @_;
    
    $self->{current_suite} = $suite_name;
    print "\n=== Test Suite: $suite_name ===\n" if $self->{verbose};
    
    my $suite_start = time;
    $code->();
    my $suite_duration = time - $suite_start;
    
    printf "Suite completed in %.3fs\n", $suite_duration if $self->{verbose};
    $self->{current_suite} = undef;
}

sub it {
    my ($self, $test_name, $code) = @_;
    
    $TEST_COUNT++;
    my $start_time = time;
    
    my $result = eval {
        $code->();
        1;
    };
    
    my $duration = time - $start_time;
    my $error = $@;
    
    if ($result) {
        push @TEST_RESULTS, {
            suite => $self->{current_suite},
            name => $test_name,
            status => 'passed',
            duration => $duration,
        };
        
        print "  ✓ $test_name (${duration}s)\n" if $self->{verbose};
    }
    else {
        $FAILED_COUNT++;
        push @TEST_RESULTS, {
            suite => $self->{current_suite},
            name => $test_name,
            status => 'failed',
            duration => $duration,
            error => $error,
        };
        
        print "  ✗ $test_name (${duration}s)\n" if $self->{verbose};
        print "    Error: $error\n" if $self->{verbose} && $error;
        
        die "Bail on first failure\n" if $self->{bail_on_fail};
    }
}

sub expect {
    my ($self, $actual) = @_;
    return Test::Framework::Expectation->new($actual);
}

sub run_all {
    my $self = shift;
    
    print "\n" . "=" x 60 . "\n";
    print "Test Summary:\n";
    print "=" x 60 . "\n";
    printf "Total tests: %d\n", $TEST_COUNT;
    printf "Passed: %d\n", $TEST_COUNT - $FAILED_COUNT;
    printf "Failed: %d\n", $FAILED_COUNT;
    print "=" x 60 . "\n";
    
    if ($FAILED_COUNT > 0) {
        print "\nFailed Tests:\n";
        foreach my $result (@TEST_RESULTS) {
            next unless $result->{status} eq 'failed';
            printf "  - %s: %s\n", $result->{suite} || 'Global', $result->{name};
            printf "    %s\n", $result->{error} if $result->{error};
        }
    }
    
    return $FAILED_COUNT == 0;
}

package Test::Framework::Expectation;

sub new {
    my ($class, $actual) = @_;
    bless { actual => $actual }, $class;
}

sub to_equal {
    my ($self, $expected) = @_;
    
    my $actual = $self->{actual};
    
    if (!defined $actual && !defined $expected) {
        return 1;
    }
    elsif (!defined $actual || !defined $expected) {
        die "Expected " . ($expected // 'undef') . " but got " . ($actual // 'undef');
    }
    elsif (ref $actual eq 'ARRAY' && ref $expected eq 'ARRAY') {
        $self->_compare_arrays($actual, $expected);
    }
    elsif (ref $actual eq 'HASH' && ref $expected eq 'HASH') {
        $self->_compare_hashes($actual, $expected);
    }
    elsif ($actual ne $expected) {
        die "Expected '$expected' but got '$actual'";
    }
    
    return 1;
}

sub to_be_true {
    my $self = shift;
    die "Expected true value but got " . ($self->{actual} // 'undef') unless $self->{actual};
    return 1;
}

sub to_be_false {
    my $self = shift;
    die "Expected false value but got '$self->{actual}'" if $self->{actual};
    return 1;
}

sub to_be_defined {
    my $self = shift;
    die "Expected defined value but got undef" unless defined $self->{actual};
    return 1;
}

sub to_match {
    my ($self, $pattern) = @_;
    die "Expected to match /$pattern/ but got " . ($self->{actual} // 'undef')
        unless defined $self->{actual} && $self->{actual} =~ /$pattern/;
    return 1;
}

#############################################################################
# Main Application Code
#############################################################################
package main;

use strict;
use warnings;
use feature 'say';

# Example application using all the modules defined above

# Initialize components
my $validator = Data::Validator->new();
my $test = Test::Framework->new(verbose => 1);

# Run validation tests
$test->describe('Data Validation', sub {
    $test->it('validates required fields', sub {
        my $result = $validator->validate(
            { name => 'John' },
            { name => 'required' }
        );
        $test->expect($result)->to_be_true();
    });
    
    $test->it('validates email addresses', sub {
        my $result = $validator->validate(
            { email => 'test@example.com' },
            { email => ['required', 'email'] }
        );
        $test->expect($result)->to_be_true();
    });
    
    $test->it('validates numeric ranges', sub {
        my $result = $validator->validate(
            { age => 25 },
            { age => ['numeric', 'positive'] }
        );
        $test->expect($result)->to_be_true();
    });
});

# Text utility tests
$test->describe('Text Utilities', sub {
    $test->it('generates slugs correctly', sub {
        my $slug = Utils::Text::slugify("Hello World! 123");
        $test->expect($slug)->to_equal('hello-world-123');
    });
    
    $test->it('converts to camel case', sub {
        my $camel = Utils::Text::camel_case("hello_world_test");
        $test->expect($camel)->to_equal('HelloWorldTest');
    });
    
    $test->it('truncates text properly', sub {
        my $truncated = Utils::Text::truncate("This is a long text", 10);
        $test->expect($truncated)->to_equal('This...');
    });
});

# Complex regex patterns
my @patterns = (
    qr/(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/,
    qr/\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b/,
    qr/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$/,
);

# Heredocs with different quoting styles
my $config = <<'CONFIG';
database:
  host: localhost
  port: 5432
  name: myapp
  user: dbuser
CONFIG

my $template = <<"TEMPLATE";
Dear $name,

Thank you for your order #$order_id.
Your items will be shipped to:

$address

Best regards,
The Team
TEMPLATE

my $script = <<`SCRIPT`;
#!/bin/bash
echo "Running system check..."
df -h
free -m
SCRIPT

# Format blocks
format REPORT =
@<<<<<<<<<<<<<< @>>>>>> @###.##
$item, $date, $price
.

format REPORT_TOP =
Item            Date    Price
=============== ======= =======
.

# Typeglobs and symbol table manipulation
*alias = *original;
*{$package . '::function'} = sub { return "Dynamic function" };

my $scalar_ref = *STDOUT{SCALAR};
my $array_ref = *ARGV{ARRAY};
my $hash_ref = *ENV{HASH};
my $code_ref = *CORE::print{CODE};

# Tie examples
{
    package TiedHash;
    
    sub TIEHASH {
        my $class = shift;
        bless { data => {}, @_ }, $class;
    }
    
    sub FETCH {
        my ($self, $key) = @_;
        return $self->{data}{$key};
    }
    
    sub STORE {
        my ($self, $key, $value) = @_;
        $self->{data}{$key} = $value;
    }
    
    sub DELETE {
        my ($self, $key) = @_;
        delete $self->{data}{$key};
    }
    
    sub EXISTS {
        my ($self, $key) = @_;
        exists $self->{data}{$key};
    }
    
    sub FIRSTKEY {
        my $self = shift;
        my $a = keys %{$self->{data}};
        each %{$self->{data}};
    }
    
    sub NEXTKEY {
        my ($self, $lastkey) = @_;
        each %{$self->{data}};
    }
}

tie my %tied_hash, 'TiedHash';
$tied_hash{foo} = 'bar';

# Advanced operators
my $result = $value // $default;
my $match = $string ~~ @array;
my $no_match = $string !~ /pattern/;
my $numeric_eq = $a <=> $b;
my $string_eq = $a cmp $b;

# File test operators
if (-e $file && -r _ && -w _ && !-d _) {
    say "File is readable and writable";
}

# Special variables
local $/ = undef;  # Slurp mode
local $\ = "\n";   # Output record separator
local $, = ", ";   # Output field separator
local $" = " ";    # List separator

# Run tests
$test->run_all();

# Benchmark data structures
my %large_hash = map { $_ => { 
    id => $_,
    value => rand(1000),
    timestamp => time,
    data => [ map { rand(100) } 1..10 ],
}} 1..1000;

my @large_array = map {{
    index => $_,
    squared => $_ ** 2,
    cubed => $_ ** 3,
    sqrt => sqrt($_),
    log => log($_),
}} 1..1000;

# More subroutines with various signatures
sub variadic_sub { 
    my ($first, @rest) = @_;
    return $first + sum(@rest);
}

sub named_params {
    my %args = @_;
    return $args{foo} * $args{bar};
}

sub with_prototype ($$$) {
    my ($x, $y, $z) = @_;
    return $x + $y + $z;
}

sub recursive_factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * recursive_factorial($n - 1);
}

# Closures and anonymous subs
my $multiplier = sub {
    my $factor = shift;
    return sub {
        my $value = shift;
        return $value * $factor;
    };
};

my $times_two = $multiplier->(2);
my $times_ten = $multiplier->(10);

say "2 * 5 = " . $times_two->(5);
say "10 * 5 = " . $times_ten->(5);

__END__

=head1 NAME

Benchmark Test File - 50KB Perl Code

=head1 DESCRIPTION

This file contains various Perl constructs and patterns to test
parser performance on medium-sized files.

=head1 MODULES

=over 4

=item * Database::Schema - Database schema management

=item * Web::Application - Web framework example

=item * Data::Validator - Input validation

=item * Utils::Text - Text manipulation utilities

=item * Test::Framework - Testing framework

=back

=cut