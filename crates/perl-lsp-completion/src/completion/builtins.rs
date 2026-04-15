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
        // UTF-8 encoding (perldoc utf8)
        "utf8::encode",
        "utf8::decode",
        "utf8::is_utf8",
        "utf8::valid",
        "utf8::upgrade",
        "utf8::downgrade",
        "utf8::native_to_unicode",
        "utf8::unicode_to_native",
    ]
    .into_iter()
    .collect()
}

/// Returns `(insert_text, detail, documentation)` for a named built-in.
///
/// `insert_text` is the text inserted into the editor (may include a template
/// suffix). `detail` is the brief signature shown inline. `documentation` is
/// the longer description shown in the completion popup hover.
///
/// All built-in names come from the `create_builtins()` HashSet which stores
/// `&'static str`, so the input lifetime is `'static`.
fn builtin_info(name: &'static str) -> (&'static str, &'static str, Option<&'static str>) {
    match name {
        // I/O
        "print" => (
            "print ",
            "print FILEHANDLE LIST",
            Some("Print a list to a filehandle (default: STDOUT). Returns true on success."),
        ),
        "printf" => (
            "printf ",
            "printf FILEHANDLE FORMAT, LIST",
            Some("Print a formatted string to a filehandle. Like sprintf but outputs directly."),
        ),
        "say" => (
            "say ",
            "say FILEHANDLE LIST",
            Some("Like print, but appends a newline. Requires 'use feature :5.10' or 'use 5.010'."),
        ),
        "sprintf" => (
            "sprintf ",
            "sprintf FORMAT, LIST",
            Some("Format a string according to FORMAT, returning the result instead of printing."),
        ),
        "open" => (
            "open(my $fh, '${1:<}', ${2:\\$file}) or die \"Cannot open ${2:\\$file}: $!\";",
            "open FILEHANDLE, MODE, FILENAME",
            Some(
                "Open a file or pipe. Three-arg form with idiomatic error handling: open(my $fh, '<', $file) or die ...",
            ),
        ),
        "close" => (
            "close ",
            "close FILEHANDLE",
            Some("Close a filehandle and flush buffers. Returns true on success."),
        ),
        "read" => (
            "read ",
            "read FILEHANDLE, SCALAR, LENGTH",
            Some("Read up to LENGTH bytes from FILEHANDLE into SCALAR. Returns bytes read."),
        ),
        "write" => (
            "write ",
            "write FILEHANDLE",
            Some("Write a formatted record to a filehandle using format/write declarations."),
        ),
        "seek" => (
            "seek ",
            "seek FILEHANDLE, POSITION, WHENCE",
            Some("Seek to POSITION in FILEHANDLE. WHENCE: 0=start, 1=current, 2=end."),
        ),
        "tell" => (
            "tell ",
            "tell FILEHANDLE",
            Some("Return current file position for FILEHANDLE as a byte offset."),
        ),
        "binmode" => (
            "binmode ",
            "binmode FILEHANDLE",
            Some(
                "Set a filehandle to binary mode (no CRLF translation). Use a layer for encoding.",
            ),
        ),
        "eof" => ("eof ", "eof FILEHANDLE", Some("Return true if at end of FILEHANDLE.")),
        "fileno" => (
            "fileno ",
            "fileno FILEHANDLE",
            Some("Return the file descriptor number for FILEHANDLE."),
        ),
        "flock" => (
            "flock ",
            "flock FILEHANDLE, OPERATION",
            Some("Advisory file lock. OPERATION: LOCK_SH, LOCK_EX, LOCK_UN, LOCK_NB."),
        ),
        "readline" => (
            "readline ",
            "readline FILEHANDLE",
            Some("Read a line from FILEHANDLE; equivalent to the <FH> operator."),
        ),
        "getc" => (
            "getc ",
            "getc FILEHANDLE",
            Some("Return the next character from FILEHANDLE, or undef on EOF."),
        ),
        // String
        "chomp" => (
            "chomp ",
            "chomp LIST",
            Some("Remove trailing input record separator (usually newline) from a string or list."),
        ),
        "chop" => ("chop ", "chop LIST", Some("Remove and return the last character of a string.")),
        "chr" => (
            "chr ",
            "chr NUMBER",
            Some("Return the character represented by NUMBER in the current character set."),
        ),
        "ord" => (
            "ord ",
            "ord EXPR",
            Some("Return the numeric character value of the first character of EXPR."),
        ),
        "lc" => ("lc ", "lc EXPR", Some("Return EXPR converted to lowercase.")),
        "uc" => ("uc ", "uc EXPR", Some("Return EXPR converted to uppercase.")),
        "lcfirst" => {
            ("lcfirst ", "lcfirst EXPR", Some("Return EXPR with the first character lowercased."))
        }
        "ucfirst" => {
            ("ucfirst ", "ucfirst EXPR", Some("Return EXPR with the first character uppercased."))
        }
        "length" => (
            "length ",
            "length EXPR",
            Some("Return the length of EXPR in characters. Use bytes::length for byte count."),
        ),
        "substr" => (
            "substr ",
            "substr EXPR, OFFSET, LENGTH",
            Some("Return a substring of EXPR starting at OFFSET for LENGTH chars."),
        ),
        "index" => (
            "index ",
            "index STR, SUBSTR, POSITION",
            Some("Return position of first occurrence of SUBSTR in STR, or -1 if not found."),
        ),
        "rindex" => (
            "rindex ",
            "rindex STR, SUBSTR, POSITION",
            Some("Like index but searches from the right. Returns last occurrence position."),
        ),
        "split" => (
            "split ",
            "split /PATTERN/, EXPR, LIMIT",
            Some(
                "Split EXPR by PATTERN. Returns a list of fields. Limit restricts number of fields.",
            ),
        ),
        "join" => (
            "join ",
            "join EXPR, LIST",
            Some("Join elements of LIST into a single string separated by EXPR."),
        ),
        "reverse" => (
            "reverse ",
            "reverse LIST",
            Some(
                "In list context, return elements in reverse order. In scalar context, reverse a string.",
            ),
        ),
        "quotemeta" => (
            "quotemeta ",
            "quotemeta EXPR",
            Some("Return EXPR with all non-word characters backslash-escaped. Same as \\Q...\\E."),
        ),
        // Array
        "push" => (
            "push(@, )",
            "push ARRAY, LIST",
            Some("Append LIST to the end of ARRAY. Returns the new number of elements."),
        ),
        "pop" => ("pop ", "pop ARRAY", Some("Remove and return the last element of ARRAY.")),
        "shift" => (
            "shift ",
            "shift ARRAY",
            Some("Remove and return the first element of ARRAY (shifts left). Default: @_."),
        ),
        "unshift" => (
            "unshift ",
            "unshift ARRAY, LIST",
            Some("Prepend LIST to the beginning of ARRAY. Returns the new count."),
        ),
        "splice" => (
            "splice ",
            "splice ARRAY, OFFSET, LENGTH, LIST",
            Some("Remove and replace elements in ARRAY. Returns removed elements."),
        ),
        "grep" => (
            "grep { } ",
            "grep BLOCK LIST",
            Some(
                "Filter LIST by evaluating BLOCK for each element ($_). Returns matching elements.",
            ),
        ),
        "map" => (
            "map { } ",
            "map BLOCK LIST",
            Some("Transform LIST by evaluating BLOCK for each element ($_). Returns new list."),
        ),
        "sort" => (
            "sort { } ",
            "sort BLOCK LIST",
            Some("Sort LIST. BLOCK receives $a and $b. Use <=> for numeric, cmp for string sort."),
        ),
        "wantarray" => (
            "wantarray",
            "wantarray",
            Some("Return true if the current sub was called in list context."),
        ),
        "scalar" => (
            "scalar ",
            "scalar EXPR",
            Some("Force scalar context on EXPR; returns count for arrays."),
        ),
        // Hash
        "keys" => (
            "keys ",
            "keys HASH",
            Some("Return a list of all keys in HASH. Order is undefined unless sorted."),
        ),
        "values" => (
            "values ",
            "values HASH",
            Some("Return a list of all values in HASH. Order matches keys()."),
        ),
        "each" => (
            "each ",
            "each HASH",
            Some("Return the next key-value pair from HASH as a 2-element list."),
        ),
        "exists" => (
            "exists ",
            "exists EXPR",
            Some("Return true if the hash key or array element named by EXPR exists."),
        ),
        "delete" => (
            "delete ",
            "delete EXPR",
            Some("Delete a hash key or array element. Returns the deleted value."),
        ),
        // Math
        "abs" => ("abs ", "abs VALUE", Some("Return the absolute value of VALUE.")),
        "atan2" => {
            ("atan2 ", "atan2 Y, X", Some("Return the arctangent of Y/X in the range -pi to pi."))
        }
        "cos" => ("cos ", "cos EXPR", Some("Return the cosine of EXPR (in radians).")),
        "sin" => ("sin ", "sin EXPR", Some("Return the sine of EXPR (in radians).")),
        "exp" => ("exp ", "exp EXPR", Some("Return e raised to the power EXPR.")),
        "log" => ("log ", "log EXPR", Some("Return the natural logarithm of EXPR.")),
        "sqrt" => ("sqrt ", "sqrt EXPR", Some("Return the non-negative square root of EXPR.")),
        "int" => (
            "int ",
            "int EXPR",
            Some("Return the integer portion of EXPR (truncates toward zero)."),
        ),
        "rand" => (
            "rand ",
            "rand EXPR",
            Some("Return a random fractional number in [0, EXPR). Default EXPR is 1."),
        ),
        "srand" => (
            "srand ",
            "srand EXPR",
            Some("Seed the random number generator. Without arg, uses a platform-specific seed."),
        ),
        "hex" => ("hex ", "hex EXPR", Some("Convert a hex string to a decimal number.")),
        "oct" => (
            "oct ",
            "oct EXPR",
            Some("Convert an octal (or hex/binary with prefix) string to a number."),
        ),
        // System
        "system" => (
            "system ",
            "system LIST",
            Some("Run a system command. Returns exit status. STDOUT goes to terminal."),
        ),
        "exec" => (
            "exec ",
            "exec LIST",
            Some("Execute a command, replacing the current process. Never returns on success."),
        ),
        "fork" => (
            "fork",
            "fork",
            Some("Fork the process. Returns child PID to parent, 0 to child, undef on failure."),
        ),
        "wait" => (
            "wait",
            "wait",
            Some("Wait for a child process to terminate. Returns PID of deceased child."),
        ),
        "waitpid" => {
            ("waitpid ", "waitpid PID, FLAGS", Some("Wait for a specific child PID to terminate."))
        }
        "kill" => (
            "kill ",
            "kill SIGNAL, LIST",
            Some("Send SIGNAL to a list of processes. Returns number of processes signalled."),
        ),
        "sleep" => (
            "sleep ",
            "sleep EXPR",
            Some("Sleep for EXPR seconds. Returns number of seconds slept."),
        ),
        "alarm" => (
            "alarm ",
            "alarm SECONDS",
            Some(
                "Schedule a SIGALRM delivery in SECONDS. Returns remaining time of previous alarm.",
            ),
        ),
        "getpid" => {
            ("getpid", "getpid", Some("Return the process ID of the current process. Same as $$."))
        }
        "getppid" => ("getppid", "getppid", Some("Return the process ID of the parent process.")),
        "times" => {
            ("times", "times", Some("Return (user, system, cuser, csystem) CPU times in seconds."))
        }
        // Time
        "time" => (
            "time",
            "time",
            Some("Return the number of seconds since the system epoch (Jan 1, 1970)."),
        ),
        "localtime" => (
            "localtime ",
            "localtime EXPR",
            Some(
                "Convert a time value to local time. In list context returns (sec,min,hour,mday,mon,year,wday,yday,isdst).",
            ),
        ),
        "gmtime" => (
            "gmtime ",
            "gmtime EXPR",
            Some("Convert a time value to UTC. Same list format as localtime."),
        ),
        // Misc
        "caller" => (
            "caller ",
            "caller EXPR",
            Some("Return info about the calling subroutine. Returns (package,filename,line)."),
        ),
        "die" => (
            "die ",
            "die LIST",
            Some("Raise an exception with LIST as the error message. Sets $@."),
        ),
        "warn" => (
            "warn ",
            "warn LIST",
            Some("Print a warning to STDERR. Like die but continues execution."),
        ),
        "eval" => (
            "eval ",
            "eval BLOCK",
            Some("Trap exceptions from BLOCK. On error, $@ is set and eval returns undef."),
        ),
        "exit" => (
            "exit ",
            "exit EXPR",
            Some(
                "Exit the program with status EXPR (default 0). Runs END blocks and DESTROY methods.",
            ),
        ),
        "require" => (
            "require ",
            "require EXPR",
            Some(
                "Load and execute a Perl file or module at runtime. Checks %INC to avoid re-loading.",
            ),
        ),
        "bless" => (
            "bless ",
            "bless REF, CLASSNAME",
            Some("Associate REF with CLASSNAME for object-oriented dispatch. Returns REF."),
        ),
        "ref" => (
            "ref ",
            "ref EXPR",
            Some("Return a string describing the type of a reference, or '' if not a reference."),
        ),
        "tied" => (
            "tied ",
            "tied VARIABLE",
            Some("Return the object underlying a tied variable, or undef if not tied."),
        ),
        "untie" => (
            "untie ",
            "untie VARIABLE",
            Some("Break the tie between a variable and its tied implementation."),
        ),
        "pack" => (
            "pack ",
            "pack TEMPLATE, LIST",
            Some("Convert a LIST into a binary string according to TEMPLATE."),
        ),
        "unpack" => (
            "unpack ",
            "unpack TEMPLATE, EXPR",
            Some("Unpack a binary string EXPR into a list according to TEMPLATE."),
        ),
        "vec" => (
            "vec ",
            "vec EXPR, OFFSET, BITS",
            Some("Treat EXPR as a bit vector. Get or set BITS-wide field at OFFSET."),
        ),
        "pos" => (
            "pos ",
            "pos SCALAR",
            Some("Return the offset of where the last m//g left off for SCALAR."),
        ),
        "defined" => (
            "defined ",
            "defined EXPR",
            Some("Return true if EXPR has a defined (non-undef) value."),
        ),
        "undef" => {
            ("undef ", "undef EXPR", Some("Undefine a variable or subroutine, freeing its memory."))
        }
        "prototype" => (
            "prototype ",
            "prototype FUNCTION",
            Some("Return the prototype string of a function, or undef if none."),
        ),
        // File system
        "stat" => (
            "stat ",
            "stat FILEHANDLE|EXPR",
            Some("Return a 13-element list of file status info (size, mtime, etc.)."),
        ),
        "lstat" => (
            "lstat ",
            "lstat FILEHANDLE|EXPR",
            Some("Like stat but on a symbolic link itself, not its target."),
        ),
        "rename" => (
            "rename(, )",
            "rename OLDNAME, NEWNAME",
            Some("Rename a file. Returns true on success."),
        ),
        "unlink" => (
            "unlink ",
            "unlink LIST",
            Some("Delete files in LIST. Returns count of files deleted."),
        ),
        "mkdir" => (
            "mkdir(, )",
            "mkdir FILENAME, MODE",
            Some("Create a directory. Mode defaults to 0777."),
        ),
        "rmdir" => ("rmdir ", "rmdir FILENAME", Some("Remove an empty directory.")),
        "chdir" => ("chdir ", "chdir EXPR", Some("Change the working directory to EXPR.")),
        "chmod" => ("chmod(, )", "chmod MODE, LIST", Some("Change permissions on files in LIST.")),
        "chown" => (
            "chown(, , )",
            "chown UID, GID, LIST",
            Some("Change owner and group on files in LIST."),
        ),
        "link" => (
            "link(, )",
            "link OLDFILE, NEWFILE",
            Some("Create a hard link NEWFILE pointing to OLDFILE."),
        ),
        "symlink" => (
            "symlink(, )",
            "symlink OLDFILE, NEWFILE",
            Some("Create a symbolic link NEWFILE pointing to OLDFILE."),
        ),
        "readlink" => {
            ("readlink ", "readlink EXPR", Some("Return the path a symbolic link points to."))
        }
        "opendir" => (
            "opendir(my $dh, )",
            "opendir DIRHANDLE, EXPR",
            Some("Open directory EXPR for reading with DIRHANDLE."),
        ),
        "readdir" => (
            "readdir ",
            "readdir DIRHANDLE",
            Some("Return next entry (or all entries in list context) from a directory."),
        ),
        "closedir" => {
            ("closedir ", "closedir DIRHANDLE", Some("Close a directory handle opened by opendir."))
        }
        "glob" => ("glob ", "glob EXPR", Some("Expand shell glob patterns in EXPR; like <*.pl>.")),
        "truncate" => (
            "truncate(, )",
            "truncate FILEHANDLE|EXPR, LENGTH",
            Some("Truncate a file to LENGTH bytes."),
        ),
        // Network / socket
        "socket" => (
            "socket(, , , )",
            "socket SOCKET, DOMAIN, TYPE, PROTOCOL",
            Some("Create a socket. See PF_INET, SOCK_STREAM in Socket module."),
        ),
        "socketpair" => (
            "socketpair(, , , )",
            "socketpair SOCK1, SOCK2, DOMAIN, TYPE, PROTOCOL",
            Some("Create a pair of connected sockets."),
        ),
        "listen" => (
            "listen(, )",
            "listen SOCKET, QUEUESIZE",
            Some("Set a socket to listen for incoming connections."),
        ),
        "accept" => (
            "accept(, )",
            "accept NEWSOCKET, GENERICSOCKET",
            Some("Accept an incoming socket connection."),
        ),
        "connect" => {
            ("connect(, )", "connect SOCKET, NAME", Some("Connect a socket to a remote address."))
        }
        "bind" => ("bind(, )", "bind SOCKET, NAME", Some("Bind a socket to a local address.")),
        "recv" => (
            "recv(, , , )",
            "recv SOCKET, SCALAR, LENGTH, FLAGS",
            Some("Receive a message from a socket into SCALAR."),
        ),
        "send" => ("send(, , )", "send SOCKET, MSG, FLAGS", Some("Send a message on a socket.")),
        "shutdown" => (
            "shutdown(, )",
            "shutdown SOCKET, HOW",
            Some("Shut down a socket connection (0=read, 1=write, 2=both)."),
        ),
        "getpeername" => (
            "getpeername ",
            "getpeername SOCKET",
            Some("Return the remote address of a connected socket."),
        ),
        "getsockname" => {
            ("getsockname ", "getsockname SOCKET", Some("Return the local address of a socket."))
        }
        "getsockopt" => (
            "getsockopt(, , )",
            "getsockopt SOCKET, LEVEL, OPTNAME",
            Some("Return a socket option value."),
        ),
        "setsockopt" => (
            "setsockopt(, , , )",
            "setsockopt SOCKET, LEVEL, OPTNAME, OPTVAL",
            Some("Set a socket option."),
        ),
        // User / group
        "getlogin" => ("getlogin", "getlogin", Some("Return the login name of the current user.")),
        "getpwnam" => {
            ("getpwnam ", "getpwnam NAME", Some("Return the passwd entry for user NAME."))
        }
        "getpwuid" => ("getpwuid ", "getpwuid UID", Some("Return the passwd entry for user UID.")),
        "getgrnam" => {
            ("getgrnam ", "getgrnam NAME", Some("Return the group entry for group NAME."))
        }
        "getgrgid" => ("getgrgid ", "getgrgid GID", Some("Return the group entry for group GID.")),
        // Network lookup
        "gethostbyname" => {
            ("gethostbyname ", "gethostbyname NAME", Some("Resolve a hostname to its address(es)."))
        }
        "gethostbyaddr" => (
            "gethostbyaddr(, )",
            "gethostbyaddr ADDR, ADDRTYPE",
            Some("Reverse-resolve a packed address to a hostname."),
        ),
        "getprotobyname" => (
            "getprotobyname ",
            "getprotobyname NAME",
            Some("Return protocol info by name (e.g. 'tcp')."),
        ),
        "getprotobynumber" => (
            "getprotobynumber ",
            "getprotobynumber NUMBER",
            Some("Return protocol info by protocol number."),
        ),
        "getservbyname" => (
            "getservbyname(, )",
            "getservbyname NAME, PROTO",
            Some("Return service info by name and protocol (e.g. 'http', 'tcp')."),
        ),
        "getservbyport" => (
            "getservbyport(, )",
            "getservbyport PORT, PROTO",
            Some("Return service info by port number and protocol."),
        ),
        // UTF-8 encoding functions (perldoc utf8). These manipulate the
        // SvUTF8 flag on scalars in place.
        "utf8::encode" => (
            "utf8::encode ",
            "utf8::encode SCALAR",
            Some(
                "Encode SCALAR in place from Unicode to UTF-8 bytes. Clears the UTF-8 flag and returns the number of octets.",
            ),
        ),
        "utf8::decode" => (
            "utf8::decode ",
            "utf8::decode SCALAR",
            Some(
                "Decode SCALAR in place from UTF-8 bytes to Unicode. Sets the UTF-8 flag on success; returns true/false.",
            ),
        ),
        "utf8::is_utf8" => (
            "utf8::is_utf8 ",
            "utf8::is_utf8 SCALAR",
            Some("Return true if the UTF-8 flag is set on SCALAR."),
        ),
        "utf8::valid" => (
            "utf8::valid ",
            "utf8::valid SCALAR",
            Some("Return true if SCALAR holds a syntactically valid UTF-8 byte sequence."),
        ),
        "utf8::upgrade" => (
            "utf8::upgrade ",
            "utf8::upgrade SCALAR",
            Some(
                "Convert SCALAR in place to Perl's internal UTF-8 form. Sets the UTF-8 flag and returns the octet count.",
            ),
        ),
        "utf8::downgrade" => (
            "utf8::downgrade ",
            "utf8::downgrade SCALAR, FAIL_OK",
            Some(
                "Convert SCALAR in place out of Perl's internal UTF-8 form. Croaks unless FAIL_OK is true and the string fits in a single byte.",
            ),
        ),
        "utf8::native_to_unicode" => (
            "utf8::native_to_unicode ",
            "utf8::native_to_unicode CODEPOINT",
            Some(
                "Return the Unicode code point corresponding to a native-platform CODEPOINT (no-op on ASCII).",
            ),
        ),
        "utf8::unicode_to_native" => (
            "utf8::unicode_to_native ",
            "utf8::unicode_to_native CODEPOINT",
            Some("Return the native-platform code point for a Unicode CODEPOINT (no-op on ASCII)."),
        ),
        // Fallthrough: minimal doc for all other builtins
        other => (other, "built-in function", Some("Perl built-in function.")),
    }
}

/// Add built-in function completions
pub fn add_builtin_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    builtins: &HashSet<&'static str>,
) {
    for builtin in builtins {
        if builtin.starts_with(&context.prefix) {
            let (insert_text, detail, documentation) = builtin_info(builtin);

            completions.push(CompletionItem {
                label: builtin.to_string(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some(detail.to_string()),
                documentation: documentation.map(str::to_string),
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
