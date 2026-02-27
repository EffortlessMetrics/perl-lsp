#!/usr/bin/env perl
# Modern OO Framework Patterns - Moose, Moo, Class::Std
# Production-quality test file representing actual enterprise Perl code

use strict;
use warnings;
use v5.20;

# Moose Object System Patterns
{
    package MooseExample::User;
    
    use Moose;
    use Moose::Util::TypeConstraints;
    use MooseX::StrictConstructor;
    use MooseX::Types::Common::String qw(NonEmptySimpleStr);
    use namespace::autoclean;
    
    # Type definitions
    subtype 'Email',
        as 'NonEmptySimpleStr',
        where { $_ =~ /\A[^@\s]+@[^@\s]+\z/ },
        message { "$_ is not a valid email address" };
    
    subtype 'PositiveInt',
        as 'Int',
        where { $_ > 0 },
        message { "$_ is not a positive integer" };
    
    # Attributes with type constraints and traits
    has 'id' => (
        is       => 'ro',
        isa      => 'PositiveInt',
        required => 1,
        traits   => ['Number'],
        handles  => {
            is_new_user => 'is_positive',
        },
    );
    
    has 'name' => (
        is       => 'rw',
        isa      => 'NonEmptySimpleStr',
        required => 1,
        trigger  => sub {
            my ($self, $new_name, $old_name) = @_;
            warn "Name changed from $old_name to $new_name" if $old_name;
        },
    );
    
    has 'email' => (
        is       => 'rw',
        isa      => 'Email',
        required => 1,
        writer   => 'set_email',
        clearer  => 'clear_email',
        predicate => 'has_email',
    );
    
    has 'roles' => (
        is      => 'rw',
        isa     => 'ArrayRef[Str]',
        default => sub { [] },
        traits  => ['Array'],
        handles => {
            add_role     => 'push',
            has_role     => 'contains',
            role_count   => 'count',
            all_roles    => 'elements',
        },
    );
    
    has 'metadata' => (
        is      => 'ro',
        isa     => 'HashRef',
        default => sub { {} },
        traits  => ['Hash'],
        handles => {
            get_metadata => 'get',
            set_metadata => 'set',
            has_metadata => 'exists',
        },
    );
    
    has 'created_at' => (
        is      => 'ro',
        isa     => 'DateTime',
        default => sub { DateTime->now },
        handles => {
            format_created => 'strftime',
        },
    );
    
    # Method modifiers
    before 'set_email' => sub {
        my ($self, $email) = @_;
        warn "Email will be set to: $email";
    };
    
    after 'set_email' => sub {
        my ($self, $email) = @_;
        warn "Email has been set to: $email";
    };
    
    around 'set_email' => sub {
        my $orig = shift;
        my $self = shift;
        my $email = shift;
        
        $email = lc($email);
        return $self->$orig($email);
    };
    
    # Methods
    sub display_info {
        my $self = shift;
        
        my $info = sprintf "User: %s (ID: %d)\n", $self->name, $self->id;
        $info .= sprintf "Email: %s\n", $self->email if $self->has_email;
        $info .= sprintf "Roles: %s\n", join(', ', $self->all_roles) if $self->role_count;
        $info .= sprintf "Created: %s\n", $self->format_created('%Y-%m-%d %H:%M:%S');
        
        return $info;
    }
    
    sub has_admin_role {
        my $self = shift;
        return $self->has_role('admin') || $self->has_role('superuser');
    }
    
    # Validation method
    sub validate {
        my $self = shift;
        my @errors;
        
        push @errors, "Name cannot be empty" unless $self->name;
        push @errors, "Email is required" unless $self->has_email;
        
        return @errors;
    }
    
    __PACKAGE__->meta->make_immutable;
}

# Moo Object System Patterns
{
    package MooExample::Product;
    
    use Moo;
    use MooX::Types::MooseLike::Base qw(Str Int Bool ArrayRef HashRef);
    use namespace::clean;
    
    # Attributes with type constraints
    has 'sku' => (
        is       => 'ro',
        isa      => Str,
        required => 1,
    );
    
    has 'name' => (
        is       => 'rw',
        isa      => Str,
        required => 1,
    );
    
    has 'price' => (
        is  => 'rw',
        isa => sub { die "Price must be numeric" unless $_[0] =~ /^\d+(\.\d{2})?$/ },
    );
    
    has 'in_stock' => (
        is      => 'rw',
        isa     => Bool,
        default => sub { 1 },
    );
    
    has 'categories' => (
        is      => 'rw',
        isa     => ArrayRef,
        default => sub { [] },
    );
    
    has 'attributes' => (
        is      => 'rw',
        isa     => HashRef,
        default => sub { {} },
    );
    
    # Method with validation
    sub set_price {
        my ($self, $price) = @_;
        die "Invalid price format" unless $price =~ /^\d+(\.\d{2})?$/;
        $self->price($price);
    }
    
    sub add_category {
        my ($self, $category) = @_;
        push @{$self->categories}, $category;
    }
    
    sub has_category {
        my ($self, $category) = @_;
        return grep { $_ eq $category } @{$self->categories};
    }
    
    sub calculate_discount_price {
        my ($self, $discount_percent) = @_;
        return unless $self->price && $discount_percent && $discount_percent > 0 && $discount_percent <= 100;
        
        my $discount = $self->price * ($discount_percent / 100);
        return sprintf "%.2f", $self->price - $discount;
    }
}

# Class::Std Patterns (Classic Perl OO)
{
    package ClassStdExample::Document;
    
    use Class::Std;
    use Carp qw(croak carp);
    use List::Util qw(first);
    
    # Attributes
    my %title_of :ATTR(:get<title> :set<title>);
    my %content_of :ATTR(:get<content> :set<content>);
    my %author_of :ATTR(:init_arg<author> :get<author>);
    my %created_at_of :ATTR(:set<created_at> :get<created_at>);
    my %tags_of :ATTR(:default<[]> :get<tags>);
    my %metadata_of :ATTR(:default<{}> :get<metadata>);
    
    # Constructor
    sub BUILD {
        my ($self, $ident, $arg_ref) = @_;
        
        $created_at_of{$ident} = time();
        
        # Validate required fields
        croak "Title is required" unless $title_of{$ident};
        croak "Content is required" unless $content_of{$ident};
        croak "Author is required" unless $author_of{$ident};
    }
    
    # Methods
    sub add_tag {
        my ($self, $tag) = @_;
        my $ident = ident $self;
        
        # Avoid duplicates
        return if first { $_ eq $tag } @{$tags_of{$ident}};
        push @{$tags_of{$ident}}, $tag;
    }
    
    sub remove_tag {
        my ($self, $tag) = @_;
        my $ident = ident $self;
        
        @{$tags_of{$ident}} = grep { $_ ne $tag } @{$tags_of{$ident}};
    }
    
    sub has_tag {
        my ($self, $tag) = @_;
        my $ident = ident $self;
        
        return first { $_ eq $tag } @{$tags_of{$ident}};
    }
    
    sub set_metadata {
        my ($self, $key, $value) = @_;
        my $ident = ident $self;
        
        $metadata_of{$ident}{$key} = $value;
    }
    
    sub get_metadata {
        my ($self, $key) = @_;
        my $ident = ident $self;
        
        return $metadata_of{$ident}{$key};
    }
    
    sub word_count {
        my $self = shift;
        my $ident = ident $self;
        
        my $content = $content_of{$ident};
        return scalar split /\s+/, $content;
    }
    
    sub summary {
        my $self = shift;
        my $ident = ident $self;
        
        my $content = $content_of{$ident};
        my $summary = substr($content, 0, 100);
        $summary .= '...' if length($content) > 100;
        
        return $summary;
    }
    
    # Destructor
    sub DEMOLISH {
        my ($self, $ident) = @_;
        # Cleanup if needed
        carp "Document '$title_of{$ident}' being destroyed";
    }
}

# Role Composition Patterns
{
    package RoleExample::Loggable;
    
    use Moo::Role;
    use namespace::clean;
    
    has 'log_level' => (
        is      => 'rw',
        default => sub { 'info' },
    );
    
    requires 'log_message';
    
    sub debug { my $self = shift; $self->log_message('debug', @_) if $self->should_log('debug'); }
    sub info  { my $self = shift; $self->log_message('info',  @_) if $self->should_log('info'); }
    sub warn  { my $self = shift; $self->log_message('warn',  @_) if $self->should_log('warn'); }
    sub error { my $self = shift; $self->log_message('error', @_) if $self->should_log('error'); }
    
    sub should_log {
        my ($self, $level) = @_;
        my %levels = (debug => 0, info => 1, warn => 2, error => 3);
        return $levels{$level} >= $levels{$self->log_level};
    }
}

{
    package RoleExample::Configurable;
    
    use Moo::Role;
    use namespace::clean;
    
    has 'config' => (
        is      => 'rw',
        default => sub { {} },
    );
    
    requires 'load_config';
    
    sub get_config {
        my ($self, $key, $default) = @_;
        return $self->config->{$key} // $default;
    }
    
    sub set_config {
        my ($self, $key, $value) = @_;
        $self->config->{$key} = $value;
    }
}

{
    package RoleExample::Service;
    
    use Moo;
    use namespace::clean;
    
    with 'RoleExample::Loggable', 'RoleExample::Configurable';
    
    has 'name' => (
        is       => 'ro',
        required => 1,
    );
    
    sub log_message {
        my ($self, $level, $message) = @_;
        my $timestamp = scalar localtime;
        warn "[$timestamp] [$level] [$self->{name}] $message\n";
    }
    
    sub load_config {
        my $self = shift;
        # In real implementation, would load from file/database
        $self->config({ debug => 1, timeout => 30 });
    }
    
    sub initialize {
        my $self = shift;
        $self->load_config;
        $self->info("Service '$self->{name}' initialized");
    }
}

# Method Attributes and Advanced Patterns
{
    package AttributeExample::Controller;
    
    use Moose;
    use namespace::autoclean;
    
    # Attribute-like method markers
    sub before_action :MethodAttr('before') {
        my ($self, $action) = @_;
        $self->info("Before action: $action");
    }
    
    sub after_action :MethodAttr('after') {
        my ($self, $action) = @_;
        $self->info("After action: $action");
    }
    
    sub around_action :MethodAttr('around') {
        my ($orig, $self, $action) = @_;
        $self->before_action($action);
        my $result = $self->$orig($action);
        $self->after_action($action);
        return $result;
    }
    
    # Cached method simulation
    sub expensive_computation :MethodAttr('cache') {
        my ($self, $input) = @_;
        
        # Simulate expensive operation
        sleep(1);
        return $input * 2;
    }
    
    # Transaction simulation
    sub transactional_operation :MethodAttr('transaction') {
        my $self = shift;
        
        eval {
            # Database operations would go here
            $self->info("Performing transactional operation");
            die "Transaction failed" if rand() < 0.3; # Simulate occasional failure
            $self->info("Transaction completed successfully");
        };
        
        if ($@) {
            $self->error("Transaction rolled back: $@");
            return;
        }
        
        return 1;
    }
}

# Complex Inheritance Patterns
{
    package InheritanceExample::BaseModel;
    
    use Moose;
    use namespace::autoclean;
    
    has 'id' => (
        is       => 'ro',
        isa      => 'Int',
        required => 1,
    );
    
    has 'created_at' => (
        is      => 'ro',
        isa     => 'DateTime',
        default => sub { DateTime->now },
    );
    
    has 'updated_at' => (
        is      => 'rw',
        isa     => 'DateTime',
        default => sub { DateTime->now },
    );
    
    sub save {
        my $self = shift;
        $self->updated_at(DateTime->now);
        # Database save logic would go here
    }
    
    sub delete {
        my $self = shift;
        # Database delete logic would go here
    }
    
    sub to_hash {
        my $self = shift;
        return {
            id         => $self->id,
            created_at => $self->created_at->iso8601,
            updated_at => $self->updated_at->iso8601,
        };
    }
}

{
    package InheritanceExample::User;
    
    use Moose;
    use namespace::autoclean;
    
    extends 'InheritanceExample::BaseModel';
    
    has 'username' => (
        is       => 'rw',
        isa      => 'Str',
        required => 1,
    );
    
    has 'email' => (
        is       => 'rw',
        isa      => 'Str',
        required => 1,
    );
    
    has 'password_hash' => (
        is       => 'rw',
        isa      => 'Str',
        required => 1,
    );
    
    override 'to_hash' => sub {
        my $self = shift;
        my $hash = super();
        
        $hash->{username} = $self->username;
        $hash->{email} = $self->email;
        # Don't include password_hash in output
        
        return $hash;
    };
    
    sub authenticate {
        my ($self, $password) = @_;
        # Password verification logic would go here
        return 1; # Simplified
    }
    
    sub change_password {
        my ($self, $new_password) = @_;
        # Password hashing and update logic would go here
        $self->password_hash('hashed_' . $new_password);
        $self->save;
    }
}

# Usage examples (would be in separate files in real code)
package main;

use Data::Dumper;

print "=== Modern OO Framework Patterns Test ===\n";

# Moose example
my $user = MooseExample::User->new(
    id    => 1,
    name  => 'John Doe',
    email => 'john@example.com',
);

$user->add_role('admin');
$user->add_role('user');
$user->set_metadata('department', 'engineering');

print $user->display_info;
print "Has admin role: " . ($user->has_admin_role ? 'Yes' : 'No') . "\n";

# Moo example
my $product = MooExample::Product->new(
    sku   => 'PROD-001',
    name  => 'Example Product',
    price => '29.99',
);

$product->add_category('electronics');
$product->add_category('gadgets');

print "\nProduct: " . $product->name . " (\$" . $product->price . ")\n";
print "Categories: " . join(', ', @{$product->categories}) . "\n";
print "Discount price (10%): \$" . $product->calculate_discount_price(10) . "\n";

# Class::Std example
my $doc = ClassStdExample::Document->new(
    title   => 'Test Document',
    content => 'This is a test document with some content for testing purposes.',
    author  => 'Test Author',
);

$doc->add_tag('test');
$doc->add_tag('document');
$doc->set_metadata('priority', 'high');

print "\nDocument: " . $doc->get_title . "\n";
print "Author: " . $doc->get_author . "\n";
print "Word count: " . $doc->word_count . "\n";
print "Summary: " . $doc->summary . "\n";

# Role composition example
my $service = RoleExample::Service->new(name => 'TestService');
$service->initialize;
$service->error("This is an error message");

print "\n=== OO Framework Patterns Test Complete ===\n";