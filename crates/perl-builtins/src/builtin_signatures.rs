//! Comprehensive built-in function signatures for Perl scripting.
//!
//! This module provides signature information for Perl built-in functions with
//! a focus on parser and LSP feature support. It enables accurate completion,
//! hover, and signature help for common built-ins.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Identifies built-in calls for syntax understanding
//! - **Index**: Classifies symbols for reference tracking
//! - **Navigate**: Powers signature help and related navigation context
//! - **Complete**: Supplies completion labels and arguments
//! - **Analyze**: Supports diagnostics that rely on built-in semantics

use std::collections::HashMap;
use std::sync::OnceLock;

/// Built-in function signature with documentation for Perl script development
///
/// Contains complete signature information and documentation for Perl built-in
/// functions, optimized for Perl parsing use cases within LSP workflows.
pub struct BuiltinSignature {
    /// Function signature variants showing different parameter combinations
    pub signatures: Vec<&'static str>,
    /// Comprehensive documentation explaining function behavior and Perl parsing use cases
    pub documentation: &'static str,
}

static SIGNATURES_CACHE: OnceLock<HashMap<&'static str, BuiltinSignature>> = OnceLock::new();

/// Create comprehensive built-in function signatures
pub fn create_builtin_signatures() -> &'static HashMap<&'static str, BuiltinSignature> {
    SIGNATURES_CACHE.get_or_init(|| {
        let mut signatures = HashMap::new();

        // ===== I/O Functions =====
        signatures.insert(
            "print",
            BuiltinSignature {
                signatures: vec![
                    "print FILEHANDLE LIST",
                    "print FILEHANDLE",
                    "print LIST",
                    "print",
                ],
                documentation: "Prints a string or list of strings to a filehandle",
            },
        );

        signatures.insert(
            "printf",
            BuiltinSignature {
                signatures: vec!["printf FILEHANDLE FORMAT, LIST", "printf FORMAT, LIST"],
                documentation: "Prints a formatted string",
            },
        );

        signatures.insert(
            "say",
            BuiltinSignature {
                signatures: vec!["say FILEHANDLE LIST", "say FILEHANDLE", "say LIST", "say"],
                documentation: "Prints with a newline",
            },
        );

        signatures.insert(
            "open",
            BuiltinSignature {
                signatures: vec![
                    "open FILEHANDLE, MODE, FILENAME",
                    "open FILEHANDLE, EXPR",
                    "open FILEHANDLE",
                ],
                documentation: "Opens a file",
            },
        );

        signatures.insert(
            "sysopen",
            BuiltinSignature {
                signatures: vec![
                    "sysopen FILEHANDLE, FILENAME, MODE, PERMS",
                    "sysopen FILEHANDLE, FILENAME, MODE",
                ],
                documentation: "Opens a file using system call semantics",
            },
        );

        signatures.insert(
            "close",
            BuiltinSignature {
                signatures: vec!["close FILEHANDLE", "close"],
                documentation: "Closes a filehandle",
            },
        );

        signatures.insert(
            "read",
            BuiltinSignature {
                signatures: vec![
                    "read FILEHANDLE, SCALAR, LENGTH, OFFSET",
                    "read FILEHANDLE, SCALAR, LENGTH",
                ],
                documentation: "Reads from a filehandle into a scalar",
            },
        );

        signatures.insert(
            "readline",
            BuiltinSignature {
                signatures: vec!["readline FILEHANDLE", "readline"],
                documentation: "Reads a line from a filehandle",
            },
        );

        signatures.insert(
            "readpipe",
            BuiltinSignature {
                signatures: vec!["readpipe EXPR", "readpipe"],
                documentation: "Executes a command and returns its output",
            },
        );

        signatures.insert(
            "sysread",
            BuiltinSignature {
                signatures: vec![
                    "sysread FILEHANDLE, SCALAR, LENGTH, OFFSET",
                    "sysread FILEHANDLE, SCALAR, LENGTH",
                ],
                documentation: "Reads from a filehandle bypassing stdio",
            },
        );

        signatures.insert(
            "write",
            BuiltinSignature {
                signatures: vec!["write FILEHANDLE", "write"],
                documentation: "Writes a formatted record",
            },
        );

        signatures.insert(
            "syswrite",
            BuiltinSignature {
                signatures: vec![
                    "syswrite FILEHANDLE, SCALAR, LENGTH, OFFSET",
                    "syswrite FILEHANDLE, SCALAR, LENGTH",
                    "syswrite FILEHANDLE, SCALAR",
                ],
                documentation: "Writes to a filehandle bypassing stdio",
            },
        );

        signatures.insert(
            "seek",
            BuiltinSignature {
                signatures: vec!["seek FILEHANDLE, POSITION, WHENCE"],
                documentation: "Sets file pointer position",
            },
        );

        signatures.insert(
            "tell",
            BuiltinSignature {
                signatures: vec!["tell FILEHANDLE", "tell"],
                documentation: "Returns current file position",
            },
        );

        signatures.insert(
            "eof",
            BuiltinSignature {
                signatures: vec!["eof FILEHANDLE", "eof"],
                documentation: "Tests for end of file",
            },
        );

        // ===== String Functions =====
        signatures.insert(
            "chomp",
            BuiltinSignature {
                signatures: vec!["chomp VARIABLE", "chomp LIST", "chomp"],
                documentation: "Removes trailing newline from string",
            },
        );

        signatures.insert(
            "chop",
            BuiltinSignature {
                signatures: vec!["chop VARIABLE", "chop LIST", "chop"],
                documentation: "Removes last character from string",
            },
        );

        signatures.insert(
            "chr",
            BuiltinSignature {
                signatures: vec!["chr NUMBER", "chr"],
                documentation: "Returns character for a number",
            },
        );

        signatures.insert(
            "ord",
            BuiltinSignature {
                signatures: vec!["ord EXPR", "ord"],
                documentation: "Returns numeric value of character",
            },
        );

        signatures.insert(
            "hex",
            BuiltinSignature {
                signatures: vec!["hex EXPR", "hex"],
                documentation: "Converts hex string to number",
            },
        );

        signatures.insert(
            "oct",
            BuiltinSignature {
                signatures: vec!["oct EXPR", "oct"],
                documentation: "Converts octal string to number",
            },
        );

        signatures.insert(
            "length",
            BuiltinSignature {
                signatures: vec!["length EXPR", "length"],
                documentation: "Returns length of string",
            },
        );

        signatures.insert(
            "substr",
            BuiltinSignature {
                signatures: vec![
                    "substr EXPR, OFFSET, LENGTH, REPLACEMENT",
                    "substr EXPR, OFFSET, LENGTH",
                    "substr EXPR, OFFSET",
                ],
                documentation: "Extracts or replaces substring",
            },
        );

        signatures.insert(
            "index",
            BuiltinSignature {
                signatures: vec!["index STR, SUBSTR, POSITION", "index STR, SUBSTR"],
                documentation: "Finds position of substring",
            },
        );

        signatures.insert(
            "rindex",
            BuiltinSignature {
                signatures: vec!["rindex STR, SUBSTR, POSITION", "rindex STR, SUBSTR"],
                documentation: "Finds position of substring from end",
            },
        );

        signatures.insert(
            "sprintf",
            BuiltinSignature {
                signatures: vec!["sprintf FORMAT, LIST"],
                documentation: "Returns formatted string",
            },
        );

        signatures.insert(
            "lc",
            BuiltinSignature {
                signatures: vec!["lc EXPR", "lc"],
                documentation: "Returns lowercase version",
            },
        );

        signatures.insert(
            "lcfirst",
            BuiltinSignature {
                signatures: vec!["lcfirst EXPR", "lcfirst"],
                documentation: "Returns string with first char lowercase",
            },
        );

        signatures.insert(
            "uc",
            BuiltinSignature {
                signatures: vec!["uc EXPR", "uc"],
                documentation: "Returns uppercase version",
            },
        );

        signatures.insert(
            "ucfirst",
            BuiltinSignature {
                signatures: vec!["ucfirst EXPR", "ucfirst"],
                documentation: "Returns string with first char uppercase",
            },
        );

        signatures.insert(
            "quotemeta",
            BuiltinSignature {
                signatures: vec!["quotemeta EXPR", "quotemeta"],
                documentation: "Quotes metacharacters",
            },
        );

        signatures.insert(
            "split",
            BuiltinSignature {
                signatures: vec![
                    "split /PATTERN/, EXPR, LIMIT",
                    "split /PATTERN/, EXPR",
                    "split /PATTERN/",
                    "split",
                ],
                documentation: "Splits string into list",
            },
        );

        signatures.insert(
            "join",
            BuiltinSignature {
                signatures: vec!["join EXPR, LIST"],
                documentation: "Joins list into string",
            },
        );

        signatures.insert(
            "reverse",
            BuiltinSignature {
                signatures: vec!["reverse LIST"],
                documentation: "Reverses list or string",
            },
        );

        // ===== Array Functions =====
        signatures.insert(
            "push",
            BuiltinSignature {
                signatures: vec!["push ARRAY, LIST"],
                documentation: "Appends values to array",
            },
        );

        signatures.insert(
            "pop",
            BuiltinSignature {
                signatures: vec!["pop ARRAY", "pop"],
                documentation: "Removes and returns last element",
            },
        );

        signatures.insert(
            "shift",
            BuiltinSignature {
                signatures: vec!["shift ARRAY", "shift"],
                documentation: "Removes and returns first element",
            },
        );

        signatures.insert(
            "unshift",
            BuiltinSignature {
                signatures: vec!["unshift ARRAY, LIST"],
                documentation: "Prepends values to array",
            },
        );

        signatures.insert(
            "splice",
            BuiltinSignature {
                signatures: vec![
                    "splice ARRAY, OFFSET, LENGTH, LIST",
                    "splice ARRAY, OFFSET, LENGTH",
                    "splice ARRAY, OFFSET",
                    "splice ARRAY",
                ],
                documentation: "Removes and replaces array elements",
            },
        );

        signatures.insert(
            "map",
            BuiltinSignature {
                signatures: vec!["map BLOCK LIST", "map EXPR, LIST"],
                documentation: "Transforms a list",
            },
        );

        signatures.insert(
            "grep",
            BuiltinSignature {
                signatures: vec!["grep BLOCK LIST", "grep EXPR, LIST"],
                documentation: "Filters a list",
            },
        );

        signatures.insert(
            "sort",
            BuiltinSignature {
                signatures: vec!["sort BLOCK LIST", "sort SUBNAME LIST", "sort LIST"],
                documentation: "Sorts a list",
            },
        );

        // ===== Hash Functions =====
        signatures.insert(
            "each",
            BuiltinSignature {
                signatures: vec!["each HASH", "each ARRAY"],
                documentation: "Returns key-value pair",
            },
        );

        signatures.insert(
            "keys",
            BuiltinSignature {
                signatures: vec!["keys HASH", "keys ARRAY"],
                documentation: "Returns list of keys",
            },
        );

        signatures.insert(
            "values",
            BuiltinSignature {
                signatures: vec!["values HASH", "values ARRAY"],
                documentation: "Returns list of values",
            },
        );

        signatures.insert(
            "exists",
            BuiltinSignature {
                signatures: vec!["exists EXPR"],
                documentation: "Tests whether key exists",
            },
        );

        signatures.insert(
            "delete",
            BuiltinSignature {
                signatures: vec!["delete EXPR"],
                documentation: "Deletes hash element",
            },
        );

        // ===== File Test Operators =====
        signatures.insert(
            "stat",
            BuiltinSignature {
                signatures: vec!["stat FILEHANDLE", "stat EXPR", "stat"],
                documentation: "Returns file statistics",
            },
        );

        signatures.insert(
            "lstat",
            BuiltinSignature {
                signatures: vec!["lstat FILEHANDLE", "lstat EXPR", "lstat"],
                documentation: "Returns symbolic link statistics",
            },
        );

        // ===== Directory Functions =====
        signatures.insert(
            "opendir",
            BuiltinSignature {
                signatures: vec!["opendir DIRHANDLE, EXPR"],
                documentation: "Opens a directory",
            },
        );

        signatures.insert(
            "readdir",
            BuiltinSignature {
                signatures: vec!["readdir DIRHANDLE"],
                documentation: "Reads directory entries",
            },
        );

        signatures.insert(
            "closedir",
            BuiltinSignature {
                signatures: vec!["closedir DIRHANDLE"],
                documentation: "Closes a directory handle",
            },
        );

        signatures.insert(
            "rewinddir",
            BuiltinSignature {
                signatures: vec!["rewinddir DIRHANDLE"],
                documentation: "Resets directory handle",
            },
        );

        signatures.insert(
            "telldir",
            BuiltinSignature {
                signatures: vec!["telldir DIRHANDLE"],
                documentation: "Returns directory position",
            },
        );

        signatures.insert(
            "seekdir",
            BuiltinSignature {
                signatures: vec!["seekdir DIRHANDLE, POS"],
                documentation: "Sets directory position",
            },
        );

        // ===== File Operations =====
        signatures.insert(
            "chdir",
            BuiltinSignature {
                signatures: vec!["chdir EXPR", "chdir"],
                documentation: "Changes the current working directory",
            },
        );

        signatures.insert(
            "chroot",
            BuiltinSignature {
                signatures: vec!["chroot FILENAME"],
                documentation: "Changes the root directory for the process",
            },
        );

        signatures.insert(
            "chmod",
            BuiltinSignature {
                signatures: vec!["chmod MODE, LIST"],
                documentation: "Changes file permissions",
            },
        );

        signatures.insert(
            "chown",
            BuiltinSignature {
                signatures: vec!["chown UID, GID, LIST"],
                documentation: "Changes file ownership",
            },
        );

        signatures.insert(
            "link",
            BuiltinSignature {
                signatures: vec!["link OLDFILE, NEWFILE"],
                documentation: "Creates hard link",
            },
        );

        signatures.insert(
            "symlink",
            BuiltinSignature {
                signatures: vec!["symlink OLDFILE, NEWFILE"],
                documentation: "Creates symbolic link",
            },
        );

        signatures.insert(
            "readlink",
            BuiltinSignature {
                signatures: vec!["readlink EXPR", "readlink"],
                documentation: "Reads symbolic link",
            },
        );

        signatures.insert(
            "rename",
            BuiltinSignature {
                signatures: vec!["rename OLDNAME, NEWNAME"],
                documentation: "Renames a file",
            },
        );

        signatures.insert(
            "unlink",
            BuiltinSignature {
                signatures: vec!["unlink LIST", "unlink"],
                documentation: "Deletes files",
            },
        );

        signatures.insert(
            "mkdir",
            BuiltinSignature {
                signatures: vec!["mkdir FILENAME, MODE", "mkdir FILENAME"],
                documentation: "Creates directory",
            },
        );

        signatures.insert(
            "rmdir",
            BuiltinSignature {
                signatures: vec!["rmdir FILENAME", "rmdir"],
                documentation: "Removes directory",
            },
        );

        // ===== Process Functions =====
        signatures.insert(
            "system",
            BuiltinSignature {
                signatures: vec!["system LIST", "system PROGRAM LIST"],
                documentation: "Executes system command",
            },
        );

        signatures.insert(
            "exec",
            BuiltinSignature {
                signatures: vec!["exec LIST", "exec PROGRAM LIST"],
                documentation: "Executes system command (never returns)",
            },
        );

        signatures.insert(
            "fork",
            BuiltinSignature { signatures: vec!["fork"], documentation: "Creates a child process" },
        );

        signatures.insert(
            "wait",
            BuiltinSignature { signatures: vec!["wait"], documentation: "Waits for child process" },
        );

        signatures.insert(
            "waitpid",
            BuiltinSignature {
                signatures: vec!["waitpid PID, FLAGS"],
                documentation: "Waits for specific child process",
            },
        );

        signatures.insert(
            "kill",
            BuiltinSignature {
                signatures: vec!["kill SIGNAL, LIST"],
                documentation: "Sends signal to processes",
            },
        );

        signatures.insert(
            "getpid",
            BuiltinSignature { signatures: vec!["getpid"], documentation: "Returns process ID" },
        );

        signatures.insert(
            "getppid",
            BuiltinSignature {
                signatures: vec!["getppid"],
                documentation: "Returns parent process ID",
            },
        );

        // ===== Time Functions =====
        signatures.insert(
            "time",
            BuiltinSignature { signatures: vec!["time"], documentation: "Returns current time" },
        );

        signatures.insert(
            "localtime",
            BuiltinSignature {
                signatures: vec!["localtime EXPR", "localtime"],
                documentation: "Converts time to local time",
            },
        );

        signatures.insert(
            "gmtime",
            BuiltinSignature {
                signatures: vec!["gmtime EXPR", "gmtime"],
                documentation: "Converts time to GMT",
            },
        );

        signatures.insert(
            "sleep",
            BuiltinSignature {
                signatures: vec!["sleep EXPR", "sleep"],
                documentation: "Sleeps for seconds",
            },
        );

        signatures.insert(
            "alarm",
            BuiltinSignature {
                signatures: vec!["alarm SECONDS", "alarm"],
                documentation: "Sets alarm signal",
            },
        );

        // ===== Mathematical Functions =====
        signatures.insert(
            "abs",
            BuiltinSignature {
                signatures: vec!["abs VALUE", "abs"],
                documentation: "Returns absolute value",
            },
        );

        signatures.insert(
            "atan2",
            BuiltinSignature {
                signatures: vec!["atan2 Y, X"],
                documentation: "Returns arctangent",
            },
        );

        signatures.insert(
            "cos",
            BuiltinSignature {
                signatures: vec!["cos EXPR", "cos"],
                documentation: "Returns cosine",
            },
        );

        signatures.insert(
            "sin",
            BuiltinSignature { signatures: vec!["sin EXPR", "sin"], documentation: "Returns sine" },
        );

        signatures.insert(
            "exp",
            BuiltinSignature {
                signatures: vec!["exp EXPR", "exp"],
                documentation: "Returns e raised to power",
            },
        );

        signatures.insert(
            "log",
            BuiltinSignature {
                signatures: vec!["log EXPR", "log"],
                documentation: "Returns natural logarithm",
            },
        );

        signatures.insert(
            "sqrt",
            BuiltinSignature {
                signatures: vec!["sqrt EXPR", "sqrt"],
                documentation: "Returns square root",
            },
        );

        signatures.insert(
            "int",
            BuiltinSignature {
                signatures: vec!["int EXPR", "int"],
                documentation: "Returns integer portion",
            },
        );

        signatures.insert(
            "rand",
            BuiltinSignature {
                signatures: vec!["rand EXPR", "rand"],
                documentation: "Returns random number",
            },
        );

        signatures.insert(
            "srand",
            BuiltinSignature {
                signatures: vec!["srand EXPR", "srand"],
                documentation: "Seeds random number generator",
            },
        );

        // ===== Type and Reference Functions =====
        signatures.insert(
            "ref",
            BuiltinSignature {
                signatures: vec!["ref EXPR", "ref"],
                documentation: "Returns type of reference",
            },
        );

        signatures.insert(
            "bless",
            BuiltinSignature {
                signatures: vec!["bless REF, CLASSNAME", "bless REF"],
                documentation: "Blesses reference into class",
            },
        );

        signatures.insert(
            "defined",
            BuiltinSignature {
                signatures: vec!["defined EXPR", "defined"],
                documentation: "Tests whether value is defined",
            },
        );

        signatures.insert(
            "undef",
            BuiltinSignature {
                signatures: vec!["undef EXPR", "undef"],
                documentation: "Undefines a value",
            },
        );

        signatures.insert(
            "scalar",
            BuiltinSignature {
                signatures: vec!["scalar EXPR"],
                documentation: "Forces scalar context",
            },
        );

        signatures.insert(
            "wantarray",
            BuiltinSignature {
                signatures: vec!["wantarray"],
                documentation: "Returns context of subroutine call",
            },
        );

        // ===== Control Flow =====
        signatures.insert(
            "die",
            BuiltinSignature {
                signatures: vec!["die LIST", "die"],
                documentation: "Raises an exception",
            },
        );

        signatures.insert(
            "warn",
            BuiltinSignature {
                signatures: vec!["warn LIST", "warn"],
                documentation: "Prints warning message",
            },
        );

        signatures.insert(
            "exit",
            BuiltinSignature {
                signatures: vec!["exit EXPR", "exit"],
                documentation: "Exits the program",
            },
        );

        signatures.insert(
            "return",
            BuiltinSignature {
                signatures: vec!["return LIST", "return"],
                documentation: "Returns from subroutine",
            },
        );

        signatures.insert(
            "next",
            BuiltinSignature {
                signatures: vec!["next LABEL", "next"],
                documentation: "Starts next iteration of loop",
            },
        );

        signatures.insert(
            "last",
            BuiltinSignature {
                signatures: vec!["last LABEL", "last"],
                documentation: "Exits loop",
            },
        );

        signatures.insert(
            "redo",
            BuiltinSignature {
                signatures: vec!["redo LABEL", "redo"],
                documentation: "Restarts current iteration",
            },
        );

        signatures.insert(
            "goto",
            BuiltinSignature {
                signatures: vec!["goto LABEL", "goto EXPR", "goto &NAME"],
                documentation: "Goes to label or subroutine",
            },
        );

        // ===== Module Functions =====
        signatures.insert(
            "require",
            BuiltinSignature {
                signatures: vec!["require VERSION", "require MODULE", "require EXPR", "require"],
                documentation: "Loads module or file",
            },
        );

        signatures.insert(
            "use",
            BuiltinSignature {
                signatures: vec![
                    "use MODULE VERSION LIST",
                    "use MODULE VERSION",
                    "use MODULE LIST",
                    "use MODULE",
                    "use VERSION",
                ],
                documentation: "Imports module",
            },
        );

        signatures.insert(
            "no",
            BuiltinSignature {
                signatures: vec![
                    "no MODULE VERSION LIST",
                    "no MODULE VERSION",
                    "no MODULE LIST",
                    "no MODULE",
                    "no VERSION",
                ],
                documentation: "Unimports module",
            },
        );

        signatures.insert(
            "import",
            BuiltinSignature {
                signatures: vec!["import MODULE LIST"],
                documentation: "Imports symbols from module",
            },
        );

        signatures.insert(
            "unimport",
            BuiltinSignature {
                signatures: vec!["unimport MODULE LIST"],
                documentation: "Unimports symbols from module",
            },
        );

        // ===== Package Functions =====
        signatures.insert(
            "package",
            BuiltinSignature {
                signatures: vec!["package NAMESPACE VERSION", "package NAMESPACE"],
                documentation: "Declares package namespace",
            },
        );

        signatures.insert(
            "caller",
            BuiltinSignature {
                signatures: vec!["caller EXPR", "caller"],
                documentation: "Returns context of current subroutine call",
            },
        );

        // ===== Eval and Do =====
        signatures.insert(
            "eval",
            BuiltinSignature {
                signatures: vec!["eval EXPR", "eval BLOCK"],
                documentation: "Evaluates code",
            },
        );

        signatures.insert(
            "do",
            BuiltinSignature {
                signatures: vec!["do FILENAME", "do BLOCK"],
                documentation: "Executes file or block",
            },
        );

        // ===== Tied Variables =====
        signatures.insert(
            "tie",
            BuiltinSignature {
                signatures: vec!["tie VARIABLE, CLASSNAME, LIST"],
                documentation: "Binds variable to class",
            },
        );

        signatures.insert(
            "tied",
            BuiltinSignature {
                signatures: vec!["tied VARIABLE"],
                documentation: "Returns object tied to variable",
            },
        );

        signatures.insert(
            "untie",
            BuiltinSignature {
                signatures: vec!["untie VARIABLE"],
                documentation: "Breaks binding on variable",
            },
        );

        // ===== Socket Functions =====
        signatures.insert(
            "socket",
            BuiltinSignature {
                signatures: vec!["socket SOCKET, DOMAIN, TYPE, PROTOCOL"],
                documentation: "Creates a socket",
            },
        );

        signatures.insert(
            "bind",
            BuiltinSignature {
                signatures: vec!["bind SOCKET, NAME"],
                documentation: "Binds address to socket",
            },
        );

        signatures.insert(
            "listen",
            BuiltinSignature {
                signatures: vec!["listen SOCKET, QUEUESIZE"],
                documentation: "Listens for connections",
            },
        );

        signatures.insert(
            "accept",
            BuiltinSignature {
                signatures: vec!["accept NEWSOCKET, GENERICSOCKET"],
                documentation: "Accepts socket connection",
            },
        );

        signatures.insert(
            "connect",
            BuiltinSignature {
                signatures: vec!["connect SOCKET, NAME"],
                documentation: "Connects to socket",
            },
        );

        signatures.insert(
            "shutdown",
            BuiltinSignature {
                signatures: vec!["shutdown SOCKET, HOW"],
                documentation: "Shuts down socket",
            },
        );

        signatures.insert(
            "send",
            BuiltinSignature {
                signatures: vec!["send SOCKET, MSG, FLAGS, TO", "send SOCKET, MSG, FLAGS"],
                documentation: "Sends message on socket",
            },
        );

        signatures.insert(
            "recv",
            BuiltinSignature {
                signatures: vec!["recv SOCKET, SCALAR, LENGTH, FLAGS"],
                documentation: "Receives message from socket",
            },
        );

        signatures.insert(
            "getsockopt",
            BuiltinSignature {
                signatures: vec!["getsockopt SOCKET, LEVEL, OPTNAME"],
                documentation: "Gets socket options",
            },
        );

        signatures.insert(
            "setsockopt",
            BuiltinSignature {
                signatures: vec!["setsockopt SOCKET, LEVEL, OPTNAME, OPTVAL"],
                documentation: "Sets socket options",
            },
        );

        signatures.insert(
            "socketpair",
            BuiltinSignature {
                signatures: vec!["socketpair SOCKET1, SOCKET2, DOMAIN, TYPE, PROTOCOL"],
                documentation: "Creates a pair of connected sockets",
            },
        );

        signatures.insert(
            "sockatmark",
            BuiltinSignature {
                signatures: vec!["sockatmark SOCKET"],
                documentation: "Tests whether a socket is at an out-of-band mark",
            },
        );

        signatures.insert(
            "getpeername",
            BuiltinSignature {
                signatures: vec!["getpeername SOCKET"],
                documentation: "Returns packed sockaddr address of other end of socket connection",
            },
        );

        signatures.insert(
            "getsockname",
            BuiltinSignature {
                signatures: vec!["getsockname SOCKET"],
                documentation: "Returns packed sockaddr address of this end of socket connection",
            },
        );

        // ===== I/O Control Functions =====
        signatures.insert(
            "pipe",
            BuiltinSignature {
                signatures: vec!["pipe READHANDLE, WRITEHANDLE"],
                documentation: "Opens a pair of connected pipes",
            },
        );

        signatures.insert(
            "fcntl",
            BuiltinSignature {
                signatures: vec!["fcntl FILEHANDLE, FUNCTION, SCALAR"],
                documentation: "File control system call",
            },
        );

        signatures.insert(
            "ioctl",
            BuiltinSignature {
                signatures: vec!["ioctl FILEHANDLE, FUNCTION, SCALAR"],
                documentation: "System-dependent device control system call",
            },
        );

        signatures.insert(
            "flock",
            BuiltinSignature {
                signatures: vec!["flock FILEHANDLE, OPERATION"],
                documentation: "Locks or unlocks file",
            },
        );

        signatures.insert(
            "select",
            BuiltinSignature {
                signatures: vec![
                    "select FILEHANDLE",
                    "select RBITS, WBITS, EBITS, TIMEOUT",
                    "select",
                ],
                documentation: "Sets default filehandle for output or performs select system call",
            },
        );

        signatures.insert(
            "getc",
            BuiltinSignature {
                signatures: vec!["getc FILEHANDLE", "getc"],
                documentation: "Gets next character from filehandle",
            },
        );

        signatures.insert(
            "binmode",
            BuiltinSignature {
                signatures: vec!["binmode FILEHANDLE, LAYER", "binmode FILEHANDLE"],
                documentation: "Sets binary mode on filehandle",
            },
        );

        signatures.insert(
            "fileno",
            BuiltinSignature {
                signatures: vec!["fileno FILEHANDLE"],
                documentation: "Returns file descriptor number",
            },
        );

        // ===== Network Functions =====
        signatures.insert(
            "gethostbyname",
            BuiltinSignature {
                signatures: vec!["gethostbyname NAME"],
                documentation: "Returns host information by name",
            },
        );

        signatures.insert(
            "gethostbyaddr",
            BuiltinSignature {
                signatures: vec!["gethostbyaddr ADDR, ADDRTYPE"],
                documentation: "Returns host information by address",
            },
        );

        signatures.insert(
            "getnetbyname",
            BuiltinSignature {
                signatures: vec!["getnetbyname NAME"],
                documentation: "Returns network information by name",
            },
        );

        signatures.insert(
            "getnetbyaddr",
            BuiltinSignature {
                signatures: vec!["getnetbyaddr ADDR, ADDRTYPE"],
                documentation: "Returns network information by address",
            },
        );

        signatures.insert(
            "getprotobyname",
            BuiltinSignature {
                signatures: vec!["getprotobyname NAME"],
                documentation: "Returns protocol information by name",
            },
        );

        signatures.insert(
            "getprotobynumber",
            BuiltinSignature {
                signatures: vec!["getprotobynumber NUMBER"],
                documentation: "Returns protocol information by number",
            },
        );

        signatures.insert(
            "getservbyname",
            BuiltinSignature {
                signatures: vec!["getservbyname NAME, PROTO"],
                documentation: "Returns service information by name",
            },
        );

        signatures.insert(
            "getservbyport",
            BuiltinSignature {
                signatures: vec!["getservbyport PORT, PROTO"],
                documentation: "Returns service information by port",
            },
        );

        signatures.insert(
            "gethostent",
            BuiltinSignature {
                signatures: vec!["gethostent"],
                documentation: "Returns next host from hosts file",
            },
        );

        signatures.insert(
            "getnetent",
            BuiltinSignature {
                signatures: vec!["getnetent"],
                documentation: "Returns next network from networks file",
            },
        );

        signatures.insert(
            "getprotoent",
            BuiltinSignature {
                signatures: vec!["getprotoent"],
                documentation: "Returns next protocol from protocols file",
            },
        );

        signatures.insert(
            "getservent",
            BuiltinSignature {
                signatures: vec!["getservent"],
                documentation: "Returns next service from services file",
            },
        );

        signatures.insert(
            "sethostent",
            BuiltinSignature {
                signatures: vec!["sethostent STAYOPEN"],
                documentation: "Opens or rewinds hosts file",
            },
        );

        signatures.insert(
            "setnetent",
            BuiltinSignature {
                signatures: vec!["setnetent STAYOPEN"],
                documentation: "Opens or rewinds networks file",
            },
        );

        signatures.insert(
            "setprotoent",
            BuiltinSignature {
                signatures: vec!["setprotoent STAYOPEN"],
                documentation: "Opens or rewinds protocols file",
            },
        );

        signatures.insert(
            "setservent",
            BuiltinSignature {
                signatures: vec!["setservent STAYOPEN"],
                documentation: "Opens or rewinds services file",
            },
        );

        signatures.insert(
            "endhostent",
            BuiltinSignature { signatures: vec!["endhostent"], documentation: "Closes hosts file" },
        );

        signatures.insert(
            "endnetent",
            BuiltinSignature {
                signatures: vec!["endnetent"],
                documentation: "Closes networks file",
            },
        );

        signatures.insert(
            "endprotoent",
            BuiltinSignature {
                signatures: vec!["endprotoent"],
                documentation: "Closes protocols file",
            },
        );

        signatures.insert(
            "endservent",
            BuiltinSignature {
                signatures: vec!["endservent"],
                documentation: "Closes services file",
            },
        );

        // ===== User and Group Functions =====
        signatures.insert(
            "getpwnam",
            BuiltinSignature {
                signatures: vec!["getpwnam NAME"],
                documentation: "Returns password entry by name",
            },
        );

        signatures.insert(
            "getpwuid",
            BuiltinSignature {
                signatures: vec!["getpwuid UID"],
                documentation: "Returns password entry by uid",
            },
        );

        signatures.insert(
            "getpwent",
            BuiltinSignature {
                signatures: vec!["getpwent"],
                documentation: "Returns next password entry",
            },
        );

        signatures.insert(
            "setpwent",
            BuiltinSignature {
                signatures: vec!["setpwent"],
                documentation: "Rewinds password file",
            },
        );

        signatures.insert(
            "endpwent",
            BuiltinSignature {
                signatures: vec!["endpwent"],
                documentation: "Closes password file",
            },
        );

        signatures.insert(
            "getgrnam",
            BuiltinSignature {
                signatures: vec!["getgrnam NAME"],
                documentation: "Returns group entry by name",
            },
        );

        signatures.insert(
            "getgrgid",
            BuiltinSignature {
                signatures: vec!["getgrgid GID"],
                documentation: "Returns group entry by gid",
            },
        );

        signatures.insert(
            "getgrent",
            BuiltinSignature {
                signatures: vec!["getgrent"],
                documentation: "Returns next group entry",
            },
        );

        signatures.insert(
            "setgrent",
            BuiltinSignature { signatures: vec!["setgrent"], documentation: "Rewinds group file" },
        );

        signatures.insert(
            "endgrent",
            BuiltinSignature { signatures: vec!["endgrent"], documentation: "Closes group file" },
        );

        signatures.insert(
            "getlogin",
            BuiltinSignature {
                signatures: vec!["getlogin"],
                documentation: "Returns current login name",
            },
        );

        signatures.insert(
            "getuid",
            BuiltinSignature {
                signatures: vec!["getuid"],
                documentation: "Returns real user ID",
            },
        );

        signatures.insert(
            "geteuid",
            BuiltinSignature {
                signatures: vec!["geteuid"],
                documentation: "Returns effective user ID",
            },
        );

        signatures.insert(
            "getgid",
            BuiltinSignature {
                signatures: vec!["getgid"],
                documentation: "Returns real group ID",
            },
        );

        signatures.insert(
            "getegid",
            BuiltinSignature {
                signatures: vec!["getegid"],
                documentation: "Returns effective group ID",
            },
        );

        signatures.insert(
            "getgroups",
            BuiltinSignature {
                signatures: vec!["getgroups"],
                documentation: "Returns supplementary group IDs",
            },
        );

        signatures.insert(
            "setuid",
            BuiltinSignature {
                signatures: vec!["setuid UID"],
                documentation: "Sets real user ID",
            },
        );

        signatures.insert(
            "seteuid",
            BuiltinSignature {
                signatures: vec!["seteuid UID"],
                documentation: "Sets effective user ID",
            },
        );

        signatures.insert(
            "setgid",
            BuiltinSignature {
                signatures: vec!["setgid GID"],
                documentation: "Sets real group ID",
            },
        );

        signatures.insert(
            "setegid",
            BuiltinSignature {
                signatures: vec!["setegid GID"],
                documentation: "Sets effective group ID",
            },
        );

        signatures.insert(
            "setgroups",
            BuiltinSignature {
                signatures: vec!["setgroups LIST"],
                documentation: "Sets supplementary group IDs",
            },
        );

        // ===== Miscellaneous System Functions =====
        signatures.insert(
            "umask",
            BuiltinSignature {
                signatures: vec!["umask EXPR", "umask"],
                documentation: "Sets file creation mode mask",
            },
        );

        signatures.insert(
            "truncate",
            BuiltinSignature {
                signatures: vec!["truncate FILEHANDLE, LENGTH", "truncate EXPR, LENGTH"],
                documentation: "Truncates file to specified length",
            },
        );

        signatures.insert(
            "glob",
            BuiltinSignature {
                signatures: vec!["glob EXPR", "glob"],
                documentation: "Returns list of filenames matching pattern",
            },
        );

        signatures.insert(
            "setpgrp",
            BuiltinSignature {
                signatures: vec!["setpgrp PID, PGRP"],
                documentation: "Sets process group",
            },
        );

        signatures.insert(
            "getpgrp",
            BuiltinSignature {
                signatures: vec!["getpgrp PID"],
                documentation: "Returns process group",
            },
        );

        signatures.insert(
            "syscall",
            BuiltinSignature {
                signatures: vec!["syscall NUMBER, LIST"],
                documentation: "Invokes a system call",
            },
        );

        signatures.insert(
            "times",
            BuiltinSignature {
                signatures: vec!["times"],
                documentation: "Returns elapsed time for process and children",
            },
        );

        signatures.insert(
            "getpriority",
            BuiltinSignature {
                signatures: vec!["getpriority WHICH, WHO"],
                documentation: "Returns current priority for process, process group, or user",
            },
        );

        signatures.insert(
            "setpriority",
            BuiltinSignature {
                signatures: vec!["setpriority WHICH, WHO, PRIORITY"],
                documentation: "Sets priority for process, process group, or user",
            },
        );

        // ===== Pack/Unpack =====
        signatures.insert(
            "pack",
            BuiltinSignature {
                signatures: vec!["pack TEMPLATE, LIST"],
                documentation: "Packs list into binary",
            },
        );

        signatures.insert(
            "unpack",
            BuiltinSignature {
                signatures: vec!["unpack TEMPLATE, EXPR"],
                documentation: "Unpacks binary into list",
            },
        );

        // ===== Regular Expression =====
        signatures.insert(
            "study",
            BuiltinSignature {
                signatures: vec!["study SCALAR", "study"],
                documentation: "Optimizes string for pattern matching",
            },
        );

        signatures.insert(
            "pos",
            BuiltinSignature {
                signatures: vec!["pos SCALAR", "pos"],
                documentation: "Returns or sets match position",
            },
        );

        signatures.insert(
            "reset",
            BuiltinSignature {
                signatures: vec!["reset EXPR", "reset"],
                documentation: "Resets variables and searches",
            },
        );

        // ===== Format Functions =====
        signatures.insert(
            "formline",
            BuiltinSignature {
                signatures: vec!["formline PICTURE, LIST"],
                documentation: "Internal function for formats",
            },
        );

        signatures.insert(
            "format",
            BuiltinSignature {
                signatures: vec!["format NAME ="],
                documentation: "Declares format",
            },
        );

        // ===== File Test Operators =====
        macro_rules! file_test {
            ($op:literal) => {
                signatures.insert(
                    $op,
                    BuiltinSignature {
                        signatures: vec![concat!($op, " FILE"), $op],
                        documentation: "File test operator",
                    },
                );
            };
        }

        file_test!("-e");
        file_test!("-f");
        file_test!("-d");
        file_test!("-r");
        file_test!("-w");
        file_test!("-x");
        file_test!("-o");
        file_test!("-R");
        file_test!("-W");
        file_test!("-X");
        file_test!("-O");
        file_test!("-z");
        file_test!("-s");
        file_test!("-l");
        file_test!("-p");
        file_test!("-S");
        file_test!("-b");
        file_test!("-c");
        file_test!("-t");
        file_test!("-u");
        file_test!("-g");
        file_test!("-k");
        file_test!("-T");
        file_test!("-B");
        file_test!("-M");
        file_test!("-A");
        file_test!("-C");

        // ===== Miscellaneous =====
        signatures.insert(
            "dump",
            BuiltinSignature {
                signatures: vec!["dump LABEL", "dump"],
                documentation: "Creates core dump",
            },
        );

        signatures.insert(
            "dbmopen",
            BuiltinSignature {
                signatures: vec!["dbmopen HASH, DBNAME, MASK"],
                documentation: "Opens DBM file (deprecated, use tie instead)",
            },
        );

        signatures.insert(
            "dbmclose",
            BuiltinSignature {
                signatures: vec!["dbmclose HASH"],
                documentation: "Closes DBM file (deprecated, use untie instead)",
            },
        );

        signatures.insert(
            "vec",
            BuiltinSignature {
                signatures: vec!["vec EXPR, OFFSET, BITS"],
                documentation: "Accesses bit vector",
            },
        );

        signatures.insert(
            "prototype",
            BuiltinSignature {
                signatures: vec!["prototype FUNCTION"],
                documentation: "Returns function prototype",
            },
        );

        signatures.insert(
            "lock",
            BuiltinSignature {
                signatures: vec!["lock THING"],
                documentation: "Locks shared variable",
            },
        );

        signatures
    })
}
