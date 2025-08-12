//! Comprehensive built-in function signatures for Perl
//!
//! This module provides complete signature information for all Perl built-in functions

use std::collections::HashMap;

/// Built-in function signature
pub struct BuiltinSignature {
    pub signatures: Vec<&'static str>,
    pub documentation: &'static str,
}

/// Create comprehensive built-in function signatures
pub fn create_builtin_signatures() -> HashMap<&'static str, BuiltinSignature> {
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
        BuiltinSignature {
            signatures: vec!["fork"],
            documentation: "Creates a child process",
        },
    );

    signatures.insert(
        "wait",
        BuiltinSignature {
            signatures: vec!["wait"],
            documentation: "Waits for child process",
        },
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
        BuiltinSignature {
            signatures: vec!["getpid"],
            documentation: "Returns process ID",
        },
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
        BuiltinSignature {
            signatures: vec!["time"],
            documentation: "Returns current time",
        },
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
        BuiltinSignature {
            signatures: vec!["sin EXPR", "sin"],
            documentation: "Returns sine",
        },
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
            signatures: vec![
                "require VERSION",
                "require MODULE",
                "require EXPR",
                "require",
            ],
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

    // ===== Miscellaneous =====
    signatures.insert(
        "dump",
        BuiltinSignature {
            signatures: vec!["dump LABEL", "dump"],
            documentation: "Creates core dump",
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
}
