#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Benchmarks for real-world CPAN module patterns.
//!
//! These benchmarks exercise the parser against representative Perl code patterns
//! commonly found in CPAN distributions, ensuring performance on realistic inputs.

use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser::Parser;
use std::hint::black_box;

// ---------------------------------------------------------------------------
// 1. Moose-style OO class definition
// ---------------------------------------------------------------------------

const MOOSE_CLASS: &str = r#"
package My::App::User;
use Moose;
use Moose::Util::TypeConstraints;
use namespace::autoclean;

extends 'My::App::Base';
with 'My::App::Role::Serializable', 'My::App::Role::Cacheable';

has 'id' => (
    is       => 'ro',
    isa      => 'Int',
    required => 1,
);

has 'username' => (
    is       => 'rw',
    isa      => 'Str',
    required => 1,
    trigger  => sub {
        my ($self, $new_val) = @_;
        $self->_clear_display_name;
    },
);

has 'email' => (
    is        => 'rw',
    isa       => 'Str',
    predicate => 'has_email',
    clearer   => 'clear_email',
);

has 'roles' => (
    is      => 'rw',
    isa     => 'ArrayRef[Str]',
    default => sub { [] },
    traits  => ['Array'],
    handles => {
        add_role   => 'push',
        all_roles  => 'elements',
        role_count => 'count',
        has_roles  => 'count',
    },
);

has 'metadata' => (
    is      => 'rw',
    isa     => 'HashRef',
    default => sub { {} },
    traits  => ['Hash'],
    handles => {
        set_meta    => 'set',
        get_meta    => 'get',
        has_meta    => 'exists',
        meta_keys   => 'keys',
        delete_meta => 'delete',
    },
);

has 'created_at' => (
    is      => 'ro',
    isa     => 'Int',
    default => sub { time() },
);

has '_display_name' => (
    is        => 'rw',
    isa       => 'Str',
    lazy      => 1,
    builder   => '_build_display_name',
    clearer   => '_clear_display_name',
);

sub _build_display_name {
    my $self = shift;
    return $self->username;
}

sub BUILD {
    my ($self, $args) = @_;
    if ($args->{roles}) {
        foreach my $role (@{$args->{roles}}) {
            $self->add_role($role);
        }
    }
    return;
}

around 'username' => sub {
    my ($orig, $self, @args) = @_;
    if (@args) {
        my $new_name = $args[0];
        $new_name =~ s/^\s+|\s+$//g;
        return $self->$orig($new_name);
    }
    return $self->$orig;
};

before 'clear_email' => sub {
    my $self = shift;
    warn "Clearing email for user " . $self->username;
};

after 'add_role' => sub {
    my ($self, $role) = @_;
    $self->set_meta("last_role_added", $role);
};

sub is_admin {
    my $self = shift;
    return grep { $_ eq 'admin' } $self->all_roles;
}

sub to_hashref {
    my $self = shift;
    return {
        id       => $self->id,
        username => $self->username,
        email    => $self->email,
        roles    => [$self->all_roles],
        metadata => {$self->metadata},
    };
}

sub validate {
    my $self = shift;
    my @errors;
    if (length($self->username) < 3) {
        push @errors, "Username must be at least 3 characters";
    }
    if ($self->has_email && $self->email !~ /\@/) {
        push @errors, "Invalid email format";
    }
    return @errors;
}

__PACKAGE__->meta->make_immutable;
1;
"#;

// ---------------------------------------------------------------------------
// 2. Module with many subroutines (25 subs)
// ---------------------------------------------------------------------------

const MANY_SUBS_MODULE: &str = r#"
package My::Utils;
use strict;
use warnings;
use Carp qw(croak confess);
use Scalar::Util qw(blessed reftype looks_like_number);
use List::Util qw(reduce any all none first);

sub trim {
    my $str = shift;
    $str =~ s/^\s+//;
    $str =~ s/\s+$//;
    return $str;
}

sub ltrim {
    my $str = shift;
    $str =~ s/^\s+//;
    return $str;
}

sub rtrim {
    my $str = shift;
    $str =~ s/\s+$//;
    return $str;
}

sub ucfirst_words {
    my $str = shift;
    $str =~ s/\b(\w)/uc($1)/ge;
    return $str;
}

sub camelize {
    my $str = shift;
    $str =~ s/_(.)/uc($1)/ge;
    return ucfirst($str);
}

sub decamelize {
    my $str = shift;
    $str =~ s/([A-Z])/'_' . lc($1)/ge;
    $str =~ s/^_//;
    return $str;
}

sub slugify {
    my $str = shift;
    $str = lc($str);
    $str =~ s/[^a-z0-9]+/-/g;
    $str =~ s/^-|-$//g;
    return $str;
}

sub truncate_str {
    my ($str, $max_len, $suffix) = @_;
    $suffix //= '...';
    return $str if length($str) <= $max_len;
    return substr($str, 0, $max_len - length($suffix)) . $suffix;
}

sub is_empty {
    my $val = shift;
    return 1 if !defined $val;
    return 1 if ref $val eq 'ARRAY' && !@{$val};
    return 1 if ref $val eq 'HASH'  && !%{$val};
    return 1 if !ref $val && $val eq '';
    return 0;
}

sub deep_clone {
    my $ref = shift;
    if (ref $ref eq 'HASH') {
        return { map { $_ => deep_clone($ref->{$_}) } keys %{$ref} };
    }
    elsif (ref $ref eq 'ARRAY') {
        return [ map { deep_clone($_) } @{$ref} ];
    }
    return $ref;
}

sub merge_hashes {
    my @hashes = @_;
    my %result;
    foreach my $h (@hashes) {
        foreach my $key (keys %{$h}) {
            if (ref $h->{$key} eq 'HASH' && ref($result{$key} // '') eq 'HASH') {
                $result{$key} = merge_hashes($result{$key}, $h->{$key});
            }
            else {
                $result{$key} = $h->{$key};
            }
        }
    }
    return \%result;
}

sub flatten_array {
    my @items = @_;
    my @result;
    foreach my $item (@items) {
        if (ref $item eq 'ARRAY') {
            push @result, flatten_array(@{$item});
        }
        else {
            push @result, $item;
        }
    }
    return @result;
}

sub uniq {
    my @items = @_;
    my %seen;
    return grep { !$seen{$_}++ } @items;
}

sub uniq_by {
    my ($code, @items) = @_;
    my %seen;
    return grep { !$seen{$code->($_)}++ } @items;
}

sub chunk_array {
    my ($size, @items) = @_;
    my @chunks;
    while (@items) {
        push @chunks, [splice @items, 0, $size];
    }
    return @chunks;
}

sub zip_arrays {
    my @arrays = @_;
    my $max_len = 0;
    foreach my $arr (@arrays) {
        $max_len = scalar @{$arr} if scalar @{$arr} > $max_len;
    }
    my @result;
    for my $i (0 .. $max_len - 1) {
        push @result, [map { $_->[$i] } @arrays];
    }
    return @result;
}

sub memoize_func {
    my $func = shift;
    my %cache;
    return sub {
        my $key = join("\0", @_);
        $cache{$key} //= $func->(@_);
        return $cache{$key};
    };
}

sub retry {
    my ($code, %opts) = @_;
    my $max    = $opts{max_attempts} // 3;
    my $delay  = $opts{delay}        // 1;
    my $last_err;
    for my $attempt (1 .. $max) {
        my $result = eval { $code->() };
        return $result if !$@;
        $last_err = $@;
        sleep($delay) if $attempt < $max;
    }
    die "Failed after $max attempts: $last_err";
}

sub timed {
    my ($label, $code) = @_;
    my $start = time();
    my $result = $code->();
    my $elapsed = time() - $start;
    warn sprintf("[%s] %.3fs\n", $label, $elapsed);
    return $result;
}

sub compose {
    my @funcs = reverse @_;
    return sub {
        my @args = @_;
        my $result;
        foreach my $fn (@funcs) {
            $result = $fn->(@args);
            @args = ($result);
        }
        return $result;
    };
}

sub parse_key_value {
    my ($str, $sep, $kv_sep) = @_;
    $sep    //= '&';
    $kv_sep //= '=';
    my %result;
    foreach my $pair (split /\Q$sep\E/, $str) {
        my ($key, $val) = split /\Q$kv_sep\E/, $pair, 2;
        $result{$key} = $val // '';
    }
    return \%result;
}

sub wrap_text {
    my ($text, $width) = @_;
    $width //= 80;
    my @lines;
    while (length($text) > $width) {
        my $pos = rindex($text, ' ', $width);
        $pos = $width if $pos <= 0;
        push @lines, substr($text, 0, $pos);
        $text = substr($text, $pos);
        $text =~ s/^\s+//;
    }
    push @lines, $text if length($text);
    return join("\n", @lines);
}

sub assert_type {
    my ($val, $type) = @_;
    if ($type eq 'HASH') {
        croak "Expected HASH ref" unless ref $val eq 'HASH';
    }
    elsif ($type eq 'ARRAY') {
        croak "Expected ARRAY ref" unless ref $val eq 'ARRAY';
    }
    elsif ($type eq 'CODE') {
        croak "Expected CODE ref" unless ref $val eq 'CODE';
    }
    elsif ($type eq 'number') {
        croak "Expected number" unless looks_like_number($val);
    }
    return $val;
}

sub safe_division {
    my ($numerator, $denominator) = @_;
    return 0 if $denominator == 0;
    return $numerator / $denominator;
}

1;
"#;

// ---------------------------------------------------------------------------
// 3. Module with heavy regex usage
// ---------------------------------------------------------------------------

const HEAVY_REGEX_MODULE: &str = r#"
package My::TextProcessor;
use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        case_sensitive => $opts{case_sensitive} // 0,
        multiline      => $opts{multiline}      // 1,
    }, $class;
}

sub extract_emails {
    my ($self, $text) = @_;
    my @emails;
    while ($text =~ m/\b([a-zA-Z0-9._%+-]+\@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b/g) {
        push @emails, $1;
    }
    return @emails;
}

sub extract_urls {
    my ($self, $text) = @_;
    my @urls;
    while ($text =~ m{(https?://[^\s<>"{}|\\^`\[\]]+)}g) {
        push @urls, $1;
    }
    return @urls;
}

sub extract_ip_addresses {
    my ($self, $text) = @_;
    my @ips;
    while ($text =~ m/\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b/g) {
        my $ip = $1;
        my @octets = split /\./, $ip;
        if (all { $_ >= 0 && $_ <= 255 } @octets) {
            push @ips, $ip;
        }
    }
    return @ips;
}

sub parse_log_line {
    my ($self, $line) = @_;
    if ($line =~ m/^(\S+)\s+(\S+)\s+(\S+)\s+\[([^\]]+)\]\s+"(\S+)\s+(\S+)\s+(\S+)"\s+(\d{3})\s+(\d+|-)/) {
        return {
            host    => $1,
            ident   => $2,
            user    => $3,
            time    => $4,
            method  => $5,
            path    => $6,
            proto   => $7,
            status  => $8,
            size    => $9,
        };
    }
    return;
}

sub strip_html {
    my ($self, $html) = @_;
    $html =~ s/<script[^>]*>.*?<\/script>//gis;
    $html =~ s/<style[^>]*>.*?<\/style>//gis;
    $html =~ s/<!--.*?-->//gs;
    $html =~ s/<[^>]+>//g;
    $html =~ s/&nbsp;/ /g;
    $html =~ s/&amp;/&/g;
    $html =~ s/&lt;/</g;
    $html =~ s/&gt;/>/g;
    $html =~ s/&quot;/"/g;
    $html =~ s/&#(\d+);/chr($1)/ge;
    $html =~ s/\s+/ /g;
    $html =~ s/^\s+|\s+$//g;
    return $html;
}

sub validate_date {
    my ($self, $str) = @_;
    if ($str =~ m{^(\d{4})[-/](\d{2})[-/](\d{2})$}) {
        my ($y, $m, $d) = ($1, $2, $3);
        return 0 if $m < 1  || $m > 12;
        return 0 if $d < 1  || $d > 31;
        return 1;
    }
    if ($str =~ m{^(\d{2})[-/](\d{2})[-/](\d{4})$}) {
        return 1;
    }
    return 0;
}

sub parse_csv_line {
    my ($self, $line) = @_;
    my @fields;
    while ($line =~ m{
        (?:^|,)
        (?:
            "([^"]*(?:""[^"]*)*)" |
            ([^,"]*)
        )
    }gx) {
        my $field = defined $1 ? $1 : $2;
        $field =~ s/""/"/g if defined $1;
        push @fields, $field;
    }
    return @fields;
}

sub sanitize_filename {
    my ($self, $name) = @_;
    $name =~ s/[<>:"|?*\\\/]/_/g;
    $name =~ s/\.{2,}/./g;
    $name =~ s/^\s+|\s+$//g;
    $name =~ s/^\.+//;
    $name = substr($name, 0, 255) if length($name) > 255;
    return $name;
}

sub highlight_matches {
    my ($self, $text, $pattern) = @_;
    my $flags = $self->{case_sensitive} ? '' : 'i';
    $text =~ s/($pattern)/\e[1;31m$1\e[0m/g;
    return $text;
}

sub extract_perl_variables {
    my ($self, $code) = @_;
    my @vars;
    while ($code =~ m/([\$\@%]\{?[a-zA-Z_]\w*(?:::[a-zA-Z_]\w*)*\}?)/g) {
        push @vars, $1;
    }
    return @vars;
}

sub normalize_whitespace {
    my ($self, $text) = @_;
    $text =~ s/\r\n/\n/g;
    $text =~ s/\r/\n/g;
    $text =~ s/\t/    /g;
    $text =~ s/ +/ /g;
    $text =~ s/\n{3,}/\n\n/g;
    $text =~ s/^\s+//;
    $text =~ s/\s+$//;
    return $text;
}

sub extract_version {
    my ($self, $text) = @_;
    if ($text =~ m/(?:version|v)\s*[:=]?\s*([\d]+(?:\.[\d]+)*(?:[-_](?:alpha|beta|rc|dev)[\d]*)?)/i) {
        return $1;
    }
    if ($text =~ m/\b(v?\d+\.\d+(?:\.\d+)?)\b/) {
        return $1;
    }
    return;
}

sub tokenize_words {
    my ($self, $text) = @_;
    my @tokens;
    while ($text =~ m/(\w+(?:'\w+)*)/g) {
        push @tokens, lc($1);
    }
    return @tokens;
}

sub replace_template_vars {
    my ($self, $template, $vars) = @_;
    $template =~ s/\{\{(\w+)\}\}/$vars->{$1} \/\/ ''/ge;
    $template =~ s/\$\{(\w+)\}/$vars->{$1} \/\/ ''/ge;
    $template =~ s/\[%\s*(\w+)\s*%\]/$vars->{$1} \/\/ ''/ge;
    return $template;
}

1;
"#;

// ---------------------------------------------------------------------------
// 4. Module with complex nested data structures
// ---------------------------------------------------------------------------

const COMPLEX_DATA_STRUCTURES: &str = r#"
package My::Config;
use strict;
use warnings;
use Carp qw(croak);

my $DEFAULT_CONFIG = {
    database => {
        primary => {
            host     => 'localhost',
            port     => 5432,
            name     => 'myapp_production',
            username => 'dbadmin',
            password => undef,
            options  => {
                pool_size       => 25,
                timeout         => 30,
                retry_count     => 3,
                ssl             => 1,
                sslmode         => 'verify-full',
                connect_timeout => 10,
                statement_cache => {
                    enabled  => 1,
                    max_size => 1000,
                },
            },
        },
        replicas => [
            {
                host   => 'replica1.db.internal',
                port   => 5432,
                weight => 3,
                tags   => ['read', 'reporting'],
            },
            {
                host   => 'replica2.db.internal',
                port   => 5432,
                weight => 2,
                tags   => ['read', 'analytics'],
            },
            {
                host   => 'replica3.db.internal',
                port   => 5433,
                weight => 1,
                tags   => ['read', 'backup'],
            },
        ],
    },
    cache => {
        backend  => 'Redis',
        servers  => ['cache1:6379', 'cache2:6379', 'cache3:6379'],
        options  => {
            namespace   => 'myapp:',
            default_ttl => 3600,
            serializer  => 'JSON',
            compression => {
                enabled   => 1,
                threshold => 1024,
                algorithm => 'zstd',
            },
        },
        pools => {
            session  => { size => 10, ttl => 86400 },
            data     => { size => 50, ttl => 3600 },
            fragment => { size => 20, ttl => 600 },
        },
    },
    logging => {
        level     => 'info',
        format    => '[%d] [%p] %c - %m%n',
        appenders => [
            {
                type   => 'File',
                path   => '/var/log/myapp/app.log',
                rotate => { size => '100M', keep => 10, compress => 1 },
            },
            {
                type     => 'Syslog',
                facility => 'local0',
                ident    => 'myapp',
            },
            {
                type    => 'Screen',
                stderr  => 1,
                pattern => '%d %p %m%n',
            },
        ],
        categories => {
            'My::App::DB'    => { level => 'warn',  additivity => 0 },
            'My::App::Cache' => { level => 'debug', additivity => 1 },
            'My::App::Auth'  => { level => 'info',  additivity => 1 },
        },
    },
    routes => [
        { method => 'GET',    path => '/',             handler => 'Root#index' },
        { method => 'GET',    path => '/api/v1/users', handler => 'API::Users#list' },
        { method => 'POST',   path => '/api/v1/users', handler => 'API::Users#create' },
        { method => 'GET',    path => '/api/v1/users/:id',    handler => 'API::Users#show' },
        { method => 'PUT',    path => '/api/v1/users/:id',    handler => 'API::Users#update' },
        { method => 'DELETE', path => '/api/v1/users/:id',    handler => 'API::Users#delete' },
        { method => 'GET',    path => '/api/v1/posts',        handler => 'API::Posts#list' },
        { method => 'POST',   path => '/api/v1/posts',        handler => 'API::Posts#create' },
        { method => 'GET',    path => '/health',              handler => 'Health#check' },
        { method => 'GET',    path => '/metrics',             handler => 'Metrics#show' },
    ],
    middleware => [
        ['RequestID'],
        ['AccessLog', { format => 'combined' }],
        ['Session',   { store => 'Redis', expires => 86400 }],
        ['Auth',      { realm => 'API', optional_paths => ['/health', '/metrics'] }],
        ['RateLimit', { max_requests => 100, window => 60, by => 'ip' }],
        ['CORS',      { origins => ['https://app.example.com'], methods => ['GET', 'POST', 'PUT', 'DELETE'] }],
    ],
};

sub new {
    my ($class, %overrides) = @_;
    my $config = _deep_merge($DEFAULT_CONFIG, \%overrides);
    return bless { config => $config }, $class;
}

sub get {
    my ($self, $path) = @_;
    my @keys = split /\./, $path;
    my $node = $self->{config};
    foreach my $key (@keys) {
        if (ref $node eq 'HASH') {
            $node = $node->{$key};
        }
        elsif (ref $node eq 'ARRAY' && $key =~ /^\d+$/) {
            $node = $node->[$key];
        }
        else {
            return;
        }
    }
    return $node;
}

sub set {
    my ($self, $path, $value) = @_;
    my @keys = split /\./, $path;
    my $last_key = pop @keys;
    my $node = $self->{config};
    foreach my $key (@keys) {
        if (ref $node eq 'HASH') {
            $node->{$key} //= {};
            $node = $node->{$key};
        }
        else {
            croak "Cannot traverse non-hash at '$key' in path '$path'";
        }
    }
    $node->{$last_key} = $value;
    return $self;
}

sub _deep_merge {
    my ($base, $override) = @_;
    my %merged = %{$base};
    foreach my $key (keys %{$override}) {
        if (ref $merged{$key} eq 'HASH' && ref $override->{$key} eq 'HASH') {
            $merged{$key} = _deep_merge($merged{$key}, $override->{$key});
        }
        else {
            $merged{$key} = $override->{$key};
        }
    }
    return \%merged;
}

sub dump_config {
    my ($self) = @_;
    return _dump_recursive($self->{config}, 0);
}

sub _dump_recursive {
    my ($data, $indent) = @_;
    my $prefix = '  ' x $indent;
    my $output = '';
    if (ref $data eq 'HASH') {
        $output .= "{\n";
        foreach my $key (sort keys %{$data}) {
            $output .= "$prefix  $key => ";
            $output .= _dump_recursive($data->{$key}, $indent + 1);
        }
        $output .= "$prefix}\n";
    }
    elsif (ref $data eq 'ARRAY') {
        $output .= "[\n";
        foreach my $item (@{$data}) {
            $output .= "$prefix  ";
            $output .= _dump_recursive($item, $indent + 1);
        }
        $output .= "$prefix]\n";
    }
    else {
        $output .= defined $data ? "'$data'" : 'undef';
        $output .= "\n";
    }
    return $output;
}

1;
"#;

// ---------------------------------------------------------------------------
// 5. Large module (~1000 lines) — generated by repeating realistic patterns
// ---------------------------------------------------------------------------

/// Build a large (~1000-line) Perl module string at runtime.
///
/// This avoids bloating the binary with a giant string constant while still
/// exercising the parser on a realistically sized file.
fn build_large_module() -> String {
    let mut code = String::with_capacity(48_000);

    // Header
    code.push_str(
        r#"package My::Large::Module;
use strict;
use warnings;
use Carp qw(croak confess);
use Scalar::Util qw(blessed reftype weaken);
use List::Util qw(reduce any all none first max min sum);
use POSIX qw(strftime ceil floor);
use File::Basename qw(basename dirname);
use File::Path qw(make_path remove_tree);
use Digest::SHA qw(sha256_hex);
use JSON qw(encode_json decode_json);
use Data::Dumper;

our $VERSION = '1.42';

my %_REGISTRY;
my @_HOOKS;
my $_INSTANCE_COUNT = 0;

use constant {
    STATUS_ACTIVE   => 'active',
    STATUS_INACTIVE => 'inactive',
    STATUS_PENDING  => 'pending',
    STATUS_ERROR    => 'error',
    MAX_RETRIES     => 5,
    DEFAULT_TIMEOUT => 30,
    BATCH_SIZE      => 100,
};

"#,
    );

    // Generate 40 realistic subroutines
    for i in 0..40 {
        let sub_code = match i % 8 {
            0 => format!(
                r#"
sub process_item_{i} {{
    my ($self, $item) = @_;
    croak "Item required" unless defined $item;

    my $key = $item->{{id}} // "unknown_{i}";
    my $data = $self->_fetch_data($key);

    if ($data && ref $data eq 'HASH') {{
        foreach my $field (sort keys %{{$data}}) {{
            my $val = $data->{{$field}};
            if (ref $val eq 'ARRAY') {{
                $item->{{$field}} = [map {{ $_ * 2 }} @{{$val}}];
            }} elsif (ref $val eq 'HASH') {{
                $item->{{$field}} = {{ %{{$val}}, processed => 1 }};
            }} else {{
                $item->{{$field}} = $val;
            }}
        }}
    }}

    $self->_log_action("processed item $key");
    return $item;
}}
"#
            ),
            1 => format!(
                r#"
sub validate_{i} {{
    my ($self, %params) = @_;
    my @errors;

    if (!defined $params{{name}} || $params{{name}} eq '') {{
        push @errors, "Name is required for validation #{i}";
    }}
    if (defined $params{{age}} && ($params{{age}} < 0 || $params{{age}} > 200)) {{
        push @errors, "Age out of range in validator #{i}";
    }}
    if (defined $params{{email}} && $params{{email}} !~ /^[^@]+\@[^@]+\.[^@]+$/) {{
        push @errors, "Invalid email format";
    }}
    if (defined $params{{phone}} && $params{{phone}} !~ /^\+?\d[\d\s-]{{7,14}}$/) {{
        push @errors, "Invalid phone format";
    }}

    return @errors ? (0, \@errors) : (1, []);
}}
"#
            ),
            2 => format!(
                r#"
sub transform_data_{i} {{
    my ($self, $input) = @_;
    return unless ref $input eq 'HASH';

    my %output;
    while (my ($key, $value) = each %{{$input}}) {{
        my $new_key = lc($key);
        $new_key =~ s/\s+/_/g;
        $new_key =~ s/[^a-z0-9_]//g;

        if (ref $value eq 'ARRAY') {{
            $output{{$new_key}} = [
                map {{
                    ref $_ eq 'HASH'
                        ? $self->transform_data_{i}($_)
                        : $_
                }} @{{$value}}
            ];
        }} elsif (ref $value eq 'HASH') {{
            $output{{$new_key}} = $self->transform_data_{i}($value);
        }} else {{
            $output{{$new_key}} = $value;
        }}
    }}

    return \%output;
}}
"#
            ),
            3 => format!(
                r#"
sub search_{i} {{
    my ($self, $query, %opts) = @_;
    my $limit  = $opts{{limit}}  // BATCH_SIZE;
    my $offset = $opts{{offset}} // 0;
    my $sort   = $opts{{sort}}   // 'id';

    my @results;
    my @items = @{{$self->{{items_{i}}} // []}};

    foreach my $item (@items) {{
        next unless defined $item;
        my $match = 0;

        if (ref $query eq 'HASH') {{
            $match = 1;
            while (my ($field, $pattern) = each %{{$query}}) {{
                if (ref $pattern eq 'Regexp') {{
                    $match = 0 unless ($item->{{$field}} // '') =~ $pattern;
                }} else {{
                    $match = 0 unless ($item->{{$field}} // '') eq $pattern;
                }}
            }}
        }} elsif (ref $query eq 'CODE') {{
            $match = $query->($item);
        }} else {{
            $match = ($item->{{name}} // '') =~ /\Q$query\E/i;
        }}

        push @results, $item if $match;
    }}

    @results = sort {{ ($a->{{$sort}} // '') cmp ($b->{{$sort}} // '') }} @results;
    my $total = scalar @results;
    @results = splice(@results, $offset, $limit);

    return {{
        items => \@results,
        total => $total,
        page  => int($offset / $limit) + 1,
    }};
}}
"#
            ),
            4 => format!(
                r#"
sub batch_process_{i} {{
    my ($self, $items, $callback) = @_;
    croak "Items must be an arrayref" unless ref $items eq 'ARRAY';

    my @chunks;
    my @copy = @{{$items}};
    while (@copy) {{
        push @chunks, [splice @copy, 0, BATCH_SIZE];
    }}

    my @all_results;
    my $chunk_num = 0;
    foreach my $chunk (@chunks) {{
        $chunk_num++;
        my @results;
        foreach my $item (@{{$chunk}}) {{
            my $result = eval {{ $callback->($item) }};
            if ($@) {{
                push @results, {{ error => $@, item => $item }};
            }} else {{
                push @results, {{ ok => 1, result => $result }};
            }}
        }}
        push @all_results, @results;
        $self->_log_action("Batch #{i} chunk $chunk_num processed");
    }}

    return \@all_results;
}}
"#
            ),
            5 => format!(
                r#"
sub format_report_{i} {{
    my ($self, $data, %opts) = @_;
    my $format = $opts{{format}} // 'text';
    my $title  = $opts{{title}}  // "Report #{i}";

    my @lines;
    push @lines, "=" x 60;
    push @lines, $title;
    push @lines, "=" x 60;
    push @lines, sprintf("Generated: %s", strftime("%Y-%m-%d %H:%M:%S", localtime));
    push @lines, "";

    if (ref $data eq 'ARRAY') {{
        my $row_num = 0;
        foreach my $row (@{{$data}}) {{
            $row_num++;
            if (ref $row eq 'HASH') {{
                push @lines, "--- Row $row_num ---";
                foreach my $key (sort keys %{{$row}}) {{
                    push @lines, sprintf("  %-20s: %s", $key, $row->{{$key}} // '(undef)');
                }}
            }} else {{
                push @lines, "  [$row_num] $row";
            }}
        }}
    }} elsif (ref $data eq 'HASH') {{
        foreach my $section (sort keys %{{$data}}) {{
            push @lines, "--- $section ---";
            my $content = $data->{{$section}};
            if (ref $content eq 'HASH') {{
                foreach my $key (sort keys %{{$content}}) {{
                    push @lines, sprintf("  %-20s: %s", $key, $content->{{$key}} // '(undef)');
                }}
            }}
        }}
    }}

    push @lines, "";
    push @lines, "=" x 60;
    return join("\n", @lines);
}}
"#
            ),
            6 => format!(
                r#"
sub cache_operation_{i} {{
    my ($self, $key, $code) = @_;
    my $cache = $self->{{_cache_{i}}} //= {{}};
    my $ttl   = $self->{{_cache_ttl}} // 300;

    if (my $entry = $cache->{{$key}}) {{
        if (time() - $entry->{{stored_at}} < $ttl) {{
            $self->{{_cache_hits}}++;
            return $entry->{{value}};
        }}
        delete $cache->{{$key}};
    }}

    my $value = eval {{ $code->() }};
    if ($@) {{
        $self->_log_action("Cache miss error for $key: $@");
        return;
    }}

    $cache->{{$key}} = {{
        value     => $value,
        stored_at => time(),
    }};
    $self->{{_cache_misses}}++;

    return $value;
}}
"#
            ),
            7 => format!(
                r#"
sub parse_input_{i} {{
    my ($self, $raw_input) = @_;
    return unless defined $raw_input;

    # Normalize line endings
    $raw_input =~ s/\r\n/\n/g;
    $raw_input =~ s/\r/\n/g;

    my @records;
    my $current_record = {{}};
    my $in_block = 0;

    foreach my $line (split /\n/, $raw_input) {{
        # Skip comments and blank lines
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;

        if ($line =~ /^\[(\w+)\]$/) {{
            if (%{{$current_record}}) {{
                push @records, {{ %{{$current_record}} }};
            }}
            $current_record = {{ section => $1 }};
            $in_block = 1;
        }}
        elsif ($in_block && $line =~ /^\s*(\w+)\s*=\s*(.*)$/) {{
            my ($key, $val) = ($1, $2);
            $val =~ s/\s+$//;
            $val =~ s/^"(.*)"$/$1/;
            $current_record->{{$key}} = $val;
        }}
    }}

    push @records, $current_record if %{{$current_record}};
    return \@records;
}}
"#
            ),
            _ => String::new(),
        };
        code.push_str(&sub_code);
    }

    // Footer
    code.push_str(
        r#"
sub new {
    my ($class, %args) = @_;
    $_INSTANCE_COUNT++;
    my $self = bless {
        id      => $_INSTANCE_COUNT,
        status  => STATUS_PENDING,
        created => time(),
        %args,
    }, $class;
    $_REGISTRY{$self->{id}} = $self;
    weaken($_REGISTRY{$self->{id}});
    return $self;
}

sub _log_action {
    my ($self, $msg) = @_;
    my $ts = strftime("%Y-%m-%d %H:%M:%S", localtime);
    push @{$self->{_log} //= []}, "[$ts] $msg";
    return;
}

sub _fetch_data {
    my ($self, $key) = @_;
    return $self->{_data}{$key};
}

sub DESTROY {
    my $self = shift;
    delete $_REGISTRY{$self->{id}} if $self->{id};
    return;
}

1;
"#,
    );

    code
}

// ---------------------------------------------------------------------------
// Benchmark functions
// ---------------------------------------------------------------------------

fn bench_moose_class(c: &mut Criterion) {
    c.bench_function("cpan/moose_oo_class", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(MOOSE_CLASS));
            let _ = parser.parse();
        });
    });
}

fn bench_many_subs(c: &mut Criterion) {
    c.bench_function("cpan/many_subs_module", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(MANY_SUBS_MODULE));
            let _ = parser.parse();
        });
    });
}

fn bench_heavy_regex(c: &mut Criterion) {
    c.bench_function("cpan/heavy_regex_module", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(HEAVY_REGEX_MODULE));
            let _ = parser.parse();
        });
    });
}

fn bench_complex_data_structures(c: &mut Criterion) {
    c.bench_function("cpan/complex_data_structures", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(COMPLEX_DATA_STRUCTURES));
            let _ = parser.parse();
        });
    });
}

fn bench_large_module(c: &mut Criterion) {
    let code = build_large_module();
    let line_count = code.lines().count();
    assert!(line_count >= 1000, "Large module should be 1000+ lines, got {}", line_count);

    c.bench_function("cpan/large_module_1000_lines", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(&code));
            let _ = parser.parse();
        });
    });
}

fn bench_large_module_scope_analysis(c: &mut Criterion) {
    use perl_parser::ScopeAnalyzer;

    let code = build_large_module();
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("large module must parse for scope benchmark");
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = vec![];

    c.bench_function("cpan/large_module_scope_analysis", |b| {
        b.iter(|| {
            analyzer.analyze(black_box(&ast), black_box(&code), black_box(&pragma_map));
        });
    });
}

criterion_group!(
    benches,
    bench_moose_class,
    bench_many_subs,
    bench_heavy_regex,
    bench_complex_data_structures,
    bench_large_module,
    bench_large_module_scope_analysis,
);
criterion_main!(benches);
