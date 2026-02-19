#!/usr/bin/env perl
# Test: Real-world CLI tool with Getopt, Pod, text processing, config, logging
# NodeKinds exercised: Use, FunctionCall, MethodCall, If, While, For, Foreach,
#     Regex, Match, Substitution, Subroutine, Variable, Assignment, HashLiteral,
#     ArrayLiteral, Return, Block, Ternary, StatementModifier, Binary
# Purpose: A realistic medium-complexity CLI tool exercising many nodekinds together

use strict;
use warnings;
use Getopt::Long qw(:config bundling no_ignore_case);
use File::Basename qw(basename dirname);
use File::Spec;
use Carp qw(croak);

# --- Configuration ---
my %DEFAULT_CONFIG = (
    verbose    => 0,
    quiet      => 0,
    color      => 1,
    max_lines  => 0,        # 0 = unlimited
    encoding   => 'utf-8',
    pattern    => undef,
    output     => '-',       # stdout
    format     => 'text',    # text, json, csv
    ignore_case => 0,
);

my %config = %DEFAULT_CONFIG;

# --- Option parsing ---
GetOptions(
    'v|verbose+'      => \$config{verbose},
    'q|quiet'         => \$config{quiet},
    'c|color!'        => \$config{color},
    'n|max-lines=i'   => \$config{max_lines},
    'e|encoding=s'    => \$config{encoding},
    'p|pattern=s'     => \$config{pattern},
    'o|output=s'      => \$config{output},
    'f|format=s'      => \$config{format},
    'i|ignore-case'   => \$config{ignore_case},
    'h|help'          => sub { usage(); exit 0 },
    'V|version'       => sub { print basename($0) . " v1.0.0\n"; exit 0 },
) or do { usage(); exit 1 };

# --- Logging ---
my %LOG_LEVELS = (DEBUG => 0, INFO => 1, WARN => 2, ERROR => 3);

sub log_msg {
    my ($level, $msg) = @_;
    return if $config{quiet} && $level ne 'ERROR';
    return if $LOG_LEVELS{$level} < $config{verbose};

    my $prefix = $config{color}
        ? colorize($level, "[$level]")
        : "[$level]";

    printf STDERR "%s %s\n", $prefix, $msg;
}

sub colorize {
    my ($level, $text) = @_;
    my %colors = (
        DEBUG => "\e[36m",    # cyan
        INFO  => "\e[32m",    # green
        WARN  => "\e[33m",    # yellow
        ERROR => "\e[31m",    # red
    );
    my $reset = "\e[0m";
    return ($colors{$level} // "") . $text . $reset;
}

# --- Text processing ---
sub process_line {
    my ($line, $line_num, $pattern_re) = @_;
    chomp $line;

    # Skip empty lines
    return undef unless length $line;

    # Apply pattern filter
    if ($pattern_re) {
        return undef unless $line =~ $pattern_re;
    }

    # Normalize whitespace
    $line =~ s/^\s+//;       # trim leading
    $line =~ s/\s+$//;       # trim trailing
    $line =~ s/\s+/ /g;      # collapse internal whitespace

    # Expand tabs
    $line =~ s/\t/    /g;

    return {
        line_num => $line_num,
        content  => $line,
        length   => length($line),
        words    => scalar(my @w = split /\s+/, $line),
    };
}

sub process_file {
    my ($filename) = @_;
    my @results;
    my $line_num = 0;
    my $total_lines = 0;
    my $matched_lines = 0;

    # Compile pattern once
    my $pattern_re;
    if (defined $config{pattern}) {
        my $flags = $config{ignore_case} ? "(?i)" : "";
        $pattern_re = eval { qr/${flags}$config{pattern}/ };
        if ($@) {
            log_msg('ERROR', "Invalid pattern: $@");
            return ();
        }
    }

    # Open input
    my $fh;
    if ($filename eq '-') {
        $fh = \*STDIN;
    } else {
        open $fh, "<:encoding($config{encoding})", $filename
            or do { log_msg('ERROR', "Cannot open $filename: $!"); return () };
    }

    log_msg('INFO', "Processing: $filename");

    while (my $line = <$fh>) {
        $line_num++;
        $total_lines++;

        last if $config{max_lines} && $matched_lines >= $config{max_lines};

        my $result = process_line($line, $line_num, $pattern_re);
        if ($result) {
            push @results, $result;
            $matched_lines++;
        }
    }

    close $fh unless $filename eq '-';

    log_msg('INFO', sprintf "  %d/%d lines matched", $matched_lines, $total_lines);

    return @results;
}

# --- Output formatting ---
sub format_results {
    my ($results_ref, $format) = @_;
    my @results = @{$results_ref};

    if ($format eq 'json') {
        return format_json(\@results);
    } elsif ($format eq 'csv') {
        return format_csv(\@results);
    } else {
        return format_text(\@results);
    }
}

sub format_text {
    my ($results) = @_;
    my @lines;

    for my $r (@{$results}) {
        my $prefix = sprintf "%4d", $r->{line_num};
        push @lines, "$prefix: $r->{content}";
    }

    return join("\n", @lines) . "\n";
}

sub format_csv {
    my ($results) = @_;
    my @lines = ("line_num,length,words,content");

    for my $r (@{$results}) {
        my $escaped = $r->{content};
        $escaped =~ s/"/""/g;         # escape quotes for CSV
        push @lines, join(",", $r->{line_num}, $r->{length}, $r->{words}, qq("$escaped"));
    }

    return join("\n", @lines) . "\n";
}

sub format_json {
    my ($results) = @_;
    my @items;

    for my $r (@{$results}) {
        my $content = $r->{content};
        $content =~ s/\\/\\\\/g;      # escape backslash
        $content =~ s/"/\\"/g;        # escape quotes
        $content =~ s/\n/\\n/g;       # escape newlines
        $content =~ s/\t/\\t/g;       # escape tabs

        push @items, sprintf(
            '  {"line": %d, "length": %d, "words": %d, "content": "%s"}',
            $r->{line_num}, $r->{length}, $r->{words}, $content
        );
    }

    return "[\n" . join(",\n", @items) . "\n]\n";
}

# --- Statistics ---
sub compute_stats {
    my (@results) = @_;
    return {} unless @results;

    my $total_words = 0;
    my $total_len = 0;
    my ($min_len, $max_len) = ($results[0]{length}, $results[0]{length});

    for my $r (@results) {
        $total_words += $r->{words};
        $total_len += $r->{length};
        $min_len = $r->{length} if $r->{length} < $min_len;
        $max_len = $r->{length} if $r->{length} > $max_len;
    }

    return {
        lines      => scalar @results,
        total_words => $total_words,
        avg_length  => $total_len / scalar @results,
        min_length  => $min_len,
        max_length  => $max_len,
    };
}

# --- Usage ---
sub usage {
    print <<'USAGE';
Usage: text_processor [OPTIONS] [FILE...]

Options:
  -v, --verbose      Increase verbosity (can repeat: -vv)
  -q, --quiet        Suppress non-error output
  -c, --color        Enable color output (default: on)
      --no-color     Disable color output
  -n, --max-lines=N  Process at most N matching lines
  -e, --encoding=E   Input encoding (default: utf-8)
  -p, --pattern=P    Filter lines matching regex pattern
  -i, --ignore-case  Case-insensitive pattern matching
  -o, --output=FILE  Output file (default: stdout)
  -f, --format=FMT   Output format: text, json, csv (default: text)
  -h, --help         Show this help
  -V, --version      Show version

Examples:
  text_processor file.txt
  text_processor -p 'error|warn' -i /var/log/syslog
  text_processor -f json -o results.json *.txt
USAGE
}

# --- Main ---
my @files = @ARGV ? @ARGV : ('-');
my @all_results;

for my $file (@files) {
    my @results = process_file($file);
    push @all_results, @results;
}

# Output
if (@all_results) {
    my $output = format_results(\@all_results, $config{format});

    if ($config{output} eq '-') {
        print $output;
    } else {
        open my $out_fh, ">:encoding($config{encoding})", $config{output}
            or die "Cannot open $config{output}: $!\n";
        print $out_fh $output;
        close $out_fh;
    }
}

# Stats in verbose mode
if ($config{verbose} >= 2 && @all_results) {
    my $stats = compute_stats(@all_results);
    log_msg('DEBUG', sprintf "Stats: %d lines, %d words, avg length %.1f",
        $stats->{lines}, $stats->{total_words}, $stats->{avg_length});
}

log_msg('INFO', "Done.") unless $config{quiet};
