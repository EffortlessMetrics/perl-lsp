//! Perl built-in function documentation and classification.

/// Documentation entry for a Perl built-in function.
///
/// Provides signature and description information for display in hover tooltips.
pub struct BuiltinDoc {
    /// Function signature showing calling conventions
    pub signature: &'static str,
    /// Brief description of what the function does
    pub description: &'static str,
}

/// Check if a function name is a Perl control-flow keyword.
///
/// Returns `true` if the name is a control-flow keyword like `next`, `last`, etc.
pub(super) fn is_control_keyword(name: &str) -> bool {
    matches!(name, "next" | "last" | "redo" | "goto" | "return" | "exit" | "die")
}

/// Check if a function name is a Perl built-in.
///
/// Returns `true` if the name matches a known Perl built-in function.
pub(super) fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "say"
            | "printf"
            | "sprintf"
            | "open"
            | "close"
            | "read"
            | "write"
            | "chomp"
            | "chop"
            | "split"
            | "join"
            | "push"
            | "pop"
            | "shift"
            | "unshift"
            | "sort"
            | "reverse"
            | "map"
            | "grep"
            | "length"
            | "substr"
            | "index"
            | "rindex"
            | "lc"
            | "uc"
            | "lcfirst"
            | "ucfirst"
            | "defined"
            | "undef"
            | "ref"
            | "blessed"
            | "die"
            | "warn"
            | "eval"
            | "require"
            | "use"
            | "return"
            | "next"
            | "last"
            | "redo"
            | "goto" // ... many more
    )
}

/// Check if an operator is a file test operator.
///
/// File test operators in Perl are unary operators that test file properties:
/// -e (exists), -d (directory), -f (file), -r (readable), -w (writable), etc.
pub(super) fn is_file_test_operator(op: &str) -> bool {
    matches!(
        op,
        "-e" | "-d"
            | "-f"
            | "-r"
            | "-w"
            | "-x"
            | "-s"
            | "-z"
            | "-T"
            | "-B"
            | "-M"
            | "-A"
            | "-C"
            | "-l"
            | "-p"
            | "-S"
            | "-u"
            | "-g"
            | "-k"
            | "-t"
            | "-O"
            | "-G"
            | "-R"
            | "-b"
            | "-c"
    )
}

/// Get documentation for a Perl built-in function.
///
/// Returns signature and description for known built-in functions,
/// or `None` if documentation is not available.
///
/// This is also used by the LSP hover handler to show builtin docs when the
/// semantic analyzer has no symbol-level hit (e.g. bare-word builtins in
/// fallback path).
pub fn get_builtin_documentation(name: &str) -> Option<BuiltinDoc> {
    match name {
        // I/O
        "print" => Some(BuiltinDoc {
            signature: "print FILEHANDLE LIST\nprint LIST\nprint",
            description: "Prints a string or list of strings. If FILEHANDLE is omitted, prints to the last selected output handle (STDOUT by default).",
        }),
        "say" => Some(BuiltinDoc {
            signature: "say FILEHANDLE LIST\nsay LIST\nsay",
            description: "Like print, but appends a newline to the output.",
        }),
        "printf" => Some(BuiltinDoc {
            signature: "printf FILEHANDLE FORMAT, LIST\nprintf FORMAT, LIST",
            description: "Prints a formatted string to FILEHANDLE (default STDOUT).",
        }),
        "sprintf" => Some(BuiltinDoc {
            signature: "sprintf FORMAT, LIST",
            description: "Returns a formatted string (like C sprintf). Does not print.",
        }),
        "open" => Some(BuiltinDoc {
            signature: "open FILEHANDLE, MODE, EXPR\nopen FILEHANDLE, EXPR\nopen FILEHANDLE",
            description: "Opens the file whose filename is given by EXPR, and associates it with FILEHANDLE.",
        }),
        "close" => Some(BuiltinDoc {
            signature: "close FILEHANDLE\nclose",
            description: "Closes the file, socket, or pipe associated with FILEHANDLE.",
        }),
        "read" => Some(BuiltinDoc {
            signature: "read FILEHANDLE, SCALAR, LENGTH, OFFSET\nread FILEHANDLE, SCALAR, LENGTH",
            description: "Reads LENGTH bytes of data into SCALAR from FILEHANDLE. Returns the number of bytes read, or undef on error.",
        }),
        "write" => Some(BuiltinDoc {
            signature: "write FILEHANDLE\nwrite",
            description: "Writes a formatted record to FILEHANDLE using the format associated with it.",
        }),
        "seek" => Some(BuiltinDoc {
            signature: "seek FILEHANDLE, POSITION, WHENCE",
            description: "Sets the position for a filehandle. WHENCE: 0=start, 1=current, 2=end.",
        }),
        "tell" => Some(BuiltinDoc {
            signature: "tell FILEHANDLE\ntell",
            description: "Returns the current position in bytes for FILEHANDLE.",
        }),
        "eof" => Some(BuiltinDoc {
            signature: "eof FILEHANDLE\neof()\neof",
            description: "Returns true if the next read on FILEHANDLE would return end of file.",
        }),
        "binmode" => Some(BuiltinDoc {
            signature: "binmode FILEHANDLE, LAYER\nbinmode FILEHANDLE",
            description: "Sets binary mode on FILEHANDLE, or specifies an I/O layer.",
        }),
        "truncate" => Some(BuiltinDoc {
            signature: "truncate FILEHANDLE, LENGTH",
            description: "Truncates the file at the given LENGTH.",
        }),

        // String functions
        "chomp" => Some(BuiltinDoc {
            signature: "chomp VARIABLE\nchomp LIST\nchomp",
            description: "Removes the trailing newline from VARIABLE. Returns the number of characters removed.",
        }),
        "chop" => Some(BuiltinDoc {
            signature: "chop VARIABLE\nchop LIST\nchop",
            description: "Removes and returns the last character from VARIABLE.",
        }),
        "length" => Some(BuiltinDoc {
            signature: "length EXPR\nlength",
            description: "Returns the length in characters of the value of EXPR.",
        }),
        "substr" => Some(BuiltinDoc {
            signature: "substr EXPR, OFFSET, LENGTH, REPLACEMENT\nsubstr EXPR, OFFSET, LENGTH\nsubstr EXPR, OFFSET",
            description: "Extracts a substring out of EXPR and returns it. With REPLACEMENT, replaces the substring in-place.",
        }),
        "index" => Some(BuiltinDoc {
            signature: "index STR, SUBSTR, POSITION\nindex STR, SUBSTR",
            description: "Returns the position of the first occurrence of SUBSTR in STR at or after POSITION. Returns -1 if not found.",
        }),
        "rindex" => Some(BuiltinDoc {
            signature: "rindex STR, SUBSTR, POSITION\nrindex STR, SUBSTR",
            description: "Returns the position of the last occurrence of SUBSTR in STR at or before POSITION.",
        }),
        "lc" => Some(BuiltinDoc {
            signature: "lc EXPR\nlc",
            description: "Returns a lowercased version of EXPR (or $_ if omitted).",
        }),
        "uc" => Some(BuiltinDoc {
            signature: "uc EXPR\nuc",
            description: "Returns an uppercased version of EXPR (or $_ if omitted).",
        }),
        "lcfirst" => Some(BuiltinDoc {
            signature: "lcfirst EXPR\nlcfirst",
            description: "Returns EXPR with the first character lowercased.",
        }),
        "ucfirst" => Some(BuiltinDoc {
            signature: "ucfirst EXPR\nucfirst",
            description: "Returns EXPR with the first character uppercased.",
        }),
        "chr" => Some(BuiltinDoc {
            signature: "chr NUMBER\nchr",
            description: "Returns the character represented by NUMBER in the character set.",
        }),
        "ord" => Some(BuiltinDoc {
            signature: "ord EXPR\nord",
            description: "Returns the numeric value of the first character of EXPR.",
        }),
        "hex" => Some(BuiltinDoc {
            signature: "hex EXPR\nhex",
            description: "Interprets EXPR as a hex string and returns the corresponding numeric value.",
        }),
        "oct" => Some(BuiltinDoc {
            signature: "oct EXPR\noct",
            description: "Interprets EXPR as an octal string and returns the corresponding value. Handles 0x, 0b, and 0 prefixes.",
        }),
        "quotemeta" => Some(BuiltinDoc {
            signature: "quotemeta EXPR\nquotemeta",
            description: "Returns EXPR with all non-alphanumeric characters backslashed (escaped for regex).",
        }),
        "join" => Some(BuiltinDoc {
            signature: "join EXPR, LIST",
            description: "Joins the separate strings of LIST into a single string with fields separated by EXPR, and returns that string.",
        }),
        "split" => Some(BuiltinDoc {
            signature: "split /PATTERN/, EXPR, LIMIT\nsplit /PATTERN/, EXPR\nsplit /PATTERN/\nsplit",
            description: "Splits the string EXPR into a list of strings and returns the list. If LIMIT is specified, splits into at most that many fields.",
        }),

        // Array/List
        "push" => Some(BuiltinDoc {
            signature: "push ARRAY, LIST",
            description: "Appends one or more values to the end of ARRAY. Returns the number of elements in the resulting array.",
        }),
        "pop" => Some(BuiltinDoc {
            signature: "pop ARRAY\npop",
            description: "Removes and returns the last element of ARRAY.",
        }),
        "shift" => Some(BuiltinDoc {
            signature: "shift ARRAY\nshift",
            description: "Removes and returns the first element of ARRAY, shortening the array by 1.",
        }),
        "unshift" => Some(BuiltinDoc {
            signature: "unshift ARRAY, LIST",
            description: "Prepends LIST to the front of ARRAY. Returns the number of elements in the new array.",
        }),
        "splice" => Some(BuiltinDoc {
            signature: "splice ARRAY, OFFSET, LENGTH, LIST\nsplice ARRAY, OFFSET, LENGTH\nsplice ARRAY, OFFSET\nsplice ARRAY",
            description: "Removes LENGTH elements from ARRAY starting at OFFSET, replacing them with LIST. Returns the removed elements.",
        }),
        "sort" => Some(BuiltinDoc {
            signature: "sort SUBNAME LIST\nsort BLOCK LIST\nsort LIST",
            description: "Sorts LIST and returns the sorted list. BLOCK or SUBNAME provides a custom comparison function using $a and $b.",
        }),
        "reverse" => Some(BuiltinDoc {
            signature: "reverse LIST",
            description: "In list context, returns LIST in reverse order. In scalar context, returns a string with characters reversed.",
        }),
        "map" => Some(BuiltinDoc {
            signature: "map BLOCK LIST\nmap EXPR, LIST",
            description: "Evaluates the BLOCK or EXPR for each element of LIST (locally setting $_ to each element) and composes a list of the results.",
        }),
        "grep" => Some(BuiltinDoc {
            signature: "grep BLOCK LIST\ngrep EXPR, LIST",
            description: "Evaluates BLOCK or EXPR for each element of LIST and returns the list of elements for which the expression is true.",
        }),
        "scalar" => Some(BuiltinDoc {
            signature: "scalar EXPR",
            description: "Forces EXPR to be interpreted in scalar context and returns the value of EXPR.",
        }),
        "wantarray" => Some(BuiltinDoc {
            signature: "wantarray",
            description: "Returns true if the currently executing subroutine is expected to return a list value.",
        }),

        // Hash
        "keys" => Some(BuiltinDoc {
            signature: "keys HASH\nkeys ARRAY",
            description: "Returns a list of all the keys of the named hash, or the indices of an array.",
        }),
        "values" => Some(BuiltinDoc {
            signature: "values HASH\nvalues ARRAY",
            description: "Returns a list of all the values of the named hash, or values of an array.",
        }),
        "each" => Some(BuiltinDoc {
            signature: "each HASH\neach ARRAY",
            description: "Returns a two-element list of the next (key, value) pair from the hash.",
        }),
        "exists" => Some(BuiltinDoc {
            signature: "exists EXPR",
            description: "Returns true if the specified hash key or array element exists, even if its value is undef.",
        }),
        "delete" => Some(BuiltinDoc {
            signature: "delete EXPR",
            description: "Deletes the specified keys and their associated values from a hash, or elements from an array.",
        }),
        "defined" => Some(BuiltinDoc {
            signature: "defined EXPR\ndefined",
            description: "Returns true if EXPR has a value other than undef.",
        }),
        "undef" => Some(BuiltinDoc {
            signature: "undef EXPR\nundef",
            description: "Undefines the value of EXPR. Can be used on scalars, arrays, hashes, subroutines, and typeglobs.",
        }),

        // References and OO
        "ref" => Some(BuiltinDoc {
            signature: "ref EXPR\nref",
            description: "Returns a string indicating the type of reference EXPR is, or empty string if not a reference. E.g. HASH, ARRAY, SCALAR, CODE.",
        }),
        "bless" => Some(BuiltinDoc {
            signature: "bless REF, CLASSNAME\nbless REF",
            description: "Associates the referent of REF with package CLASSNAME (or current package). Returns the reference.",
        }),
        "blessed" => Some(BuiltinDoc {
            signature: "blessed EXPR",
            description: "Returns the name of the package EXPR is blessed into, or undef if EXPR is not a blessed reference. From Scalar::Util.",
        }),
        "tie" => Some(BuiltinDoc {
            signature: "tie VARIABLE, CLASSNAME, LIST",
            description: "Binds a variable to a package class that provides the implementation for the variable.",
        }),
        "untie" => Some(BuiltinDoc {
            signature: "untie VARIABLE",
            description: "Breaks the binding between a variable and its package.",
        }),
        "tied" => Some(BuiltinDoc {
            signature: "tied VARIABLE",
            description: "Returns a reference to the object underlying VARIABLE if it is tied, or undef if not.",
        }),

        // Control flow
        "die" => Some(BuiltinDoc {
            signature: "die LIST",
            description: "Raises an exception. The error is stored in $@ and can be caught with eval {}. If LIST does not end in a newline, the current script file and line number are appended. In modules, prefer Carp::croak to report the error from the caller's perspective.",
        }),
        "warn" => Some(BuiltinDoc {
            signature: "warn LIST",
            description: "Prints a warning message to STDERR without throwing an exception. In modules, prefer Carp::carp to report the warning from the caller's perspective.",
        }),
        "eval" => Some(BuiltinDoc {
            signature: "eval BLOCK\neval EXPR",
            description: "Evaluates EXPR or BLOCK and traps any errors, making them available in $@.",
        }),
        "return" => Some(BuiltinDoc {
            signature: "return EXPR\nreturn",
            description: "Returns from a subroutine with the value of EXPR.",
        }),
        "next" => Some(BuiltinDoc {
            signature: "next LABEL\nnext",
            description: "Starts the next iteration of the loop (like C 'continue').",
        }),
        "last" => Some(BuiltinDoc {
            signature: "last LABEL\nlast",
            description: "Exits the loop immediately (like C 'break').",
        }),
        "redo" => Some(BuiltinDoc {
            signature: "redo LABEL\nredo",
            description: "Restarts the loop block without re-evaluating the condition.",
        }),
        "goto" => Some(BuiltinDoc {
            signature: "goto LABEL\ngoto EXPR\ngoto &NAME",
            description: "Transfers control to the named label, computed label, or substitutes a call to the named subroutine.",
        }),
        "caller" => Some(BuiltinDoc {
            signature: "caller EXPR\ncaller",
            description: "Returns information about the calling subroutine's context. Returns (package, filename, line) in list context.",
        }),
        "exit" => Some(BuiltinDoc {
            signature: "exit EXPR\nexit",
            description: "Exits the program with status EXPR (default 0). Calls END blocks and DESTROY methods before exit.",
        }),

        // Modules and loading
        "require" => Some(BuiltinDoc {
            signature: "require EXPR\nrequire",
            description: "Loads a library module at runtime. Raises an exception on failure.",
        }),
        "use" => Some(BuiltinDoc {
            signature: "use Module VERSION LIST\nuse Module VERSION\nuse Module LIST\nuse Module",
            description: "Loads and imports a module at compile time. Equivalent to BEGIN { require Module; Module->import( LIST ); }",
        }),
        "do" => Some(BuiltinDoc {
            signature: "do BLOCK\ndo EXPR",
            description: "As do BLOCK: executes BLOCK and returns its value. As do EXPR: reads and executes a Perl file.",
        }),

        // Math
        "abs" => Some(BuiltinDoc {
            signature: "abs VALUE\nabs",
            description: "Returns the absolute value of its argument.",
        }),
        "int" => Some(BuiltinDoc {
            signature: "int EXPR\nint",
            description: "Returns the integer portion of EXPR (truncates toward zero).",
        }),
        "sqrt" => Some(BuiltinDoc {
            signature: "sqrt EXPR\nsqrt",
            description: "Returns the positive square root of EXPR.",
        }),
        "log" => Some(BuiltinDoc {
            signature: "log EXPR\nlog",
            description: "Returns the natural logarithm (base e) of EXPR.",
        }),
        "exp" => Some(BuiltinDoc {
            signature: "exp EXPR\nexp",
            description: "Returns e (the natural logarithm base) to the power of EXPR.",
        }),
        "sin" => Some(BuiltinDoc {
            signature: "sin EXPR\nsin",
            description: "Returns the sine of EXPR (expressed in radians).",
        }),
        "cos" => Some(BuiltinDoc {
            signature: "cos EXPR\ncos",
            description: "Returns the cosine of EXPR (expressed in radians).",
        }),
        "atan2" => Some(BuiltinDoc {
            signature: "atan2 Y, X",
            description: "Returns the arctangent of Y/X in the range -PI to PI.",
        }),
        "rand" => Some(BuiltinDoc {
            signature: "rand EXPR\nrand",
            description: "Returns a random fractional number greater than or equal to 0 and less than EXPR (default 1).",
        }),
        "srand" => Some(BuiltinDoc {
            signature: "srand EXPR\nsrand",
            description: "Sets the random number seed for the rand operator.",
        }),

        // File tests and operations
        "stat" => Some(BuiltinDoc {
            signature: "stat FILEHANDLE\nstat EXPR",
            description: "Returns a 13-element list giving the status info for a file. (dev, ino, mode, nlink, uid, gid, rdev, size, atime, mtime, ctime, blksize, blocks).",
        }),
        "lstat" => Some(BuiltinDoc {
            signature: "lstat FILEHANDLE\nlstat EXPR",
            description: "Like stat, but if the last component of the filename is a symbolic link, stats the link itself.",
        }),
        "chmod" => Some(BuiltinDoc {
            signature: "chmod MODE, LIST",
            description: "Changes the permissions of a list of files. Returns the number of files successfully changed.",
        }),
        "chown" => Some(BuiltinDoc {
            signature: "chown UID, GID, LIST",
            description: "Changes the owner and group of a list of files.",
        }),
        "unlink" => Some(BuiltinDoc {
            signature: "unlink LIST\nunlink",
            description: "Deletes a list of files. Returns the number of files successfully deleted.",
        }),
        "rename" => Some(BuiltinDoc {
            signature: "rename OLDNAME, NEWNAME",
            description: "Renames a file. Returns true on success, false otherwise.",
        }),
        "mkdir" => Some(BuiltinDoc {
            signature: "mkdir FILENAME, MODE\nmkdir FILENAME",
            description: "Creates the directory specified by FILENAME. Returns true on success.",
        }),
        "rmdir" => Some(BuiltinDoc {
            signature: "rmdir FILENAME\nrmdir",
            description: "Deletes the directory if it is empty. Returns true on success.",
        }),
        "opendir" => Some(BuiltinDoc {
            signature: "opendir DIRHANDLE, EXPR",
            description: "Opens a directory for reading by readdir.",
        }),
        "readdir" => Some(BuiltinDoc {
            signature: "readdir DIRHANDLE",
            description: "Returns the next entry (or entries in list context) from the directory.",
        }),
        "closedir" => Some(BuiltinDoc {
            signature: "closedir DIRHANDLE",
            description: "Closes a directory opened by opendir.",
        }),
        "link" => Some(BuiltinDoc {
            signature: "link OLDFILE, NEWFILE",
            description: "Creates a new hard link for an existing file.",
        }),
        "symlink" => Some(BuiltinDoc {
            signature: "symlink OLDFILE, NEWFILE",
            description: "Creates a new symbolic link for an existing file.",
        }),
        "readlink" => Some(BuiltinDoc {
            signature: "readlink EXPR\nreadlink",
            description: "Returns the value of a symbolic link.",
        }),
        "chdir" => Some(BuiltinDoc {
            signature: "chdir EXPR\nchdir",
            description: "Changes the working directory to EXPR (or home directory if omitted).",
        }),
        "glob" => Some(BuiltinDoc {
            signature: "glob EXPR\nglob",
            description: "Returns the filenames matching the shell-style glob pattern EXPR.",
        }),

        // System/Process
        "system" => Some(BuiltinDoc {
            signature: "system LIST\nsystem PROGRAM LIST",
            description: "Executes a system command and returns the exit status. The return value is the exit status of the program as returned by the wait call.",
        }),
        "exec" => Some(BuiltinDoc {
            signature: "exec LIST\nexec PROGRAM LIST",
            description: "Replaces the current process with an external command. Never returns on success.",
        }),
        "fork" => Some(BuiltinDoc {
            signature: "fork",
            description: "Creates a child process. Returns the child pid to the parent, 0 to the child, or undef on failure.",
        }),
        "wait" => Some(BuiltinDoc {
            signature: "wait",
            description: "Waits for a child process to terminate and returns the pid of the deceased process.",
        }),
        "waitpid" => Some(BuiltinDoc {
            signature: "waitpid PID, FLAGS",
            description: "Waits for a particular child process to terminate and returns the pid.",
        }),
        "kill" => Some(BuiltinDoc {
            signature: "kill SIGNAL, LIST",
            description: "Sends a signal to a list of processes. Returns the number of processes signalled.",
        }),
        "sleep" => Some(BuiltinDoc {
            signature: "sleep EXPR\nsleep",
            description: "Causes the script to sleep for EXPR seconds (or forever if no argument).",
        }),
        "alarm" => Some(BuiltinDoc {
            signature: "alarm SECONDS\nalarm",
            description: "Arranges to have a SIGALRM delivered after SECONDS seconds.",
        }),

        // Encoding/Decoding
        "pack" => Some(BuiltinDoc {
            signature: "pack TEMPLATE, LIST",
            description: "Takes a list of values and packs it into a binary string according to TEMPLATE.",
        }),
        "unpack" => Some(BuiltinDoc {
            signature: "unpack TEMPLATE, EXPR",
            description: "Takes a binary string and expands it into a list of values according to TEMPLATE.",
        }),
        "crypt" => Some(BuiltinDoc {
            signature: "crypt PLAINTEXT, SALT",
            description: "Encrypts a string using the system crypt() function.",
        }),

        // Time
        "time" => Some(BuiltinDoc {
            signature: "time",
            description: "Returns the number of seconds since the epoch (January 1, 1970 UTC).",
        }),
        "localtime" => Some(BuiltinDoc {
            signature: "localtime EXPR\nlocaltime",
            description: "Converts a time value to a 9-element list with the time analyzed for the local time zone. In scalar context returns a ctime(3) string.",
        }),
        "gmtime" => Some(BuiltinDoc {
            signature: "gmtime EXPR\ngmtime",
            description: "Like localtime but uses Greenwich Mean Time (UTC).",
        }),

        // Misc
        "prototype" => Some(BuiltinDoc {
            signature: "prototype FUNCTION",
            description: "Returns the prototype of a function as a string, or undef if the function has no prototype.",
        }),
        "local" => Some(BuiltinDoc {
            signature: "local EXPR",
            description: "Temporarily localizes the listed global variables to the enclosing block. The original values are restored at the end of the block.",
        }),
        "my" => Some(BuiltinDoc {
            signature: "my VARLIST\nmy TYPE VARLIST",
            description: "Declares lexically scoped variables. Variables are visible only within the enclosing block.",
        }),
        "our" => Some(BuiltinDoc {
            signature: "our VARLIST",
            description: "Declares package variables visible in the current lexical scope without qualifying the name.",
        }),
        "state" => Some(BuiltinDoc {
            signature: "state VARLIST",
            description: "Declares lexically scoped variables that persist across calls to the enclosing subroutine (like C static variables).",
        }),

        // Carp — stack-aware exception helpers (use Carp)
        "croak" => Some(BuiltinDoc {
            signature: "croak LIST",
            description: "Like die but reports the error from the caller's perspective (one stack level up). Use in modules so the error points at the calling code, not the module internals.",
        }),
        "carp" => Some(BuiltinDoc {
            signature: "carp LIST",
            description: "Like warn but reports the warning from the caller's perspective (one stack level up). Use in modules so the warning points at the calling code.",
        }),
        "confess" => Some(BuiltinDoc {
            signature: "confess LIST",
            description: "Like die but includes a full stack trace. Use when deep call chains make croak's single-level context insufficient.",
        }),
        "cluck" => Some(BuiltinDoc {
            signature: "cluck LIST",
            description: "Like warn but includes a full stack trace. Use when you need a warning with complete call context.",
        }),

        _ => None,
    }
}

/// Structured exception context for die/warn and Carp functions.
///
/// Provides IDE guidance about exception semantics: which variable captures the
/// error and whether a stack-aware alternative is preferred.
pub struct ExceptionContext {
    /// The variable that captures the thrown exception, if applicable.
    /// For `die` this is `$@`; Carp functions do not set a distinct variable.
    pub error_variable: Option<String>,
    /// A preferred alternative (e.g. `Carp::croak` instead of `die`).
    /// `None` when the function is already the preferred form.
    pub preferred_alternative: Option<String>,
}

/// Check if a function name belongs to Perl's exception-handling family.
///
/// Returns `true` for `die`, `warn`, and all four Carp functions
/// (`croak`, `carp`, `confess`, `cluck`).
pub fn is_exception_function(name: &str) -> bool {
    matches!(name, "die" | "warn" | "croak" | "carp" | "confess" | "cluck")
}

/// Return exception context for a die/warn/Carp function.
///
/// Returns `None` for any function that is not part of the exception family.
pub fn get_exception_context(name: &str) -> Option<ExceptionContext> {
    match name {
        "die" => Some(ExceptionContext {
            error_variable: Some("$@".to_string()),
            preferred_alternative: Some("Carp::croak".to_string()),
        }),
        "warn" => Some(ExceptionContext {
            error_variable: None,
            preferred_alternative: Some("Carp::carp".to_string()),
        }),
        "croak" | "confess" => Some(ExceptionContext {
            error_variable: Some("$@".to_string()),
            preferred_alternative: None,
        }),
        "carp" | "cluck" => {
            Some(ExceptionContext { error_variable: None, preferred_alternative: None })
        }
        _ => None,
    }
}
