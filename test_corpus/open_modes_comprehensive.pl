#!/usr/bin/env perl
# Test: All open() modes, sysopen, encoding layers, pipes, and duplication
# NodeKinds exercised: FunctionCall, String, Variable, Assignment, If, Binary, Block
# Coverage gap: open() is the most polymorphic Perl builtin

use strict;
use warnings;
use Fcntl qw(:DEFAULT :flock :seek);

# --- Three-argument open (modern) ---
# Read
open my $fh_read, "<", "input.txt" or die "Cannot open: $!";
close $fh_read;

# Write (truncate)
open my $fh_write, ">", "output.txt" or die "Cannot open: $!";
print $fh_write "data\n";
close $fh_write;

# Append
open my $fh_append, ">>", "output.txt" or die "Cannot open: $!";
close $fh_append;

# Read-write
open my $fh_rw, "+<", "data.txt" or die "Cannot open: $!";
close $fh_rw;

# Read-write truncate
open my $fh_rwt, "+>", "data.txt" or die "Cannot open: $!";
close $fh_rwt;

# Read-write append
open my $fh_rwa, "+>>", "data.txt" or die "Cannot open: $!";
close $fh_rwa;

# --- Two-argument open (legacy, still common) ---
# open FH, "<filename";        # bareword filehandle
# open FH, ">filename";
# open FH, ">>filename";

# --- Encoding layers ---
open my $fh_utf8, "<:encoding(UTF-8)", "utf8.txt" or die $!;
close $fh_utf8;

open my $fh_latin, "<:encoding(iso-8859-1)", "latin.txt" or die $!;
close $fh_latin;

# Raw binary mode
open my $fh_raw, "<:raw", "binary.dat" or die $!;
close $fh_raw;

# Multiple layers
open my $fh_layers, "<:raw:perlio", "file.dat" or die $!;
close $fh_layers;

# Unix line endings
open my $fh_unix, "<:raw:crlf", "mixed.txt" or die $!;
close $fh_unix;

# Bytes layer
open my $fh_bytes, "<:bytes", "data.bin" or die $!;
close $fh_bytes;

# --- In-memory open (string references) ---
my $buffer = "line1\nline2\nline3\n";
open my $fh_str_read, "<", \$buffer or die $!;
while (<$fh_str_read>) {
    chomp;
}
close $fh_str_read;

# Write to string
my $output = "";
open my $fh_str_write, ">", \$output or die $!;
print $fh_str_write "captured output\n";
close $fh_str_write;

# Append to string
open my $fh_str_append, ">>", \$output or die $!;
print $fh_str_append "more output\n";
close $fh_str_append;

# --- Pipe open ---
# Read from command
open my $pipe_read, "-|", "ls", "-la" or die $!;
while (<$pipe_read>) {
    chomp;
}
close $pipe_read;

# Write to command
open my $pipe_write, "|-", "sort" or die $!;
print $pipe_write "banana\napple\ncherry\n";
close $pipe_write;

# Two-arg pipe (legacy)
# open my $legacy_pipe, "ls -la |" or die $!;
# open my $legacy_pipe_w, "| sort" or die $!;

# --- File descriptor duplication ---
# Dup stdout
open my $saved_stdout, ">&", \*STDOUT or die $!;
open my $saved_stderr, ">&", \*STDERR or die $!;

# Redirect stderr to stdout
# open STDERR, ">&", STDOUT or die $!;

# Dup with fileno
open my $dup_stdin, "<&=", fileno(STDIN) or die $!;

# --- sysopen ---
sysopen my $sys_fh, "sysfile.txt", O_RDONLY or die $!;
close $sys_fh;

sysopen my $sys_create, "new.txt", O_WRONLY | O_CREAT | O_TRUNC, 0644 or die $!;
close $sys_create;

sysopen my $sys_excl, "exclusive.txt", O_WRONLY | O_CREAT | O_EXCL, 0600 or die $!;
close $sys_excl;

sysopen my $sys_append, "append.txt", O_WRONLY | O_APPEND | O_CREAT or die $!;
close $sys_append;

# --- opendir ---
opendir my $dh, "." or die "Cannot opendir: $!";
my @entries = readdir $dh;
closedir $dh;

# Grep with readdir
opendir my $dh2, "." or die $!;
my @pl_files = grep { /\.pl$/ } readdir $dh2;
closedir $dh2;

# --- Filehandle operations after open ---
my $test_file = "test_open.txt";
open my $fh, ">", $test_file or die $!;

# Autoflush
my $old_fh = select($fh);
$| = 1;
select($old_fh);

# Print variants
print $fh "print\n";
printf $fh "printf %d\n", 42;
say { $fh } "say with block";

# File locking
flock($fh, LOCK_EX) or die "Cannot lock: $!";
flock($fh, LOCK_UN);

# Seek and tell
seek($fh, 0, SEEK_SET) or die $!;
my $position = tell($fh);

# Truncate
truncate($fh, 0) or die $!;

# Binmode
binmode($fh);
binmode($fh, ":utf8");

# EOF and stat on filehandle
my @stat_result = stat($fh);
# eof($fh);

close $fh;

# --- Special filehandles ---
# DATA section filehandle (see data_end_sections.pl)
# ARGV for <> operator (see readline_diamond_operator_comprehensive.pl)
# ARGVOUT for -i inplace editing

print "Open modes test complete\n";
