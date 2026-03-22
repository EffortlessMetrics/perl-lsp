//! Built-in function completion for Perl
//!
//! Provides completion for Perl built-in functions with signatures.

use super::{context::CompletionContext, items::CompletionItem};
use std::collections::HashSet;

/// Create the builtins HashSet
pub fn create_builtins() -> HashSet<&'static str> {
    [
        // I/O
        "print",
        "printf",
        "say",
        "sprintf",
        "open",
        "close",
        "read",
        "write",
        "seek",
        "tell",
        "binmode",
        "eof",
        "fileno",
        "flock",
        "getc",
        "readline",
        "sysread",
        "syswrite",
        "sysseek",
        "pipe",
        // String
        "chomp",
        "chop",
        "chr",
        "ord",
        "lc",
        "uc",
        "lcfirst",
        "ucfirst",
        "length",
        "substr",
        "index",
        "rindex",
        "split",
        "join",
        "reverse",
        "quotemeta",
        // Array
        "push",
        "pop",
        "shift",
        "unshift",
        "splice",
        "grep",
        "map",
        "sort",
        "wantarray",
        "scalar",
        // Hash
        "keys",
        "values",
        "each",
        "exists",
        "delete",
        // Math / numeric
        "abs",
        "atan2",
        "cos",
        "sin",
        "exp",
        "log",
        "sqrt",
        "int",
        "rand",
        "srand",
        "hex",
        "oct",
        // File tests (operators, not functions, but surfaced as completions)
        "-r",
        "-w",
        "-x",
        "-o",
        "-R",
        "-W",
        "-X",
        "-O",
        "-e",
        "-z",
        "-s",
        "-f",
        "-d",
        "-l",
        "-p",
        "-S",
        "-b",
        "-c",
        "-t",
        "-u",
        "-g",
        "-k",
        "-T",
        "-B",
        "-M",
        "-A",
        "-C",
        // File system
        "stat",
        "lstat",
        "rename",
        "unlink",
        "mkdir",
        "rmdir",
        "chdir",
        "chmod",
        "chown",
        "link",
        "symlink",
        "readlink",
        "glob",
        "opendir",
        "readdir",
        "closedir",
        "rewinddir",
        "telldir",
        "seekdir",
        "truncate",
        // System / process
        "system",
        "exec",
        "fork",
        "wait",
        "waitpid",
        "kill",
        "sleep",
        "alarm",
        "getpid",
        "getppid",
        "times",
        // Time
        "time",
        "localtime",
        "gmtime",
        // Misc / context
        "caller",
        "die",
        "warn",
        "eval",
        "exit",
        "require",
        "use",
        "no",
        "import",
        "unimport",
        "bless",
        "ref",
        "tied",
        "untie",
        "pack",
        "unpack",
        "vec",
        "study",
        "pos",
        "qr",
        "defined",
        "undef",
        "prototype",
        "reset",
        "dump",
        "dbmopen",
        "dbmclose",
        // Network / socket
        "socket",
        "socketpair",
        "listen",
        "accept",
        "connect",
        "bind",
        "recv",
        "send",
        "shutdown",
        "getpeername",
        "getsockname",
        "getsockopt",
        "setsockopt",
        // IPC
        "msgctl",
        "msgget",
        "msgrcv",
        "msgsnd",
        "semctl",
        "semget",
        "semop",
        "shmctl",
        "shmget",
        "shmread",
        "shmwrite",
        // User / group
        "getlogin",
        "getpwnam",
        "getpwuid",
        "getpwent",
        "setpwent",
        "endpwent",
        "getgrnam",
        "getgrgid",
        "getgrent",
        "setgrent",
        "endgrent",
        // Network lookup
        "gethostbyname",
        "gethostbyaddr",
        "gethostent",
        "sethostent",
        "endhostent",
        "getnetbyname",
        "getnetbyaddr",
        "getnetent",
        "setnetent",
        "endnetent",
        "getprotobyname",
        "getprotobynumber",
        "getprotoent",
        "setprotoent",
        "endprotoent",
        "getservbyname",
        "getservbyport",
        "getservent",
        "setservent",
        "endservent",
    ]
    .into_iter()
    .collect()
}

/// Add built-in function completions
pub fn add_builtin_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    builtins: &HashSet<&'static str>,
) {
    for builtin in builtins {
        if builtin.starts_with(&context.prefix) {
            let (insert_text, detail, doc) = match *builtin {
                "print" => (
                    "print ",
                    "print FILEHANDLE LIST",
                    "Print to a filehandle. Default output is STDOUT.",
                ),
                "printf" => (
                    "printf ",
                    "printf FILEHANDLE FORMAT, LIST",
                    "Print formatted output to a filehandle.",
                ),
                "say" => (
                    "say ",
                    "say FILEHANDLE LIST",
                    "Like print but appends a newline. Requires use feature 'say'.",
                ),
                "open" => (
                    "open(my $fh, '<', )",
                    "open FILEHANDLE, MODE, FILENAME",
                    "Open a file. Mode: '<' read, '>' write, '>>' append, '+<' read/write.",
                ),
                "close" => (
                    "close ",
                    "close FILEHANDLE",
                    "Close an open filehandle, flushing output buffers.",
                ),
                "read" => (
                    "read ",
                    "read FILEHANDLE, SCALAR, LENGTH",
                    "Read up to LENGTH bytes from FILEHANDLE into SCALAR.",
                ),
                "write" => (
                    "write ",
                    "write FILEHANDLE",
                    "Write a formatted record to the current format (see format/write).",
                ),
                "readline" => (
                    "readline ",
                    "readline FILEHANDLE",
                    "Read a line from FILEHANDLE; equivalent to the <FH> operator.",
                ),
                "getc" => (
                    "getc ",
                    "getc FILEHANDLE",
                    "Return the next character from FILEHANDLE, or undef on EOF.",
                ),
                "push" => (
                    "push(@, )",
                    "push ARRAY, LIST",
                    "Append LIST elements to the end of ARRAY. Returns new count.",
                ),
                "pop" => ("pop ", "pop ARRAY", "Remove and return the last element of ARRAY."),
                "shift" => {
                    ("shift ", "shift ARRAY", "Remove and return the first element of ARRAY.")
                }
                "unshift" => (
                    "unshift ",
                    "unshift ARRAY, LIST",
                    "Prepend LIST to ARRAY. Returns new element count.",
                ),
                "splice" => (
                    "splice(@, , , )",
                    "splice ARRAY, OFFSET, LENGTH, LIST",
                    "Remove and replace elements in ARRAY.",
                ),
                "map" => (
                    "map { } ",
                    "map BLOCK LIST",
                    "Apply BLOCK to each element of LIST; return the transformed list.",
                ),
                "grep" => (
                    "grep { } ",
                    "grep BLOCK LIST",
                    "Filter LIST keeping elements for which BLOCK returns true.",
                ),
                "sort" => (
                    "sort { } ",
                    "sort BLOCK LIST",
                    "Sort LIST, optionally using BLOCK as a comparison function.",
                ),
                "reverse" => (
                    "reverse ",
                    "reverse LIST",
                    "In list context: reverse a list. In scalar context: reverse a string.",
                ),
                "split" => (
                    "split(//, )",
                    "split /PATTERN/, EXPR, LIMIT",
                    "Split EXPR by PATTERN. Returns a list of substrings.",
                ),
                "join" => (
                    "join(', ', )",
                    "join EXPR, LIST",
                    "Join LIST elements with EXPR as separator. Returns a string.",
                ),
                "chomp" => (
                    "chomp ",
                    "chomp VARIABLE",
                    "Remove trailing newline from string. Returns number of characters removed.",
                ),
                "chop" => {
                    ("chop ", "chop VARIABLE", "Remove and return the last character of a string.")
                }
                "substr" => (
                    "substr(, , )",
                    "substr EXPR, OFFSET, LENGTH",
                    "Extract a substring from EXPR. Can also be used as an lvalue.",
                ),
                "index" => (
                    "index(, )",
                    "index STR, SUBSTR, POSITION",
                    "Return the position of SUBSTR in STR, or -1 if not found.",
                ),
                "rindex" => (
                    "rindex(, )",
                    "rindex STR, SUBSTR, POSITION",
                    "Return the rightmost position of SUBSTR in STR, or -1.",
                ),
                "length" => ("length ", "length EXPR", "Return the number of characters in EXPR."),
                "sprintf" => (
                    "sprintf(, )",
                    "sprintf FORMAT, LIST",
                    "Return a formatted string (like printf but to a string).",
                ),
                "lc" => ("lc ", "lc EXPR", "Return a lowercased copy of EXPR."),
                "uc" => ("uc ", "uc EXPR", "Return an uppercased copy of EXPR."),
                "lcfirst" => {
                    ("lcfirst ", "lcfirst EXPR", "Return EXPR with the first character lowercased.")
                }
                "ucfirst" => {
                    ("ucfirst ", "ucfirst EXPR", "Return EXPR with the first character uppercased.")
                }
                "defined" => (
                    "defined ",
                    "defined EXPR",
                    "Return true if EXPR has a defined (non-undef) value.",
                ),
                "undef" => (
                    "undef ",
                    "undef EXPR",
                    "Undefine a variable or subroutine, freeing its memory.",
                ),
                "wantarray" => (
                    "wantarray",
                    "wantarray",
                    "Return true if the current sub was called in list context.",
                ),
                "scalar" => (
                    "scalar ",
                    "scalar EXPR",
                    "Force scalar context on EXPR; returns count for arrays.",
                ),
                "ref" => (
                    "ref ",
                    "ref EXPR",
                    "Return the type of reference EXPR is (e.g. 'SCALAR', 'ARRAY', 'HASH', 'CODE').",
                ),
                "bless" => (
                    "bless(, )",
                    "bless REF, CLASSNAME",
                    "Associate REF with CLASSNAME for OO dispatch.",
                ),
                "caller" => (
                    "caller",
                    "caller EXPR",
                    "Return information about the calling sub: (package, filename, line).",
                ),
                "die" => ("die ", "die LIST", "Raise an exception with LIST as the error message."),
                "warn" => ("warn ", "warn LIST", "Print LIST to STDERR as a warning."),
                "eval" => {
                    ("eval { }", "eval BLOCK", "Trap exceptions from BLOCK; check $@ afterwards.")
                }
                "exit" => ("exit ", "exit EXPR", "Exit the program with status EXPR (default 0)."),
                "require" => (
                    "require ",
                    "require EXPR",
                    "Load and execute a Perl module or file at runtime.",
                ),
                "stat" => (
                    "stat ",
                    "stat FILEHANDLE|EXPR",
                    "Return a 13-element list of file status info (size, mtime, etc.).",
                ),
                "lstat" => (
                    "lstat ",
                    "lstat FILEHANDLE|EXPR",
                    "Like stat but on a symbolic link itself, not its target.",
                ),
                "rename" => (
                    "rename(, )",
                    "rename OLDNAME, NEWNAME",
                    "Rename a file. Returns true on success.",
                ),
                "unlink" => (
                    "unlink ",
                    "unlink LIST",
                    "Delete files in LIST. Returns count of files deleted.",
                ),
                "mkdir" => (
                    "mkdir(, )",
                    "mkdir FILENAME, MODE",
                    "Create a directory. Mode defaults to 0777.",
                ),
                "rmdir" => ("rmdir ", "rmdir FILENAME", "Remove an empty directory."),
                "chdir" => ("chdir ", "chdir EXPR", "Change the working directory to EXPR."),
                "chmod" => {
                    ("chmod(, )", "chmod MODE, LIST", "Change permissions on files in LIST.")
                }
                "chown" => (
                    "chown(, , )",
                    "chown UID, GID, LIST",
                    "Change owner and group on files in LIST.",
                ),
                "link" => (
                    "link(, )",
                    "link OLDFILE, NEWFILE",
                    "Create a hard link NEWFILE pointing to OLDFILE.",
                ),
                "symlink" => (
                    "symlink(, )",
                    "symlink OLDFILE, NEWFILE",
                    "Create a symbolic link NEWFILE pointing to OLDFILE.",
                ),
                "readlink" => {
                    ("readlink ", "readlink EXPR", "Return the path a symbolic link points to.")
                }
                "opendir" => (
                    "opendir(my $dh, )",
                    "opendir DIRHANDLE, EXPR",
                    "Open directory EXPR for reading with DIRHANDLE.",
                ),
                "readdir" => (
                    "readdir ",
                    "readdir DIRHANDLE",
                    "Return next entry (or all entries in list context) from a directory.",
                ),
                "closedir" => (
                    "closedir ",
                    "closedir DIRHANDLE",
                    "Close a directory handle opened by opendir.",
                ),
                "glob" => {
                    ("glob ", "glob EXPR", "Expand shell glob patterns in EXPR; like <*.pl>.")
                }
                "truncate" => (
                    "truncate(, )",
                    "truncate FILEHANDLE|EXPR, LENGTH",
                    "Truncate a file to LENGTH bytes.",
                ),
                "socket" => (
                    "socket(, , , )",
                    "socket SOCKET, DOMAIN, TYPE, PROTOCOL",
                    "Create a socket. See PF_INET, SOCK_STREAM in Socket module.",
                ),
                "socketpair" => (
                    "socketpair(, , , )",
                    "socketpair SOCK1, SOCK2, DOMAIN, TYPE, PROTOCOL",
                    "Create a pair of connected sockets.",
                ),
                "listen" => (
                    "listen(, )",
                    "listen SOCKET, QUEUESIZE",
                    "Set a socket to listen for incoming connections.",
                ),
                "accept" => (
                    "accept(, )",
                    "accept NEWSOCKET, GENERICSOCKET",
                    "Accept an incoming socket connection.",
                ),
                "connect" => {
                    ("connect(, )", "connect SOCKET, NAME", "Connect a socket to a remote address.")
                }
                "bind" => ("bind(, )", "bind SOCKET, NAME", "Bind a socket to a local address."),
                "recv" => (
                    "recv(, , , )",
                    "recv SOCKET, SCALAR, LENGTH, FLAGS",
                    "Receive a message from a socket into SCALAR.",
                ),
                "send" => ("send(, , )", "send SOCKET, MSG, FLAGS", "Send a message on a socket."),
                "shutdown" => (
                    "shutdown(, )",
                    "shutdown SOCKET, HOW",
                    "Shut down a socket connection (0=read, 1=write, 2=both).",
                ),
                "getpeername" => (
                    "getpeername ",
                    "getpeername SOCKET",
                    "Return the remote address of a connected socket.",
                ),
                "getsockname" => {
                    ("getsockname ", "getsockname SOCKET", "Return the local address of a socket.")
                }
                "getsockopt" => (
                    "getsockopt(, , )",
                    "getsockopt SOCKET, LEVEL, OPTNAME",
                    "Return a socket option value.",
                ),
                "setsockopt" => (
                    "setsockopt(, , , )",
                    "setsockopt SOCKET, LEVEL, OPTNAME, OPTVAL",
                    "Set a socket option.",
                ),
                "times" => (
                    "times",
                    "times",
                    "Return (user, system, cuser, csystem) CPU times in seconds.",
                ),
                "getlogin" => {
                    ("getlogin", "getlogin", "Return the login name of the current user.")
                }
                "getpwnam" => {
                    ("getpwnam ", "getpwnam NAME", "Return the passwd entry for user NAME.")
                }
                "getpwuid" => {
                    ("getpwuid ", "getpwuid UID", "Return the passwd entry for user UID.")
                }
                "getgrnam" => {
                    ("getgrnam ", "getgrnam NAME", "Return the group entry for group NAME.")
                }
                "getgrgid" => {
                    ("getgrgid ", "getgrgid GID", "Return the group entry for group GID.")
                }
                "gethostbyname" => (
                    "gethostbyname ",
                    "gethostbyname NAME",
                    "Resolve a hostname to its address(es).",
                ),
                "gethostbyaddr" => (
                    "gethostbyaddr(, )",
                    "gethostbyaddr ADDR, ADDRTYPE",
                    "Reverse-resolve a packed address to a hostname.",
                ),
                "getprotobyname" => (
                    "getprotobyname ",
                    "getprotobyname NAME",
                    "Return protocol info by name (e.g. 'tcp').",
                ),
                "getprotobynumber" => (
                    "getprotobynumber ",
                    "getprotobynumber NUMBER",
                    "Return protocol info by protocol number.",
                ),
                "getservbyname" => (
                    "getservbyname(, )",
                    "getservbyname NAME, PROTO",
                    "Return service info by name and protocol (e.g. 'http', 'tcp').",
                ),
                "getservbyport" => (
                    "getservbyport(, )",
                    "getservbyport PORT, PROTO",
                    "Return service info by port number and protocol.",
                ),
                "hex" => ("hex ", "hex EXPR", "Convert a hex string to a decimal number."),
                "oct" => (
                    "oct ",
                    "oct EXPR",
                    "Convert an octal (or hex/binary with prefix) string to a number.",
                ),
                "abs" => ("abs ", "abs EXPR", "Return the absolute value of EXPR."),
                "int" => (
                    "int ",
                    "int EXPR",
                    "Return the integer portion of EXPR (truncate toward zero).",
                ),
                "sqrt" => ("sqrt ", "sqrt EXPR", "Return the square root of EXPR."),
                "prototype" => (
                    "prototype ",
                    "prototype FUNCTION",
                    "Return the prototype string of a function, or undef if none.",
                ),
                "pack" => (
                    "pack(, )",
                    "pack TEMPLATE, LIST",
                    "Pack LIST values into a binary string according to TEMPLATE.",
                ),
                "unpack" => (
                    "unpack(, )",
                    "unpack TEMPLATE, EXPR",
                    "Unpack a binary string EXPR according to TEMPLATE.",
                ),
                _ => (*builtin, "built-in function", "Perl built-in function."),
            };

            completions.push(CompletionItem {
                label: builtin.to_string(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some(detail.to_string()),
                documentation: Some(doc.to_string()),
                insert_text: Some(insert_text.to_string()),
                sort_text: Some(format!("3_{}", builtin)),
                filter_text: Some(builtin.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}
