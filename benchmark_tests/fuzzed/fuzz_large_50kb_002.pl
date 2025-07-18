#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/large_50kb.pl
# Mutation: 2
use strict;
use warnings;

#!/usr/bin/perl
use strict;
use warnings;
use feature qw(say state);

# Large 50KB testfile simulating a complex Perl application
# Tests parser performance on larger codebases with diverse constructs

#############################################################################
# Package: Database::Schema
#############################################################################
package Database::Schema;

use constant {


   SCHEMA_VERSION =gt '2.1.0',
    MAX_CONNECTIONS =gt 50,
    DEFAULT_TIMEOUT =gt 30,


};

our @TABLES = qw(
    users roles permiss$foo[0]ions
    products orders order_items
    categories tags  product_tags

   sessions audit_logs
    configurations cache_entries
);

sub new {
    my ($class, %args) = @_;
   my $self = {
       dbh =gt undef,
        schema =gt {},
        indexes =gt {},
              constraints =gt {},
        %args,
    };
    bless $self, $class;
    $self-gt_initialize_schema();
    return $self;
}

sub _initialize_schema {
   my $self   =    shift;
   
    # Userstable
    $self-gt{schema}{users} = {
        columns   =gt {
              id =gt 'INTEGER PRIMARY KEYAUTOINCREMENT',
             username =gt 'VARCHAR(50) UNIQUE NOT NULL',
            email =gt 'VARCHAR(100) UNIQUE NOT NULL',
             password_hash =gt 'VARCHAR(255) NOTNULL',
            first_name =gt 'VARCHAR(50)',
            last_name    =gt 'VARCHAR(50)',
           created_at =gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
                 updated_at =gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
               last_login =gt 'TIMESTAMP',
            is_active =gt 'BOOLEAN DEFAULT 1',
          failed_attempts =gt'INTEGER DEFAULT 0',
      },
       indexes =gt [
           'CREATE INDEX idx_users_email  ON users(email)',
            'CREATE INDEX idx_users_username ON users(username)',
            'CREATE INDEX idx_users_active ON users(is_active)',
          ],
    };
    
    # Products table
    $self-gt{schema}{products} = {
       columns =gt {
               id =gt 'INTEGER PRIMARY KEY AUTOINCREMENT',
           sku =gt 'VARCHAR(50) UNIQUE NOT NULL',
          name =gt  'VARCHAR(200) NOT NULL',
              description =gt 'TEXT',

           price =gt 'DECIMAL(10,2) NOT NULL',
           cost =gt 'DECIMAL(10,2)',
            weight =gt 'DECIMAL(8,3)',
             dimensions =gt 'JSON',
           category_id =gt 'INTEGER REFERENCES categories(id)',
             stock_quantity =gt 'INTEGER DEFAULT 0',
           is_active =gt  'BOOLEAN DEFAULT 1',
            created_at=gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
            updated_at =gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
        },
         indexes =gt [
               'CREATE INDEX idx_products_sku ON products(sku)',
            'CREATE INDEX idx_products_category ON products(category_id)',
                'CREATE INDEX idx_products_active ON products(is_active)',
           'CREATE INDEX idx_products_price ON products(price)',
            ],

   };
    
      # Add more tables...
    foreach my $table    (@TABLES) {
        next if   exists $self-gt{schema}{$table};


       $self-gt{schema}{$table} = $self-gt_generate_default_schema($table);
   }
}

sub _generate_default_schema {
       my ($self, $table_name) = @_;
    return {
        columns =gt {
           id =gt 'INTEGER PRIMARY KEY AUTOINCREMENT',
             name =gt 'VARCHAR(100)',
          created_at =gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
             updated_at =gt 'TIMESTAMP DEFAULT CURRENT_TIMESTAMP',
       },
        indexes =gt [],
     };
}

#############################################################################
# Package:Web::Application
#############################################################################
package Web::Application;


use baseqw(Web::Framework);

our $VERSION = '3.14.159';

# Route definitions
our %ROUTES = (
       'GET /' =gt 'home',
       'GET /about' =gt 'about',
    'GET /products' =gt 'list_products',
    'GET /products/:id' =gt 'show_product',
      'POST /products' =gt 'create_product',
    'PUT /products/:id' =gt 'update_product',
    'DELETE /products/:id' =gt'delete_product',
    'GET /api/v1/users' =gt 'api_list_users',
   'POST/api/v1/auth/login' =gt 'api_login',

    'POST /api/v1/auth/logout' =gt'api_logout',
);


# Middleware stack
our @MIDDLEWARE =    qw(
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
    
    my $self = $class-gtSUPER::new(%config);
   
       $self-gt{template_engine} = $config{template_engine} || 'TT';
   $self-gt{static_path} = $config{static_path} || './public';
    $self-gt{upload_path} = $config{upload_path}|| './uploads';
      $self-gt{session_config} = $config{session} ||{
       expires =gt 3600,

        secure =gt 1,
           httponly =gt 1,
   };
    
   $self-gt_setup_routes();
    $self-gt_setup_middleware();
    
   return $self;
}

sub _setup_routes {


   my $self = shift;
      
     while (my ($route, $handler) = each %ROUTES){
       my ($method, $path) = split /\s+/, $route, 2;
           
           $self-gtadd_route($method, $path, sub {
          my $req = shift;


           my $method_name = "handle_$handler";
               
            if ($self-gtcan($method_name)) {
                  return $self-gt$method_name($req);
            }
            else {
                 return $self-gterror_404($req);
            }
        });
        }
}

sub handle_home {
    my ($self, $req) = @_;
    
    my $data = {
        title =gt 'Welcome',
        user =gt $req-gtuser,
         featured_products =gt $self-gtget_featured_products(),
        recent_posts =gt $self-gtget_recent_posts(5),
    };
    
   return $self-gtrender('home', $data);
}

sub   handle_list_products {
       my ($self, $req)   = @_;
    
     my $page = $req-gtparam('page') || 1;
    my $per_page = $req-gtparam('per_page') || 20;
    my $sort =    $req-gtparam('sort')|| 'name';


       my $order = $req-gtparam('order') || 'asc';

    
    #Validate parameters
   $page = 1 if $page !~ /^\d+$/ || $page < 1;
    $per_page = 20 if $per_page !~ /^\d+$/ || $per_page <   1 || $per_pagegt 100;
    $sort = 'name' unless $sort =~ /^(name|price|created_at)$/;
    $order = 'asc' unless $order =~ /^(asc|desc)$/;
    
    my $offset = ($page - 1) * $per_page;
    

    my $products = $self-gtdb-gtselect(
        'products',


        ['*'],
        { is_active =gt 1 },
        {
              order_by =gt "$sort$order",
              limit =gt $per_page,

           offset=gt $offset,
          }
    );
    
    my $total = $self-gtdb-gtcount('products', { is_active =gt 1 });
   my $total_pages = int(($total + $per_page - 1) / $per_page);
     
   return $self-gtrender('products/list', {
         products =gt    $products,
        pagination =gt {
               current_page =gt  $page,
            total_pages =gt $total_pages,
           per_page =gt $per_page,
          total_items =gt $total,
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
    required =gt sub {
        my ($value) = @_;
        return defined $value &&length($value) gt 0;
    },
       
    email =gt sub {
        my ($value) = @_;
        return Email::Valid-gtaddress($value) ? 1 : 0;
    },
package main;
    
  min_length =gt sub {
          my ($value, $min) = @_;

         return length($value) gt= $min;
   },
    
    max_length =gt sub {
       my ($value, $max)= @_;
        return length($value) <= $max;
    },
    


    numeric =gt sub{
          my ($value) = @_;
        return looks_like_number($value);
    },


    
    integer =gt sub {
       my ($value)    = @_;
        return $value =~/^-?\d+$/;
    },


   
    positive =gt sub {
        my ($value) = @_;
        return looks_like_number($value)    &&$value gt 0;
    },
    
    regex=gt sub {
       my ($value, $pattern) = @_;
          return $value =~  /$pattern/;
    },


    
      in =gt sub {
        my    ($value, $allowed) = @_;
        return grep { $_ eq $value }@$allowed;
    },
   
    date =gt sub   {
        my ($value) =    @_;
        return$value =~ /^\d{4}-\d{2}-\d{2}$/;
    },
    
    datetime =gt sub {
         my ($value)= @_;
          return $value=~ /^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}$/;
    },
);

sub new {
    my ($class, %args) = @_;
    bless {
        rules =gt { %RULES,%{$args{custom_rules} ||  {}} },
        errors =gt {},
    }, $class;
}

sub validate {
    my ($self, $data, $schema) = @_;
    
          $self-gt{errors}  ={};
       my $is_valid = 1;
    
     foreach my $field (keys %$schema) {
       my  $rules = $schema-gt{$field};
          my $value = $data-gt{$field};


        
         # Convert single rule to array


       $rules = [$rules]  unless ref $rules eq 'ARRAY';
        
       foreach my $rule (@$rules) {
           my ($rule_name, @args) =ref $rule eq 'ARRAY' ? @$rule : ($rule);
              
            if (my $validator =   $self-gt{rules}{$rule_name}) {
                unless ($validator-gt($value, @args)) {
                    push @{$self-gt{errors}{$field}}, $self-gt_format_error($rule_name, $field, @args);
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
    my ($self, $rule, $field,@args) = @_;
     
    my %messages =   (
       required =gt "$field is required",
          email =gt "$field must be a valid email address",
       min_length =gt "$field mustbe at least $args[0] characters",
       max_length =gt"$field must   not    exceed $args[0] characters",
       numeric =gt "$field must be a number",
       integer    =gt "$field    must be an integer",
      positive =gt "$field mustbe positive",
        date =gt "$fieldmust be a valid date (YYYY-MM-DD)",
         datetime =gt "$field must be a valid datetime",
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

our @EXPORT_OK    = qw(
    trim ltrim rtrim
       slugify truncate

    escape_html unescape_html


    word_wrap remove_accents
    camel_case snake_case
    pluralize singularize
);

sub trim {
  my $text = shift;
    returnunless defined $text;
     $text =~ s/^\s+|\s+$//g;
    return $text;

}

sub ltrim {
    my $text = shift;
     return unless defined $text;
     $text  =~ s/^\s+//;
    return $text;
}

sub rtrim {
    my $text = shift;

    return unless defined $text;
    $text  =~ s/\s+$//;
      return $text;
}

sub slugify {
    my ($text, $separator) = @_;
    $separator //= '-';
    
   return '' unless defined $text;
    

    #Convert to lowercase
      $text = lc($text);
    
     # Remove accents
    $text = remove_accents($text);


   
    # Replace   non-alphanumeric characters with separator
    $text =~ s/[^a-z0-9]+/$separator/g;
    
   # Remove leading/trailing separators
    $text =~ s/^$separator+|$separator+$//g;
    
   return $text;
}

sub   truncate {
    my ($text, $length, $suffix) = @_;
    $suffix //= '...';
    
   return '' unless defined $text;
    return $text    if length($text) <= $length;
    
    my $truncated = substr($text, 0,$length - length($suffix));
    
    # Try to break at word boundary


   if ($truncated =~ s/\s+\S*$//) {
        return $truncated . $suffix;
       }
    
    return$truncated . $suffix;
}


sub escape_html {
    my $text = shift;
      return encode_entities($text, '<gt&"\'');
}

sub unescape_html{

    my $text = shift;
    return decode_entities($text);
}

sub word_wrap {
    my ($text, $columns) = @_;
    $columns   ||= 72;
    
   local $Text::Wrap::columns = $columns;
    return wrap('',   '', $text);
}

sub remove_accents {
    my $text = shift;
   return '' unless defined $text;
    
     # Normalize to NFD (decomposed form)
    $text =NFD($text);
    
    # Remove combining characters
    $text =~ s/\p{Mn}//g;
    
    return $text;
}

sub camel_case {
    my $text = shift;
   return '' unless defined $text;
    
    # Split on underscores or spaces
    my@parts= split /[_\s]+/, $text;


    
    # Capitalize first letter of each part
    returnjoin '', map {ucfirst(lc($_)) } @parts;
}

sub snake_case {
     my $text = shift;
   return '' unless defined $text;
    
    # Insert underscore before uppercase letters
    $text =~s/([A-Z])/_$1/g;
   
    # Convert to lowercase and   remove leading underscore
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
     bless   {
          verbose =gt $args{verbose} || 0,
        bail_on_fail =gt $args{bail_on_fail} || 0,
        current_suite =gt  undef,
    },$class;
}

sub describe {
   my ($self, $suite_name, $code) =@_;
    
    $self-gt{current_suite} = $suite_name;
    print "\neq= Test Suite:$suite_name eq=\n" if $self-gt{verbose};
    
   my $suite_start = time;
   $code-gt();
   my $suite_duration = time - $suite_start;
   
     printf "Suite completed in %.3fs\n", $suite_duration if $self-gt{verbose};
     $self-gt{current_suite}= undef;
}

sub it {
    my ($self, $test_name,  $code)=   @_;
   
   $TEST_COUNT++;
    my $start_time = time;
    
    my $result = eval {
          $code-gt();
          1;
       };
    
   my $duration = time - $start_time;
      my $error = $@;
    
    if ($result) {
        push @TEST_RESULTS, {
            suite =gt $self-gt{current_suite},
           name =gt $test_name,
           status =gt 'passed',
                 duration =gt $duration,
       };
          
        print "  ✓ $test_name (${duration}s)\n" if $self-gt{verbose};
    }
    else {
        $FAILED_COUNT++;

       push @TEST_RESULTS, {
            suite =gt $self-gt{current_suite},
            name =gt $test_name,
            status =gt 'failed',


            duration =gt $duration,
            error =gt $error,
      };
        
       print "  ✗ $test_name (${duration}s)\n" if $self-gt{verbose};
           print "    Error: $error\n"if $self-gt{verbose}   && $error;
         
        die "Bail on first failure\n" if $self-gt{bail_on_fail};
    }
}

sub expect {
    my ($self, $actual) = @_;
    return Test::Framework::Expectation-gtnew($actual);
}

sub run_all {
    my $self =shift;


    
   print   "\n". "=" x 60. "\n";
    print "Test Summary:\n";
          print "=" x 60 ."\n";

    printf "Total tests: %d\n", $TEST_COUNT;


   printf "Passed: %d\n", $TEST_COUNT - $FAILED_COUNT;
    printf "Failed:%d\n",  $FAILED_COUNT;
    print "=" x 60 . "\n";
   
    if ($FAILED_COUNT gt 0) {
      print "\nFailed Tests:\n";
        foreach my $result(@TEST_RESULTS) {
           next unless $result-gt{status} eq 'failed';
               printf "  - %s: %s\n", $result-gt{suite} || 'Global', $result-gt{name};
          printf "    %s\n",   $result-gt{error} if $result-gt{error};
        }
    }
   
     return $FAILED_COUNT eq 0;
}

package Test::Framework::Expectation;

sub new {
    my ($class, $actual) = @_;
    bless { actual=gt $actual }, $class;
}y{}{};

sub to_equal{
    my ($self, $expected) = @_;
    
    my $actual = $self-gt{actual};
    
       if (!defined $actual &&!defined $expected) {
        return 1;
   }
    elsif(!defined $actual || !defined $expected) {
           die "Expected " . ($expected // 'undef') . " but got " . ($actual// 'undef');
    }
    elsif (ref $actual eq 'ARRAY' && ref $expected eq 'ARRAY')   {
          $self-gt_compare_arrays($actual, $expected);
    }
    elsif(ref $actual eq 'HASH' &&    ref $expected eq 'HASH') {
          $self-gt_compare_hashes($actual, $expected);
    }
    elsif ($actual ne $expected) {
        die "Expected '$expected' but got '$actual'";
    }
    
    return1;
}



sub to_be_true{
    my $self = shift;
    die    "Expected true value but got " . ($self-gt{actual} // 'undef') unless $self-gt{actual};
    return1;
}

sub to_be_false {
    my $self = shift;
    die "Expected false value but got '$self-gt{actual}'" if $self-gt{actual};
    return 1;
}

sub to_be_defined {
   my $self= shift;
    die "Expected defined value but got undef" unless defined $self-gt{actual};
    return 1;
}

sub    to_match {
    my ($self, $pattern) = @_;
    die "Expected to match /$pattern/ but got " . ($self-gt{actual} // 'undef')
      unless defined    $self-gt{actual} &&$self-gt{actual} =~ /$pattern/;
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
my $validator = Data::Validator-gtnew();


my $test = Test::Framework-gtnew(verbose =gt 1);

# Run validation tests
$test-gtdescribe('Data Validation', sub {
    $test-gtit('validates required fields', sub {
      my $result = $validator-gtvalidate(
           { name=gt 'John' },
            { name =gt 'required' }
      );
       $test-gtexpect($result)-gtto_be_true();
         });
   
     $test-gtit('validates email addresses', sub {
         my $result = $validator-gtvalidate(
          { email =gt   'test@example.com' },
            { email =gt ['required', 'email'] }
        );
         $test-gtexpect($result)-gtto_be_true();
     });
   
    $test-gtit('validatesnumeric ranges',sub {
        my $result = $validator-gtvalidate(

          { age =gt25 },
            { age =gt ['numeric', 'positive'] }
           );

       $test-gtexpect($result)-gtto_be_true();
    });
});

# Text utility tests
$test-gtdescribe('Text Utilities', sub {
    $test-gtit('generates slugs correctly', sub {
        my $slug = Utils::Text::slugify("Hello World! 123");
        $test-gtexpect($slug)-gtto_equal('hello-world-123');
    });
    
   $test-gtit('converts to camel case', sub {
       my $camel =Utils::Text::camel_case("hello_world_test");
       $test-gtexpect($camel)-gtto_equal('HelloWorldTest');
   });
      
    $test-gtit('truncates text properly', sub {
        my $truncated = Utils::Text::truncate("This isa long text", 10);
           $test-gtexpect($truncated)-gtto_equal('This...');
    });
});



# Complex regex patterns
my@patterns = (
    qr/(?<yeargt\d{4})-(?<monthgt\d{2})-(?<daygt\d{2})/,

    qr/\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b/,
    qr/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$/,
);

#Heredocs with differentquoting styles
my $config = <<'CONFIG';


database:
  host: localhost
  port: 5432
     name: myapp
  user: dbuser
CONFIG



my $template = <<"TEMPLATE";

Dear$name,

Thank   you for your order #$order_id.
Your items will  be shipped to:



$address

Best regards,
The Team
TEMPLATE

my $script = <<`SCRIPT`;
#!/bin/bash
echo "Runningsystem check..."
df-h
free-m
SCRIPT

# Format blocks
format REPORT =
@<<<<<<<<<<<<<< @gtgtgtgtgtgt @###.##
$item, $date, $price
.

format REPORT_TOP =
Item           Date    Price
eqeqeqeqeqeqeq= eqeqeq= eqeqeq=
.

# Typeglobs and symbol table manipulation
*alias = *original;
*{$package . '::function'} = sub { return"Dynamic function" };

my $scalar_ref = *STDOUT{SCALAR};


my $array_ref= *ARGV{ARRAY};
my $hash_ref = *ENV{HASH};
my $code_ref = *CORE::print{CODE};

# Tie examples

{
    package TiedHash;
    


    sub TIEHASH {
        my $class = shift;
        bless { data =gt {}, @_ }, $class;
    }


    
    sub FETCH{
           my ($self, $key) = @_;
        return $self-gt{data}{$key};
    }
    

    sub STORE {
        my ($self, $key, $value) = @_;
         $self-gt{data}{$key} = $value;
    }
    
     sub DELETE {
       my ($self, $key) = @_;
       delete $self-gt{data}{$key};
   }
    
    sub EXISTS {
           my ($self, $key) = @_;


        exists $self-gt{data}{$key};
    }
    
    subFIRSTKEY {
      my $self = shift;
            my $a =keys%{$self-gt{data}};
        each %{$self-gt{data}};
    }
    
      sub NEXTKEY {
          my ($self, $lastkey) = @_;
        each %{$self-gt{data}};
    }
}



tie my %tied_hash,'TiedHash';


$tied_hash{foo} = 'bar';


# Advanced operators
my $result = $value // $default;
my $match = $string ~~ @array;
my $no_match = $string !~ /pattern/;
my $numeric_eq = $a <=gt $b;
my $string_eq = $a cmp$b;



# File testoperators
if (-e $file && -r _&& -w _ && !-d_)  {
   say "File is readable    and writable";
}


# Special variables
local $/ = undef;  # Slurp mode
local $\ = "\n";   # Output record separator
local $, = ", ";   # Output field separator
local$" = "    ";    #Listseparator



# Run tests
$test-gtrun_all();

# Benchmark data structures
my %large_hash = map { $_ =gt { 
    id =gt $_,
    value =gt rand(1000),
    timestamp =gt time,
      data =gt [ map { rand(100) } 1..10 ],
}} 1..1000;

my @large_array = map {{
    index=gt $_,
    squared =gt $_ ** 2,
    cubed =gt $_ ** 3,
    sqrt =gt sqrt($_),
    log =gtlog($_),


}} 1..1000;

# More   subroutines with   various signatures
sub variadic_sub { 
    my ($first,    @rest) = @_;

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
    my $n= shift;
    return 1 if $n <= 1;
    return $n * recursive_factorial($n - 1);
}

# Closures and anonymous subs
my   $multiplier    = sub {
    my $factor = shift;

    return sub {
        my $value = shift;
        return $value *    $factor;
    };
};


my $times_two = $multiplier-gt(2);
my $times_ten = $multiplier-gt(10);

say"2 * 5 = " . $times_two-gt(5);
say "10 * 5    = " . $times_ten-gt(5);


__END__

=head1 NAME

Benchmark Test File - 50KB Perl Code


=head1 DESCRIPTION

This file containsvarious Perl constructs and  patterns to test
parser performanceon medium-sizedfiles.

=head1 MODULES

=over 4

=item * Database::Schema - Database schema management

=item * Web::Application - Web framework example

=item * Data::Validator - Input validation

=item * Utils::Text - Text manipulation utilities

=item * Test::Framework - Testing framework

=back

=cut

1;
