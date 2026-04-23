//! Edge case tests for file operations and I/O edge cases

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Basic file operations
        ("open FH, 'file.txt'", "open 2-arg"),
        ("open FH, '<', 'file.txt'", "open 3-arg read"),
        ("open FH, '>', 'file.txt'", "open 3-arg write"),
        ("open FH, '>>', 'file.txt'", "open 3-arg append"),
        ("open FH, '+<', 'file.txt'", "open read-write"),
        ("open FH, '+>', 'file.txt'", "open write-read"),
        ("open FH, '+>>', 'file.txt'", "open append-read"),
        // Lexical filehandles
        ("open my $fh, '<', 'file.txt'", "open lexical filehandle"),
        ("open my $fh, '<', 'file.txt' or die $!", "open with error check"),
        ("open(my $fh, '<', 'file.txt') || die", "open parens style"),
        // Pipe operations
        ("open FH, '|-', 'command'", "open pipe to command"),
        ("open FH, '-|', 'command'", "open pipe from command"),
        ("open FH, '|command'", "open pipe to (2-arg)"),
        ("open FH, 'command|'", "open pipe from (2-arg)"),
        // Complex open modes
        ("open FH, '<:utf8', 'file.txt'", "open with encoding"),
        ("open FH, '<:raw', 'file.txt'", "open raw mode"),
        ("open FH, '<:crlf', 'file.txt'", "open crlf mode"),
        ("open FH, '<:encoding(UTF-8)', 'file.txt'", "open with encoding layer"),
        ("open FH, '<:via(Module)', 'file.txt'", "open with via layer"),
        // Multiple layers
        ("open FH, '<:raw:utf8', 'file.txt'", "open multiple layers"),
        ("open FH, '<:utf8:crlf', 'file.txt'", "open utf8 and crlf"),
        // Memory files
        ("open my $fh, '<', \\$scalar", "open scalar ref"),
        ("open my $fh, '>', \\$scalar", "open scalar ref write"),
        ("open my $fh, '>>', \\$scalar", "open scalar ref append"),
        ("open my $fh, '+<', \\$scalar", "open scalar ref read-write"),
        // Duplicate filehandles
        ("open my $fh, '<&', \\*STDIN", "duplicate STDIN"),
        ("open my $fh, '>&', \\*STDOUT", "duplicate STDOUT"),
        ("open my $fh, '<&=', $fileno", "duplicate by fileno"),
        ("open my $fh, '>&=', $fileno", "duplicate output by fileno"),
        // Special filenames
        ("open FH, '-'", "open stdin/stdout"),
        ("open FH, '<', '-'", "open stdin explicit"),
        ("open FH, '>', '-'", "open stdout explicit"),
        // Null filehandle
        ("<>", "null filehandle readline"),
        ("while (<>) { }", "null filehandle in while"),
        ("@lines = <>", "null filehandle to array"),
        // ARGV filehandle
        ("<ARGV>", "ARGV filehandle"),
        ("while (<ARGV>) { }", "ARGV in while"),
        // Readline operations
        ("<FH>", "readline filehandle"),
        ("<$fh>", "readline scalar filehandle"),
        ("<*FH>", "readline glob"),
        ("readline FH", "readline function"),
        ("readline $fh", "readline scalar"),
        ("readline", "readline default"),
        // Glob readline
        ("<*.txt>", "glob pattern"),
        ("<~user/*>", "glob with tilde"),
        ("glob '*.txt'", "glob function"),
        // Print variations
        ("print", "print default"),
        ("print $_", "print explicit default"),
        ("print 'hello'", "print string"),
        ("print FH 'hello'", "print to filehandle"),
        ("print $fh 'hello'", "print to scalar filehandle"),
        ("print {$fh} 'hello'", "print to braced filehandle"),
        ("print { select_fh() } 'hello'", "print to expression filehandle"),
        // Printf variations
        ("printf '%s', $x", "printf default handle"),
        ("printf FH '%s', $x", "printf to filehandle"),
        ("printf $fh '%s', $x", "printf to scalar filehandle"),
        // Say variations
        ("say", "say default"),
        ("say 'hello'", "say string"),
        ("say FH 'hello'", "say to filehandle"),
        ("say $fh 'hello'", "say to scalar filehandle"),
        // Close operations
        ("close FH", "close filehandle"),
        ("close $fh", "close scalar filehandle"),
        ("close", "close default"),
        ("close FH or die $!", "close with error check"),
        // Binmode operations
        ("binmode FH", "binmode basic"),
        ("binmode FH, ':utf8'", "binmode with layer"),
        ("binmode FH, ':raw'", "binmode raw"),
        ("binmode FH, ':encoding(UTF-8)'", "binmode encoding"),
        // Select operations
        ("select FH", "select filehandle"),
        ("select $fh", "select scalar filehandle"),
        ("select", "select default"),
        ("$old = select FH", "select with return"),
        ("select((select(FH), $| = 1)[0])", "select autoflush idiom"),
        // Seek and tell
        ("seek FH, 0, 0", "seek to beginning"),
        ("seek FH, 0, 1", "seek current"),
        ("seek FH, 0, 2", "seek to end"),
        ("seek $fh, $pos, $whence", "seek variables"),
        ("tell FH", "tell filehandle"),
        ("tell $fh", "tell scalar filehandle"),
        ("tell", "tell default"),
        // Truncate
        ("truncate FH, 0", "truncate filehandle"),
        ("truncate 'file.txt', 100", "truncate filename"),
        ("truncate $fh, $size", "truncate variables"),
        // Stat operations
        ("stat FH", "stat filehandle"),
        ("stat 'file.txt'", "stat filename"),
        ("stat", "stat default"),
        ("@info = stat FH", "stat to array"),
        ("lstat 'link'", "lstat"),
        // File test operators
        ("-e 'file.txt'", "file exists"),
        ("-f FH", "is regular file"),
        ("-d 'dir'", "is directory"),
        ("-l 'link'", "is symlink"),
        ("-r FH", "is readable"),
        ("-w FH", "is writable"),
        ("-x FH", "is executable"),
        ("-o FH", "is owned by user"),
        ("-R FH", "is readable by real uid"),
        ("-W FH", "is writable by real uid"),
        ("-X FH", "is executable by real uid"),
        ("-O FH", "is owned by real uid"),
        ("-z FH", "is zero size"),
        ("-s FH", "file size"),
        ("-M FH", "modification age"),
        ("-A FH", "access age"),
        ("-C FH", "inode change age"),
        ("-t FH", "is tty"),
        ("-p FH", "is pipe"),
        ("-S FH", "is socket"),
        ("-b FH", "is block device"),
        ("-c FH", "is char device"),
        ("-u FH", "is setuid"),
        ("-g FH", "is setgid"),
        ("-k FH", "is sticky"),
        ("-T FH", "is text file"),
        ("-B FH", "is binary file"),
        // Stacked file tests
        ("-f -r 'file.txt'", "stacked file tests"),
        ("-r -w -x FH", "multiple stacked tests"),
        // Default file test
        ("-e", "file test on $_"),
        ("-f", "file test on $_ regular"),
        // Eof operations
        ("eof", "eof default"),
        ("eof FH", "eof filehandle"),
        ("eof $fh", "eof scalar filehandle"),
        ("eof()", "eof parentheses"),
        // Fileno operations
        ("fileno FH", "fileno filehandle"),
        ("fileno $fh", "fileno scalar filehandle"),
        ("fileno \\*STDIN", "fileno glob ref"),
        // Flock operations
        ("flock FH, 1", "flock shared"),
        ("flock FH, 2", "flock exclusive"),
        ("flock FH, 8", "flock non-blocking"),
        ("flock $fh, LOCK_EX", "flock with constant"),
        // Fcntl and ioctl
        ("fcntl FH, $cmd, $arg", "fcntl"),
        ("ioctl FH, $cmd, $arg", "ioctl"),
        // Sysopen
        ("sysopen FH, 'file.txt', O_RDONLY", "sysopen read"),
        ("sysopen FH, 'file.txt', O_WRONLY|O_CREAT", "sysopen write create"),
        ("sysopen FH, 'file.txt', O_RDWR, 0644", "sysopen with mode"),
        // Sysread/syswrite
        ("sysread FH, $buf, 1024", "sysread"),
        ("sysread FH, $buf, 1024, 100", "sysread with offset"),
        ("syswrite FH, $buf", "syswrite"),
        ("syswrite FH, $buf, 1024", "syswrite with length"),
        ("syswrite FH, $buf, 1024, 100", "syswrite with offset"),
        // Sysseek
        ("sysseek FH, 0, 0", "sysseek"),
        // Directory operations
        ("opendir DH, '.'", "opendir"),
        ("opendir my $dh, '.'", "opendir lexical"),
        ("readdir DH", "readdir scalar"),
        ("@files = readdir DH", "readdir list"),
        ("closedir DH", "closedir"),
        ("rewinddir DH", "rewinddir"),
        ("telldir DH", "telldir"),
        ("seekdir DH, $pos", "seekdir"),
        // Chmod/chown
        ("chmod 0755, 'file.txt'", "chmod"),
        ("chmod 0755, @files", "chmod multiple"),
        ("chown $uid, $gid, 'file.txt'", "chown"),
        ("chown -1, -1, @files", "chown unchanged"),
        // Link operations
        ("link 'old', 'new'", "hard link"),
        ("symlink 'target', 'link'", "symbolic link"),
        ("readlink 'link'", "read link"),
        // Unlink/rename
        ("unlink 'file.txt'", "unlink single"),
        ("unlink @files", "unlink multiple"),
        ("rename 'old', 'new'", "rename"),
        // Mkdir/rmdir
        ("mkdir 'dir'", "mkdir default perms"),
        ("mkdir 'dir', 0755", "mkdir with perms"),
        ("rmdir 'dir'", "rmdir"),
        // Utime
        ("utime $atime, $mtime, @files", "utime"),
        ("utime undef, undef, @files", "utime now"),
        // Pipe operations
        ("pipe READ, WRITE", "pipe creation"),
        ("pipe my $read, my $write", "pipe lexical"),
        // Socket operations
        ("socket SOCK, AF_INET, SOCK_STREAM, 0", "socket creation"),
        ("socketpair S1, S2, AF_UNIX, SOCK_STREAM, 0", "socketpair"),
        // Select with timeout
        ("select $rout, $wout, $eout, $timeout", "select multiplexing"),
        ("select undef, undef, undef, 0.5", "select sleep"),
        // Getc
        ("getc", "getc default"),
        ("getc FH", "getc filehandle"),
        ("getc $fh", "getc scalar filehandle"),
        // Read
        ("read FH, $buf, 1024", "read"),
        ("read FH, $buf, 1024, $offset", "read with offset"),
        // Special variables
        ("$.", "input line number"),
        ("$/", "input record separator"),
        ("$\\", "output record separator"),
        ("$|", "autoflush"),
        ("$,", "output field separator"),
        ("$\"", "list separator"),
        // STDIN/STDOUT/STDERR
        ("print STDERR 'error'", "print to STDERR"),
        ("<STDIN>", "read from STDIN"),
        ("print STDOUT 'output'", "print to STDOUT"),
        // DATA filehandle
        ("<DATA>", "read from DATA"),
        ("while (<DATA>) { }", "DATA in while"),
        ("__DATA__", "DATA section marker"),
        ("__END__", "END section marker"),
    ]
}
