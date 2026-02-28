use strict;
use warnings;

# --- glob() function syntax ---
my @files = glob("*.txt");
my @files2 = glob "*.pl";
my @multi = glob "*.pl *.pm";
my @all = glob "**/*.pm";
my @hidden = glob ".*";
my @chars = glob "[a-z]*.pl";
my @brace = glob "file{1,2,3}.txt";
my @nested = glob "dir1/*/dir2/*.pm";
my @mix = glob "/tmp/{a,b,c}*.txt";

# --- glob with variables ---
my $pattern = "*.rs";
my @matched = glob($pattern);
my $dir = "/usr/lib";
my @libs = glob("$dir/*.pm");

# --- Diamond operator / angle bracket glob ---
my @pm = <*.pm>;
my @deep = <lib/**/*.pm>;
my @interp = <$dir/*.pl>;

# --- glob in scalar context ---
my $single = glob "*.log";

# --- glob in while loop ---
while (my $file = glob("*.log")) {
    print "Found: $file\n";
}

# --- Readline disambiguation ---
# These are readline, NOT glob:
while (<STDIN>) {
    chomp;
}
open my $fh, "<", "file.txt" or die $!;
while (<$fh>) {
    print;
}

# This is glob (contains metacharacter):
my @f = <*.txt>;

# --- Complex patterns ---
my @types = glob("{*.pl,*.pm,*.t}");
my @deep2 = glob("lib/**/*.pm");
my @question = glob("file?.txt");
my @negated = glob("file[!0-9].txt");
my @tilde = glob("~/*.txt");
