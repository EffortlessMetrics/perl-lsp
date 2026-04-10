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

/// Get documentation for a Perl file test operator.
///
/// Returns signature and description for known file test operators,
/// or `None` if documentation is not available.
pub fn get_operator_documentation(op: &str) -> Option<BuiltinDoc> {
    macro_rules! doc {
        ($signature:expr, $description:expr) => {
            Some(BuiltinDoc { signature: $signature, description: $description })
        };
    }

    match op {
        "-e" => doc!("-e FILE\n-e", "Returns true if FILE exists. If FILE is omitted, tests `$_`."),
        "-f" => doc!(
            "-f FILE\n-f",
            "Returns true if FILE is a plain file. If FILE is omitted, tests `$_`."
        ),
        "-d" => doc!(
            "-d FILE\n-d",
            "Returns true if FILE is a directory. If FILE is omitted, tests `$_`."
        ),
        "-r" => doc!(
            "-r FILE\n-r",
            "Returns true if FILE is readable by the effective user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-w" => doc!(
            "-w FILE\n-w",
            "Returns true if FILE is writable by the effective user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-x" => doc!(
            "-x FILE\n-x",
            "Returns true if FILE is executable by the effective user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-o" => doc!(
            "-o FILE\n-o",
            "Returns true if FILE is owned by the effective user ID. If FILE is omitted, tests `$_`."
        ),
        "-R" => doc!(
            "-R FILE\n-R",
            "Returns true if FILE is readable by the real user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-W" => doc!(
            "-W FILE\n-W",
            "Returns true if FILE is writable by the real user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-X" => doc!(
            "-X FILE\n-X",
            "Returns true if FILE is executable by the real user or group ID. If FILE is omitted, tests `$_`."
        ),
        "-O" => doc!(
            "-O FILE\n-O",
            "Returns true if FILE is owned by the real user ID. If FILE is omitted, tests `$_`."
        ),
        "-z" => doc!(
            "-z FILE\n-z",
            "Returns true if FILE exists and has zero size. If FILE is omitted, tests `$_`."
        ),
        "-s" => doc!(
            "-s FILE\n-s",
            "Returns the file size in bytes in scalar context, or true if FILE has nonzero size. If FILE is omitted, tests `$_`."
        ),
        "-l" => doc!(
            "-l FILE\n-l",
            "Returns true if FILE is a symbolic link. If FILE is omitted, tests `$_`."
        ),
        "-p" => doc!(
            "-p FILE\n-p",
            "Returns true if FILE is a named pipe (FIFO). If FILE is omitted, tests `$_`."
        ),
        "-S" => {
            doc!("-S FILE\n-S", "Returns true if FILE is a socket. If FILE is omitted, tests `$_`.")
        }
        "-u" => doc!(
            "-u FILE\n-u",
            "Returns true if FILE has the setuid bit set. If FILE is omitted, tests `$_`."
        ),
        "-g" => doc!(
            "-g FILE\n-g",
            "Returns true if FILE has the setgid bit set. If FILE is omitted, tests `$_`."
        ),
        "-k" => doc!(
            "-k FILE\n-k",
            "Returns true if FILE has the sticky bit set. If FILE is omitted, tests `$_`."
        ),
        "-t" => doc!(
            "-t FILEHANDLE\n-t",
            "Returns true if FILEHANDLE is connected to a tty. If FILEHANDLE is omitted, tests `STDIN`."
        ),
        "-T" => doc!(
            "-T FILE\n-T",
            "Returns true if FILE looks like a text file. If FILE is omitted, tests `$_`."
        ),
        "-B" => doc!(
            "-B FILE\n-B",
            "Returns true if FILE looks like a binary file. If FILE is omitted, tests `$_`."
        ),
        "-M" => doc!(
            "-M FILE\n-M",
            "Returns the file age in days at program start, based on the file's modification time."
        ),
        "-A" => doc!("-A FILE\n-A", "Returns the file age in days based on the last access time."),
        "-C" => {
            doc!("-C FILE\n-C", "Returns the file age in days based on the last inode change time.")
        }
        "-b" => doc!(
            "-b FILE\n-b",
            "Returns true if FILE is a block special file. If FILE is omitted, tests `$_`."
        ),
        "-c" => doc!(
            "-c FILE\n-c",
            "Returns true if FILE is a character special file. If FILE is omitted, tests `$_`."
        ),
        _ => None,
    }
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
            description: "Joins the separate strings of LIST into a single string with fields separated by EXPR, and returns that string.\n\n```perl\nmy $str = join(', ', 'a', 'b', 'c');  # \"a, b, c\"\nmy $csv = join(',', @fields);\n```",
        }),
        "split" => Some(BuiltinDoc {
            signature: "split /PATTERN/, EXPR, LIMIT\nsplit /PATTERN/, EXPR\nsplit /PATTERN/\nsplit",
            description: "Splits the string EXPR into a list of strings and returns the list. If LIMIT is specified, splits into at most that many fields.\n\n```perl\nmy @words = split /\\s+/, $line;       # split on whitespace\nmy @fields = split /,/, $csv, 10;    # at most 10 fields\n```",
        }),

        // Array/List
        "push" => Some(BuiltinDoc {
            signature: "push ARRAY, LIST",
            description: "Appends one or more values to the end of ARRAY. Returns the number of elements in the resulting array.\n\n```perl\nmy @list = (1, 2);\npush @list, 3, 4;   # @list is now (1, 2, 3, 4)\n```",
        }),
        "pop" => Some(BuiltinDoc {
            signature: "pop ARRAY\npop",
            description: "Removes and returns the last element of ARRAY.\n\n```perl\nmy @stack = (1, 2, 3);\nmy $top = pop @stack;   # $top = 3, @stack = (1, 2)\n```",
        }),
        "shift" => Some(BuiltinDoc {
            signature: "shift ARRAY\nshift",
            description: "Removes and returns the first element of ARRAY, shortening the array by 1.\n\n```perl\nmy @queue = ('first', 'second');\nmy $item = shift @queue;   # $item = 'first'\n```",
        }),
        "unshift" => Some(BuiltinDoc {
            signature: "unshift ARRAY, LIST",
            description: "Prepends LIST to the front of ARRAY. Returns the number of elements in the new array.\n\n```perl\nmy @list = (3, 4);\nunshift @list, 1, 2;   # @list is now (1, 2, 3, 4)\n```",
        }),
        "splice" => Some(BuiltinDoc {
            signature: "splice ARRAY, OFFSET, LENGTH, LIST\nsplice ARRAY, OFFSET, LENGTH\nsplice ARRAY, OFFSET\nsplice ARRAY",
            description: "Removes LENGTH elements from ARRAY starting at OFFSET, replacing them with LIST. Returns the removed elements. In scalar context, returns the last removed element.",
        }),
        "sort" => Some(BuiltinDoc {
            signature: "sort SUBNAME LIST\nsort BLOCK LIST\nsort LIST",
            description: "Sorts LIST and returns the sorted list. BLOCK or SUBNAME provides a custom comparison function using $a and $b. Only valid in list context; using sort in scalar context returns undef (avoid).",
        }),
        "reverse" => Some(BuiltinDoc {
            signature: "reverse LIST",
            description: "In list context, returns LIST in reverse order. In scalar context, returns a string with characters reversed.",
        }),
        "map" => Some(BuiltinDoc {
            signature: "map BLOCK LIST\nmap EXPR, LIST",
            description: "Evaluates the BLOCK or EXPR for each element of LIST (locally setting $_ to each element) and composes a list of the results. In scalar context, returns the number of elements the expression would produce.\n\n```perl\nmy @doubled = map { $_ * 2 } @numbers;\nmy @names   = map { $_->{name} } @records;\n```",
        }),
        "grep" => Some(BuiltinDoc {
            signature: "grep BLOCK LIST\ngrep EXPR, LIST",
            description: "Evaluates BLOCK or EXPR for each element of LIST and returns the list of elements for which the expression is true. In scalar context, returns the number of matching elements rather than the list.\n\n```perl\nmy @evens = grep { $_ % 2 == 0 } @numbers;\nmy $count = grep { /pattern/ } @lines;\n```",
        }),
        "scalar" => Some(BuiltinDoc {
            signature: "scalar EXPR",
            description: "Forces EXPR to be interpreted in scalar context and returns the value of EXPR.",
        }),
        "wantarray" => Some(BuiltinDoc {
            signature: "wantarray",
            description: "Returns true if the subroutine is called in list context, false (defined but false) in scalar context, and undef in void context. Use to write context-sensitive subs: `return wantarray ? @list : $count;`",
        }),

        // Hash
        "keys" => Some(BuiltinDoc {
            signature: "keys HASH\nkeys ARRAY",
            description: "In list context, returns all keys of the named hash or indices of an array. In scalar context, returns the number of keys (an integer count). Note: `scalar keys %h` is the idiomatic way to count hash entries.",
        }),
        "values" => Some(BuiltinDoc {
            signature: "values HASH\nvalues ARRAY",
            description: "In list context, returns all values of the named hash or values of an array. In scalar context, returns the number of values (same as scalar keys).",
        }),
        "each" => Some(BuiltinDoc {
            signature: "each HASH\neach ARRAY",
            description: "Returns the next key-value pair from the hash as a two-element list, or an empty list when exhausted. The iterator resets when the list is exhausted, when keys() or values() is called on the hash, or when the hash is modified. Call in a while loop: `while (my ($k, $v) = each %h) { ... }`",
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

        // Tie magic methods
        "TIESCALAR" => Some(BuiltinDoc {
            signature: "TIESCALAR CLASSNAME, LIST",
            description: "Constructor called when `tie $scalar, CLASSNAME, LIST` is used. Must return a blessed reference.",
        }),
        "TIEARRAY" => Some(BuiltinDoc {
            signature: "TIEARRAY CLASSNAME, LIST",
            description: "Constructor called when `tie @array, CLASSNAME, LIST` is used. Must return a blessed reference.",
        }),
        "TIEHASH" => Some(BuiltinDoc {
            signature: "TIEHASH CLASSNAME, LIST",
            description: "Constructor called when `tie %hash, CLASSNAME, LIST` is used. Must return a blessed reference.",
        }),
        "TIEHANDLE" => Some(BuiltinDoc {
            signature: "TIEHANDLE CLASSNAME, LIST",
            description: "Constructor called when `tie *FH, CLASSNAME, LIST` is used. Must return a blessed reference.",
        }),
        "FETCH" => Some(BuiltinDoc {
            signature: "FETCH this",
            description: "Called on every access of a tied scalar or array/hash element. Returns the value.",
        }),
        "STORE" => Some(BuiltinDoc {
            signature: "STORE this, value",
            description: "Called on every assignment to a tied scalar or array/hash element.",
        }),
        "FIRSTKEY" => Some(BuiltinDoc {
            signature: "FIRSTKEY this",
            description: "Called when `keys` or `each` is first invoked on a tied hash.",
        }),
        "NEXTKEY" => Some(BuiltinDoc {
            signature: "NEXTKEY this, lastkey",
            description: "Called during iteration of a tied hash with `each` or `keys`.",
        }),
        "DESTROY" => Some(BuiltinDoc {
            signature: "DESTROY this",
            description: "Called when the tied object goes out of scope or is explicitly untied.",
        }),

        // Control flow
        "die" => Some(BuiltinDoc {
            signature: "die LIST",
            description: "Raises an exception. If LIST does not end in '\\n', Perl appends the script name and line number. In modules, prefer Carp::croak() to preserve the caller's stack frame. The exception is available in $@ after an eval block.",
        }),
        "warn" => Some(BuiltinDoc {
            signature: "warn LIST",
            description: "Prints a warning to STDERR. Does not exit. If the message does not end in '\\n', Perl appends the script name and line number. In modules, prefer Carp::carp() to report from the caller's perspective.",
        }),
        "eval" => Some(BuiltinDoc {
            signature: "eval BLOCK\neval EXPR",
            description: "Evaluates BLOCK or EXPR and traps exceptions. After the eval, check $@ for errors: if ($@) { ... }. BLOCK form is preferred — EXPR form (string eval) is a security risk and triggers the PL600 diagnostic.",
        }),
        // Carp module functions
        "croak" => Some(BuiltinDoc {
            signature: "croak LIST",
            description: "Like die but reports the error from the caller's perspective. Part of the Carp module. Use instead of die in library code so the stack trace points to the caller, not the module internals.",
        }),
        "carp" => Some(BuiltinDoc {
            signature: "carp LIST",
            description: "Like warn but reports the warning from the caller's perspective. Part of the Carp module. Prefer over warn in library code.",
        }),
        "confess" => Some(BuiltinDoc {
            signature: "confess LIST",
            description: "Like croak but includes a full stack trace. Part of the Carp module. Use when the full call chain is needed for debugging.",
        }),
        "cluck" => Some(BuiltinDoc {
            signature: "cluck LIST",
            description: "Like carp but includes a full stack trace. Part of the Carp module. Use for warnings that benefit from call chain context.",
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
            description: "Without argument, returns (package, filename, line) in list context or the package name in scalar context. With EXPR returns additional call-frame info: (package, filename, line, subroutine, hasargs, wantarray, evaltext, is_require, hints, bitmask, hinthash).",
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
            description: "Returns a 13-element list (dev, ino, mode, nlink, uid, gid, rdev, size, atime, mtime, ctime, blksize, blocks) or an empty list on failure.",
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
            description: "Like localtime but uses Greenwich Mean Time (UTC). In list context returns a 9-element time list (sec, min, hour, mday, mon, year, wday, yday, isdst). In scalar context returns a ctime(3)-style string.",
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
        "BEGIN" => Some(BuiltinDoc {
            signature: "BEGIN { BLOCK }",
            description: "Executed at **compile time**, before the rest of the program runs. \
                          Used to initialize modules, set up the symbol table, or run code \
                          that must complete before compilation continues. Multiple BEGIN \
                          blocks run in the order they appear in source.",
        }),
        "END" => Some(BuiltinDoc {
            signature: "END { BLOCK }",
            description: "Executed at **program exit**, after the main program finishes (including \
                          `die` and `exit`). Used for cleanup. Multiple END blocks run in \
                          reverse order of definition. `$?` holds the exit status.",
        }),
        "INIT" => Some(BuiltinDoc {
            signature: "INIT { BLOCK }",
            description: "Executed after compilation completes but **before** the main program \
                          runs. Runs in first-seen order. Unlike BEGIN, INIT sees the fully \
                          compiled symbol table.",
        }),
        "CHECK" => Some(BuiltinDoc {
            signature: "CHECK { BLOCK }",
            description: "Executed at the **end of compilation**, after all BEGIN blocks. Runs \
                          in reverse order of definition. Used by modules that need to inspect \
                          or modify the compiled program before it runs (e.g. B::* modules).",
        }),
        "UNITCHECK" => Some(BuiltinDoc {
            signature: "UNITCHECK { BLOCK }",
            description: "Executed at the **end of the compilation unit** that defined it \
                          (file, string eval, or require). Runs in reverse order of definition \
                          within that unit. More granular than CHECK — each required file's \
                          UNITCHECK runs before the requiring file's UNITCHECK.",
        }),

        // I/O: additional file and directory operations
        "fileno" => Some(BuiltinDoc {
            signature: "fileno FILEHANDLE",
            description: "Returns the file descriptor number for FILEHANDLE, or undef if the filehandle is not open.",
        }),
        "flock" => Some(BuiltinDoc {
            signature: "flock FILEHANDLE, OPERATION",
            description: "Calls flock(2) on FILEHANDLE. OPERATION is one of LOCK_SH, LOCK_EX, LOCK_UN, or LOCK_NB (from Fcntl). Returns true on success.",
        }),
        "select" => Some(BuiltinDoc {
            signature: "select RBITS, WBITS, EBITS, TIMEOUT\nselect FILEHANDLE\nselect",
            description: "With four args: calls select(2) to wait for input/output readiness. With one arg or no args: sets or returns the currently selected output filehandle.",
        }),
        "getc" => Some(BuiltinDoc {
            signature: "getc FILEHANDLE\ngetc",
            description: "Returns the next character from FILEHANDLE (STDIN if omitted), or undef at EOF.",
        }),
        "readline" => Some(BuiltinDoc {
            signature: "readline EXPR\nreadline",
            description: "Reads a line from the filehandle in EXPR. Equivalent to the angle-bracket operator <EXPR>. In list context, reads all remaining lines.",
        }),
        "readpipe" => Some(BuiltinDoc {
            signature: "readpipe EXPR",
            description: "Executes EXPR as a shell command and returns the standard output as a string in scalar context or as a list of lines in list context. Equivalent to `EXPR`.",
        }),
        "rewinddir" => Some(BuiltinDoc {
            signature: "rewinddir DIRHANDLE",
            description: "Sets the position of the directory at the beginning of a directory opened by opendir.",
        }),
        "seekdir" => Some(BuiltinDoc {
            signature: "seekdir DIRHANDLE, POS",
            description: "Sets the position of POS for the directory from telldir. DIRHANDLE must have been opened by opendir.",
        }),
        "telldir" => Some(BuiltinDoc {
            signature: "telldir DIRHANDLE",
            description: "Returns the current position of the readdir routines on DIRHANDLE.",
        }),
        "chroot" => Some(BuiltinDoc {
            signature: "chroot FILENAME\nchroot",
            description: "Changes the root directory for the current process to FILENAME. Requires root privileges.",
        }),
        "umask" => Some(BuiltinDoc {
            signature: "umask EXPR\numask",
            description: "Sets the umask for the process to EXPR and returns the previous value. If EXPR is omitted, returns the current umask.",
        }),

        // Socket functions
        "socket" => Some(BuiltinDoc {
            signature: "socket SOCKET, DOMAIN, TYPE, PROTOCOL",
            description: "Opens a socket of the specified kind and attaches it to filehandle SOCKET. Use constants from Socket module (AF_INET, SOCK_STREAM, etc.).",
        }),
        "socketpair" => Some(BuiltinDoc {
            signature: "socketpair SOCKET1, SOCKET2, DOMAIN, TYPE, PROTOCOL",
            description: "Creates an unnamed pair of sockets in the specified domain, of the specified type. Returns true on success.",
        }),
        "bind" => Some(BuiltinDoc {
            signature: "bind SOCKET, NAME",
            description: "Binds an address (NAME) to an already-opened socket. NAME is a packed address string created with sockaddr_in().",
        }),
        "connect" => Some(BuiltinDoc {
            signature: "connect SOCKET, NAME",
            description: "Attempts to connect to a remote socket. NAME is a packed address (see sockaddr_in). Returns true on success.",
        }),
        "listen" => Some(BuiltinDoc {
            signature: "listen SOCKET, QUEUESIZE",
            description: "Listens for incoming connections on a socket. QUEUESIZE is the maximum number of pending connections.",
        }),
        "accept" => Some(BuiltinDoc {
            signature: "accept NEWSOCKET, GENERICSOCKET",
            description: "Accepts an incoming socket connect, returning the packed address if it succeeded, or false on failure.",
        }),
        "shutdown" => Some(BuiltinDoc {
            signature: "shutdown SOCKET, HOW",
            description: "Shuts down a socket. HOW: 0=stop receiving, 1=stop sending, 2=stop both.",
        }),
        "send" => Some(BuiltinDoc {
            signature: "send SOCKET, MSG, FLAGS, TO\nsend SOCKET, MSG, FLAGS",
            description: "Sends a message on a socket. Returns the number of characters sent, or undef on error.",
        }),
        "recv" => Some(BuiltinDoc {
            signature: "recv SOCKET, SCALAR, LENGTH, FLAGS",
            description: "Receives a message on a socket, placing the message into SCALAR. Returns the address of the sender, or undef on error.",
        }),
        "setsockopt" => Some(BuiltinDoc {
            signature: "setsockopt SOCKET, LEVEL, OPTNAME, OPTVAL",
            description: "Sets the socket option requested. Returns undefined if there is an error.",
        }),
        "getsockopt" => Some(BuiltinDoc {
            signature: "getsockopt SOCKET, LEVEL, OPTNAME",
            description: "Queries the socket option requested, returning the value or undef on error.",
        }),
        "getsockname" => Some(BuiltinDoc {
            signature: "getsockname SOCKET",
            description: "Returns the packed sockaddr address of this end of the SOCKET connection.",
        }),
        "getpeername" => Some(BuiltinDoc {
            signature: "getpeername SOCKET",
            description: "Returns the packed sockaddr address of the other end of the SOCKET connection.",
        }),
        "pipe" => Some(BuiltinDoc {
            signature: "pipe READHANDLE, WRITEHANDLE",
            description: "Opens a pair of connected pipes like the corresponding system call. Returns true on success.",
        }),

        // Low-level system calls
        "syscall" => Some(BuiltinDoc {
            signature: "syscall NUMBER, LIST",
            description: "Calls the system call specified by NUMBER with the arguments in LIST.",
        }),
        "sysopen" => Some(BuiltinDoc {
            signature: "sysopen FILEHANDLE, FILENAME, MODE\nsysopen FILEHANDLE, FILENAME, MODE, PERMS",
            description: "Opens the file with the given FILENAME using C-level open(). MODE is a combination of O_RDONLY, O_WRONLY, O_RDWR, etc. from Fcntl.",
        }),
        "sysread" => Some(BuiltinDoc {
            signature: "sysread FILEHANDLE, SCALAR, LENGTH, OFFSET\nsysread FILEHANDLE, SCALAR, LENGTH",
            description: "Reads LENGTH bytes from FILEHANDLE using C-level read(), bypassing I/O buffering. Returns number of bytes read or undef on error.",
        }),
        "syswrite" => Some(BuiltinDoc {
            signature: "syswrite FILEHANDLE, SCALAR, LENGTH, OFFSET\nsyswrite FILEHANDLE, SCALAR, LENGTH",
            description: "Writes LENGTH bytes from SCALAR to FILEHANDLE using C-level write(), bypassing I/O buffering. Returns number of bytes written or undef on error.",
        }),
        "sysseek" => Some(BuiltinDoc {
            signature: "sysseek FILEHANDLE, POSITION, WHENCE",
            description: "Sets the position for FILEHANDLE and returns the new position in bytes. Uses C-level lseek(), bypassing stdio buffering.",
        }),
        "fcntl" => Some(BuiltinDoc {
            signature: "fcntl FILEHANDLE, FUNCTION, SCALAR",
            description: "Implements the fcntl(2) function. Requires Fcntl module for the function constants.",
        }),
        "ioctl" => Some(BuiltinDoc {
            signature: "ioctl FILEHANDLE, FUNCTION, SCALAR",
            description: "Implements the ioctl(2) function for device control.",
        }),

        // String and regex
        "pos" => Some(BuiltinDoc {
            signature: "pos SCALAR\npos",
            description: "Returns the offset of where the last m//g search left off for the variable SCALAR (or $_ if omitted). Can be assigned to.",
        }),
        "reset" => Some(BuiltinDoc {
            signature: "reset EXPR\nreset",
            description: "Resets the ?? search state and clears package variables matching the pattern EXPR. Rarely used.",
        }),
        "study" => Some(BuiltinDoc {
            signature: "study SCALAR\nstudy",
            description: "Historically asked Perl to pre-analyze the string for faster repeated pattern matching. Now a no-op in modern Perl but still valid syntax.",
        }),
        "vec" => Some(BuiltinDoc {
            signature: "vec EXPR, OFFSET, BITS",
            description: "Treats EXPR as a bit vector and returns the element at OFFSET. BITS must be a power of 2 (1, 2, 4, 8, 16, 32, or 64). Can be used as an lvalue.",
        }),

        // Process and user info
        "times" => Some(BuiltinDoc {
            signature: "times",
            description: "Returns a 4-element list (user, system, cuser, csystem) of the CPU times in seconds for this process and its children.",
        }),
        "getlogin" => Some(BuiltinDoc {
            signature: "getlogin",
            description: "Returns the current login name from /etc/utmp, or gives undef if not found.",
        }),
        "getppid" => Some(BuiltinDoc {
            signature: "getppid",
            description: "Returns the process ID of the parent process.",
        }),
        "getpgrp" => Some(BuiltinDoc {
            signature: "getpgrp PID\ngetpgrp",
            description: "Returns the current process group for PID (0 or omitted means the current process).",
        }),
        "setpgrp" => Some(BuiltinDoc {
            signature: "setpgrp PID, PGRP",
            description: "Sets the current process group for PID (0 means the current process).",
        }),
        "getpriority" => Some(BuiltinDoc {
            signature: "getpriority WHICH, WHO",
            description: "Returns the current priority for a process, process group, or user.",
        }),
        "setpriority" => Some(BuiltinDoc {
            signature: "setpriority WHICH, WHO, PRIORITY",
            description: "Sets the priority for a process, process group, or user.",
        }),

        // Password/group/host DB functions
        "getpwnam" => Some(BuiltinDoc {
            signature: "getpwnam NAME",
            description: "Returns a 9-element list of information about the user NAME from the password database.",
        }),
        "getpwuid" => Some(BuiltinDoc {
            signature: "getpwuid UID",
            description: "Returns a 9-element list of information about the user with UID from the password database.",
        }),
        "getpwent" => Some(BuiltinDoc {
            signature: "getpwent",
            description: "Returns the next entry from the password database, one entry at a time.",
        }),
        "setpwent" => Some(BuiltinDoc {
            signature: "setpwent",
            description: "Rewinds the password database to the beginning for iteration with getpwent.",
        }),
        "endpwent" => Some(BuiltinDoc {
            signature: "endpwent",
            description: "Closes the password database after iterating with getpwent.",
        }),
        "getgrnam" => Some(BuiltinDoc {
            signature: "getgrnam NAME",
            description: "Returns a 4-element list of information about the group NAME from the group database.",
        }),
        "getgrgid" => Some(BuiltinDoc {
            signature: "getgrgid GID",
            description: "Returns a 4-element list of information about the group with GID from the group database.",
        }),
        "getgrent" => Some(BuiltinDoc {
            signature: "getgrent",
            description: "Returns the next entry from the group database, one entry at a time.",
        }),
        "setgrent" => Some(BuiltinDoc {
            signature: "setgrent",
            description: "Rewinds the group database to the beginning for iteration with getgrent.",
        }),
        "endgrent" => Some(BuiltinDoc {
            signature: "endgrent",
            description: "Closes the group database after iterating with getgrent.",
        }),
        "gethostbyname" => Some(BuiltinDoc {
            signature: "gethostbyname NAME",
            description: "Translates a network hostname NAME to its corresponding network addresses, returning a list (name, aliases, addrtype, length, @addrs).",
        }),
        "gethostbyaddr" => Some(BuiltinDoc {
            signature: "gethostbyaddr ADDR, ADDRTYPE",
            description: "Translates a network address to a hostname, returning a list (name, aliases, addrtype, length, @addrs).",
        }),
        "gethostent" => Some(BuiltinDoc {
            signature: "gethostent",
            description: "Returns the next entry from the hosts database, one entry at a time.",
        }),
        "sethostent" => Some(BuiltinDoc {
            signature: "sethostent STAYOPEN",
            description: "Opens or rewinds the hosts database. If STAYOPEN is true, the database is not closed between calls.",
        }),
        "endhostent" => Some(BuiltinDoc {
            signature: "endhostent",
            description: "Closes the hosts database after iterating with gethostent.",
        }),
        "getnetbyname" => Some(BuiltinDoc {
            signature: "getnetbyname NAME",
            description: "Returns information about the named network from the networks database.",
        }),
        "getnetbyaddr" => Some(BuiltinDoc {
            signature: "getnetbyaddr ADDR, ADDRTYPE",
            description: "Returns information about the network with the given address from the networks database.",
        }),
        "getnetent" => Some(BuiltinDoc {
            signature: "getnetent",
            description: "Returns the next entry from the networks database, one entry at a time.",
        }),
        "setnetent" => Some(BuiltinDoc {
            signature: "setnetent STAYOPEN",
            description: "Opens or rewinds the networks database.",
        }),
        "endnetent" => Some(BuiltinDoc {
            signature: "endnetent",
            description: "Closes the networks database after iterating with getnetent.",
        }),
        "getprotobyname" => Some(BuiltinDoc {
            signature: "getprotobyname NAME",
            description: "Returns information about the named protocol from the protocols database.",
        }),
        "getprotobynumber" => Some(BuiltinDoc {
            signature: "getprotobynumber NUMBER",
            description: "Returns information about the protocol with the given number from the protocols database.",
        }),
        "getprotoent" => Some(BuiltinDoc {
            signature: "getprotoent",
            description: "Returns the next entry from the protocols database, one entry at a time.",
        }),
        "setprotoent" => Some(BuiltinDoc {
            signature: "setprotoent STAYOPEN",
            description: "Opens or rewinds the protocols database.",
        }),
        "endprotoent" => Some(BuiltinDoc {
            signature: "endprotoent",
            description: "Closes the protocols database after iterating with getprotoent.",
        }),
        "getservbyname" => Some(BuiltinDoc {
            signature: "getservbyname NAME, PROTO",
            description: "Returns information about the named service from the services database.",
        }),
        "getservbyport" => Some(BuiltinDoc {
            signature: "getservbyport PORT, PROTO",
            description: "Returns information about the service at the given port from the services database.",
        }),
        "getservent" => Some(BuiltinDoc {
            signature: "getservent",
            description: "Returns the next entry from the services database, one entry at a time.",
        }),
        "setservent" => Some(BuiltinDoc {
            signature: "setservent STAYOPEN",
            description: "Opens or rewinds the services database.",
        }),
        "endservent" => Some(BuiltinDoc {
            signature: "endservent",
            description: "Closes the services database after iterating with getservent.",
        }),

        // IPC: message queues and shared memory
        "msgget" => Some(BuiltinDoc {
            signature: "msgget KEY, FLAGS",
            description: "Calls the System V IPC function msgget(2). Returns the message queue ID or undef on error.",
        }),
        "msgctl" => Some(BuiltinDoc {
            signature: "msgctl ID, CMD, ARG",
            description: "Calls the System V IPC function msgctl(2). Performs control operations on the message queue ID.",
        }),
        "msgsnd" => Some(BuiltinDoc {
            signature: "msgsnd ID, MSG, FLAGS",
            description: "Calls the System V IPC function msgsnd(2). Sends MSG to the message queue ID.",
        }),
        "msgrcv" => Some(BuiltinDoc {
            signature: "msgrcv ID, VAR, SIZE, TYPE, FLAGS",
            description: "Calls the System V IPC function msgrcv(2). Reads a message from the message queue into VAR.",
        }),
        "semget" => Some(BuiltinDoc {
            signature: "semget KEY, NSEMS, FLAGS",
            description: "Calls the System V IPC function semget(2). Returns the semaphore set ID or undef.",
        }),
        "semctl" => Some(BuiltinDoc {
            signature: "semctl ID, SEMNUM, CMD, ARG",
            description: "Calls the System V IPC function semctl(2). Performs control on the semaphore.",
        }),
        "semop" => Some(BuiltinDoc {
            signature: "semop KEY, OPSTRING",
            description: "Calls the System V IPC function semop(2). Performs semaphore operations.",
        }),
        "shmget" => Some(BuiltinDoc {
            signature: "shmget KEY, SIZE, FLAGS",
            description: "Calls the System V IPC function shmget(2). Returns the shared memory segment ID or undef.",
        }),
        "shmctl" => Some(BuiltinDoc {
            signature: "shmctl ID, CMD, ARG",
            description: "Calls the System V IPC function shmctl(2). Performs control on the shared memory segment.",
        }),
        "shmread" => Some(BuiltinDoc {
            signature: "shmread ID, VAR, POS, SIZE",
            description: "Reads SIZE bytes from the shared memory segment ID at position POS into VAR.",
        }),
        "shmwrite" => Some(BuiltinDoc {
            signature: "shmwrite ID, STRING, POS, SIZE",
            description: "Writes SIZE bytes of STRING to the shared memory segment ID at position POS.",
        }),

        // Legacy/format
        "format" => Some(BuiltinDoc {
            signature: "format NAME =\n  FORMLIST\n.",
            description: "Declares a picture format for use by the write() function. NAME defaults to the current package name. Uses `~` and `~~` for text fields.",
        }),
        "formline" => Some(BuiltinDoc {
            signature: "formline PICTURE, LIST",
            description: "An internal function used by write() for formatting. PICTURE is the format template; LIST provides values.",
        }),
        "dump" => Some(BuiltinDoc {
            signature: "dump LABEL\ndump",
            description: "Causes an immediate core dump. Now mostly obsolete. Jumps to the LABEL after restart if specified.",
        }),
        "dbmopen" => Some(BuiltinDoc {
            signature: "dbmopen HASH, DBNAME, MODE",
            description: "Opens a dbm file and ties it to HASH. Deprecated — use GDBM_File, DB_File, or similar tie-based alternatives.",
        }),
        "dbmclose" => Some(BuiltinDoc {
            signature: "dbmclose HASH",
            description: "Breaks the binding between a DBM file and a hash (deprecated; use untie instead).",
        }),

        _ => None,
    }
}

/// Get documentation for a Moose/Moo/Mouse built-in type constraint.
///
/// Accepts both bare types (`Str`, `ArrayRef`) and parametrized forms
/// (`ArrayRef[Int]`, `Maybe[Str]`).  For parametrized forms the base
/// type is extracted and used for the lookup.
///
/// Returns signature and description suitable for LSP hover display,
/// or `None` if the type is not a known Moose built-in.
pub fn get_moose_type_documentation(type_str: &str) -> Option<BuiltinDoc> {
    // Strip optional parametrization: "ArrayRef[Int]" -> "ArrayRef"
    let base = type_str.split('[').next().unwrap_or(type_str).trim();

    match base {
        // Moose::Util::TypeConstraints — Any / Item
        "Any" => Some(BuiltinDoc {
            signature: "Any",
            description: "The root type. Every value passes this constraint.",
        }),
        "Item" => Some(BuiltinDoc {
            signature: "Item",
            description: "Synonym for Any. Used as a base for the type hierarchy.",
        }),
        // Undef / Defined
        "Undef" => Some(BuiltinDoc { signature: "Undef", description: "Accepts only undef." }),
        "Defined" => Some(BuiltinDoc {
            signature: "Defined",
            description: "Accepts any defined value (anything that is not undef).",
        }),
        // Value / Bool
        "Value" => Some(BuiltinDoc {
            signature: "Value",
            description: "Accepts any defined, non-reference value (scalars and strings).",
        }),
        "Bool" => Some(BuiltinDoc {
            signature: "Bool",
            description: "Accepts 1, 0, the empty string '', or undef — Perl's boolean-ish values.",
        }),
        // Strings
        "Str" => Some(BuiltinDoc {
            signature: "Str",
            description: "Accepts any defined, non-reference scalar value (a string or number).",
        }),
        "Num" => Some(BuiltinDoc {
            signature: "Num",
            description: "Accepts any value that looks like a number (integer or float).",
        }),
        "Int" => Some(BuiltinDoc {
            signature: "Int",
            description: "Accepts only integer values (no decimal point).",
        }),
        "ClassName" => Some(BuiltinDoc {
            signature: "ClassName",
            description: "Accepts a string that is the name of a loaded Perl package/class.",
        }),
        "RoleName" => Some(BuiltinDoc {
            signature: "RoleName",
            description: "Accepts a string that is the name of a loaded Moose role.",
        }),
        // References
        "Ref" => Some(BuiltinDoc { signature: "Ref", description: "Accepts any reference." }),
        "ScalarRef" => Some(BuiltinDoc {
            signature: "ScalarRef[TYPE]",
            description: "Accepts a scalar reference. Optionally parametrized: ScalarRef[Int] requires the referent to satisfy Int.",
        }),
        "ArrayRef" => Some(BuiltinDoc {
            signature: "ArrayRef[TYPE]",
            description: "Accepts an array reference. Optionally parametrized: ArrayRef[Int] requires all elements to satisfy Int.",
        }),
        "HashRef" => Some(BuiltinDoc {
            signature: "HashRef[TYPE]",
            description: "Accepts a hash reference. Optionally parametrized: HashRef[Str] requires all values to satisfy Str.",
        }),
        "CodeRef" => Some(BuiltinDoc {
            signature: "CodeRef",
            description: "Accepts a code reference (subroutine reference).",
        }),
        "RegexpRef" => Some(BuiltinDoc {
            signature: "RegexpRef",
            description: "Accepts a compiled regular expression reference (qr//).",
        }),
        "GlobRef" => {
            Some(BuiltinDoc { signature: "GlobRef", description: "Accepts a glob reference." })
        }
        "FileHandle" => Some(BuiltinDoc {
            signature: "FileHandle",
            description: "Accepts an IO object or a glob reference that can be used as a filehandle.",
        }),
        // Object / Role
        "Object" => Some(BuiltinDoc {
            signature: "Object",
            description: "Accepts any blessed reference (an object).",
        }),
        // Maybe
        "Maybe" => Some(BuiltinDoc {
            signature: "Maybe[TYPE]",
            description: "Accepts undef or any value satisfying TYPE. Useful for optional attributes: Maybe[Str] accepts either a string or undef.",
        }),
        // Type::Tiny extras commonly used with Moo
        "InstanceOf" => Some(BuiltinDoc {
            signature: "InstanceOf[CLASSNAME]",
            description: "Accepts a blessed object that is an instance of CLASSNAME.",
        }),
        "ConsumerOf" => Some(BuiltinDoc {
            signature: "ConsumerOf[ROLENAME]",
            description: "Accepts a blessed object that consumes ROLENAME.",
        }),
        "HasMethods" => Some(BuiltinDoc {
            signature: "HasMethods[METHOD, ...]",
            description: "Accepts a blessed object that has all the listed methods.",
        }),
        "Dict" => Some(BuiltinDoc {
            signature: "Dict[KEY => TYPE, ...]",
            description: "Accepts a hash reference matching a specific key/type schema (Type::Tiny).",
        }),
        "Tuple" => Some(BuiltinDoc {
            signature: "Tuple[TYPE, ...]",
            description: "Accepts an array reference matching a specific positional type schema (Type::Tiny).",
        }),
        "Map" => Some(BuiltinDoc {
            signature: "Map[KEYTYPE, VALUETYPE]",
            description: "Accepts a hash reference where keys satisfy KEYTYPE and values satisfy VALUETYPE (Type::Tiny).",
        }),
        "Enum" => Some(BuiltinDoc {
            signature: "Enum[VALUE, ...]",
            description: "Accepts a string that is one of the listed values (Type::Tiny).",
        }),

        _ => None,
    }
}

/// Get documentation for a Perl subroutine or variable attribute.
///
/// Attributes are declared with `:name` syntax, e.g. `sub foo :lvalue { ... }`.
/// Pass the attribute name without the leading colon.
///
/// Returns signature and description suitable for LSP hover display,
/// or `None` if the attribute is not a known built-in.
pub fn get_attribute_documentation(attr: &str) -> Option<BuiltinDoc> {
    // Strip leading colon if present
    let name = attr.trim_start_matches(':');

    match name {
        "lvalue" => Some(BuiltinDoc {
            signature: ":lvalue",
            description: "Marks a subroutine as an lvalue subroutine. The return value can be assigned to, enabling constructs like `foo() = 42;`.",
        }),
        "method" => Some(BuiltinDoc {
            signature: ":method",
            description: "Marks a subroutine as a method. Used by some attribute handlers to modify dispatch or prototype checking.",
        }),
        "prototype" => Some(BuiltinDoc {
            signature: ":prototype(PROTO)",
            description: "Sets the prototype of a subroutine. Controls how Perl parses calls to the sub (e.g. `prototype($$)` for two scalar args).",
        }),
        "const" => Some(BuiltinDoc {
            signature: ":const",
            description: "Marks a subroutine as a constant. The value is computed once and cached; subsequent calls return the cached value immutably.",
        }),
        "shared" => Some(BuiltinDoc {
            signature: ":shared",
            description: "Marks a variable or subroutine as shared across threads (requires `threads::shared`). The variable is accessible from all threads.",
        }),
        "weak_ref" => Some(BuiltinDoc {
            signature: ":weak_ref",
            description: "Marks a Moose/Moo attribute as a weak reference. The stored reference will not prevent the referent from being garbage-collected.",
        }),
        "overload" => Some(BuiltinDoc {
            signature: ":overload(OP)",
            description: "Declares that a subroutine implements an operator overload for OP.",
        }),
        _ => None,
    }
}

/// Structured exception context for exception-family functions.
///
/// Used by code actions and semantic analysis to understand exception
/// handling semantics — upgrade paths (die → croak) and associated
/// error variables.
#[derive(Debug, Clone)]
pub struct ExceptionContext {
    /// Special variable that captures the exception after an eval block (e.g. `$@`).
    pub error_variable: Option<String>,
    /// Recommended replacement function, if the current function is not preferred
    /// (e.g. `die` → `Carp::croak`, `warn` → `Carp::carp`).
    pub preferred_alternative: Option<String>,
}

/// Check if a function name is in the Perl exception family.
///
/// Returns `true` for: `die`, `warn`, `croak`, `carp`, `confess`, `cluck`.
///
/// This is a classification helper for future diagnostic and code-action use.
/// It is not currently called from any LSP code path — callers may use it to
/// decide whether to invoke [`get_exception_context`].
///
/// # Examples
/// ```
/// use perl_semantic_analyzer::analysis::semantic::is_exception_function;
///
/// assert!(is_exception_function("die"));
/// assert!(is_exception_function("croak"));
/// assert!(!is_exception_function("print"));
/// ```
pub fn is_exception_function(name: &str) -> bool {
    matches!(name, "die" | "warn" | "croak" | "carp" | "confess" | "cluck")
}

/// Get exception context for upgrade suggestions and error variables.
///
/// Returns metadata about exception handling semantics:
/// - `error_variable`: special variable capturing the exception (`$@`)
/// - `preferred_alternative`: recommended upgrade path (`die` → `Carp::croak`)
///
/// Returns `None` for non-exception functions (e.g. `eval`, `print`).
///
/// # Examples
/// ```
/// use perl_semantic_analyzer::analysis::semantic::get_exception_context;
///
/// let die_ctx = get_exception_context("die").unwrap();
/// assert_eq!(die_ctx.error_variable, Some("$@".to_string()));
/// assert_eq!(die_ctx.preferred_alternative, Some("Carp::croak".to_string()));
///
/// let croak_ctx = get_exception_context("croak").unwrap();
/// assert_eq!(croak_ctx.preferred_alternative, None);  // already preferred
/// ```
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

/// Validate that a function name is safe to pass to perldoc.
///
/// Accepts only names composed of ASCII alphanumeric characters and underscores.
/// This rejects `::` package separators, path separators, shell metacharacters,
/// and any other characters that could cause injection or path traversal.
///
/// Returns `true` if the name is safe to use in a perldoc subprocess call.
pub(crate) fn is_safe_perldoc_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Result of a dynamic perldoc lookup, containing owned strings for caching.
#[derive(Debug, Clone)]
pub struct PerldocResult {
    /// Function signature extracted from perldoc output.
    pub signature: String,
    /// Description extracted from perldoc output.
    pub description: String,
}

/// Parse the raw text output of `perldoc -f <name>` into a structured result.
///
/// Extracts the first synopsis/signature line(s) and the following description.
/// Returns `None` if the output is empty or doesn't look like valid perldoc.
pub(crate) fn parse_perldoc_output(output: &str) -> Option<PerldocResult> {
    let text = output.trim();
    if text.is_empty() {
        return None;
    }

    // Strip ANSI escape codes that some perldoc versions emit.
    let clean: String = {
        let mut cleaned = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip until terminating alphabetic character of the escape sequence
                for skip in chars.by_ref() {
                    if skip.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                cleaned.push(c);
            }
        }
        cleaned
    };

    let lines: Vec<&str> = clean.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // The first non-empty line(s) are the synopsis/signature.
    // Description follows after a blank line.
    let mut sig_lines: Vec<&str> = Vec::new();
    let mut desc_lines: Vec<&str> = Vec::new();
    let mut in_desc = false;

    for line in &lines {
        if !in_desc {
            if line.trim().is_empty() && !sig_lines.is_empty() {
                in_desc = true;
            } else if !line.trim().is_empty() {
                sig_lines.push(line.trim());
            }
        } else if desc_lines.len() < 20 {
            // Limit description to 20 lines for manageable hover tooltips
            desc_lines.push(line.trim_end());
        }
    }

    if sig_lines.is_empty() {
        return None;
    }

    let signature = sig_lines.join("\n");
    let description = desc_lines.join("\n").trim().to_string();

    if description.is_empty() {
        Some(PerldocResult { signature: signature.clone(), description: signature })
    } else {
        Some(PerldocResult { signature, description })
    }
}

/// Look up documentation for a Perl function via the system `perldoc` command.
///
/// This is the dynamic fallback for builtins not covered by [`get_builtin_documentation`].
/// It calls `perldoc -f <name>` with a 500ms timeout and caches results in a
/// process-wide cache (up to 100 entries).
///
/// # Security
///
/// The function name is validated with [`is_safe_perldoc_name`] before being
/// passed to the subprocess. Only `[a-zA-Z0-9_]` characters are accepted.
///
/// # Availability
///
/// Returns `None` if the name fails validation, `perldoc` is not installed,
/// the subprocess times out, the exit code is non-zero, or the output
/// cannot be parsed. This function is not available in WASM builds.
#[cfg(not(target_arch = "wasm32"))]
pub fn perldoc_lookup(name: &str) -> Option<PerldocResult> {
    use std::collections::HashMap;
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    static CACHE: OnceLock<Mutex<HashMap<String, Option<PerldocResult>>>> = OnceLock::new();
    const MAX_CACHE_ENTRIES: usize = 100;
    const TIMEOUT_MS: u64 = 500;

    if !is_safe_perldoc_name(name) {
        return None;
    }

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    // Return cached result if available.
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(name) {
            return cached.clone();
        }
    }

    // Run perldoc -f <name> with a manual timeout loop.
    let result = (|| {
        let mut cmd = Command::new("perldoc");
        cmd.arg("-f").arg(name);
        // Disable interactive pager.
        cmd.env("PERLDOC_PAGER", "").env("PAGER", "cat");

        let mut child = cmd.spawn().ok()?;
        let deadline = std::time::Instant::now() + Duration::from_millis(TIMEOUT_MS);

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        let output = child.wait_with_output().ok()?;
                        let text = String::from_utf8_lossy(&output.stdout).into_owned();
                        return parse_perldoc_output(&text);
                    } else {
                        return None;
                    }
                }
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return None,
            }
        }
    })();

    // Store in cache (skip if cache is full to avoid unbounded growth).
    if let Ok(mut guard) = cache.lock() {
        if guard.len() < MAX_CACHE_ENTRIES {
            guard.insert(name.to_string(), result.clone());
        }
    }

    result
}

/// Look up documentation for a Perl function via the system `perldoc` command.
///
/// WASM stub: always returns `None` since subprocess execution is not available.
#[cfg(target_arch = "wasm32")]
pub fn perldoc_lookup(_name: &str) -> Option<PerldocResult> {
    None
}

#[cfg(test)]
mod tests {
    use super::{get_builtin_documentation, is_safe_perldoc_name, parse_perldoc_output};

    #[test]
    fn test_get_builtin_documentation_begin() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_builtin_documentation("BEGIN").ok_or("BEGIN should have docs")?;
        assert!(
            doc.description.contains("compile time") || doc.description.contains("compile-time"),
            "BEGIN doc should mention compile time, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_builtin_documentation_end() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_builtin_documentation("END").ok_or("END should have docs")?;
        assert!(
            doc.description.contains("exit") || doc.description.contains("cleanup"),
            "END doc should mention exit or cleanup, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_builtin_documentation_check() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_builtin_documentation("CHECK").ok_or("CHECK should have docs")?;
        assert!(
            doc.description.contains("compilation") || doc.description.contains("compile"),
            "CHECK doc should mention compilation, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_builtin_documentation_init() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_builtin_documentation("INIT").ok_or("INIT should have docs")?;
        assert!(
            doc.description.contains("compilation") || doc.description.contains("before"),
            "INIT doc should mention post-compile execution, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_builtin_documentation_unitcheck() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_builtin_documentation("UNITCHECK").ok_or("UNITCHECK should have docs")?;
        assert!(
            doc.description.contains("compilation unit") || doc.description.contains("unit"),
            "UNITCHECK doc should mention compilation unit scope, got: {}",
            doc.description
        );
        Ok(())
    }

    // --- Tests for newly added builtins ---

    #[test]
    fn test_socket_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let socket_fns = [
            "socket",
            "socketpair",
            "bind",
            "connect",
            "listen",
            "accept",
            "shutdown",
            "send",
            "recv",
            "setsockopt",
            "getsockopt",
            "getsockname",
            "getpeername",
            "pipe",
        ];
        for name in &socket_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("socket builtin '{}' should have docs", name))?;
            assert!(
                !doc.signature.is_empty(),
                "socket builtin '{}' should have non-empty signature",
                name
            );
            assert!(
                !doc.description.is_empty(),
                "socket builtin '{}' should have non-empty description",
                name
            );
        }
        Ok(())
    }

    #[test]
    fn test_io_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let io_fns = [
            "fileno",
            "flock",
            "select",
            "getc",
            "readline",
            "readpipe",
            "rewinddir",
            "seekdir",
            "telldir",
            "chroot",
            "umask",
            "sysopen",
            "sysread",
            "syswrite",
            "sysseek",
            "syscall",
            "fcntl",
            "ioctl",
        ];
        for name in &io_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("I/O builtin '{}' should have docs", name))?;
            assert!(!doc.signature.is_empty(), "I/O builtin '{}' should have a signature", name);
            assert!(
                !doc.description.is_empty(),
                "I/O builtin '{}' should have a description",
                name
            );
        }
        Ok(())
    }

    #[test]
    fn test_process_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let process_fns =
            ["times", "getlogin", "getppid", "getpgrp", "setpgrp", "getpriority", "setpriority"];
        for name in &process_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("process builtin '{}' should have docs", name))?;
            assert!(
                !doc.description.is_empty(),
                "process builtin '{}' should have a description",
                name
            );
        }
        Ok(())
    }

    #[test]
    fn test_db_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let db_fns = [
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
            "gethostbyname",
            "gethostbyaddr",
            "getnetbyname",
            "getnetbyaddr",
            "getprotobyname",
            "getprotobynumber",
            "getservbyname",
            "getservbyport",
        ];
        for name in &db_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("db lookup builtin '{}' should have docs", name))?;
            assert!(
                !doc.description.is_empty(),
                "db lookup builtin '{}' should have a description",
                name
            );
        }
        Ok(())
    }

    #[test]
    fn test_ipc_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let ipc_fns = [
            "msgget", "msgctl", "msgsnd", "msgrcv", "semget", "semctl", "semop", "shmget",
            "shmctl", "shmread", "shmwrite",
        ];
        for name in &ipc_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("IPC builtin '{}' should have docs", name))?;
            assert!(
                !doc.description.is_empty(),
                "IPC builtin '{}' should have a description",
                name
            );
        }
        Ok(())
    }

    #[test]
    fn test_misc_builtins_have_docs() -> Result<(), Box<dyn std::error::Error>> {
        let misc_fns = ["pos", "reset", "study", "vec", "formline", "dump", "dbmopen", "dbmclose"];
        for name in &misc_fns {
            let doc = get_builtin_documentation(name)
                .ok_or_else(|| format!("misc builtin '{}' should have docs", name))?;
            assert!(
                !doc.description.is_empty(),
                "misc builtin '{}' should have a description",
                name
            );
        }
        Ok(())
    }

    // --- Tests for perldoc_lookup helpers ---

    #[test]
    fn test_is_safe_perldoc_name_valid() {
        assert!(is_safe_perldoc_name("print"));
        assert!(is_safe_perldoc_name("open"));
        assert!(is_safe_perldoc_name("my_function"));
        assert!(is_safe_perldoc_name("chomp"));
        assert!(is_safe_perldoc_name("AUTOLOAD"));
    }

    #[test]
    fn test_is_safe_perldoc_name_invalid() {
        assert!(!is_safe_perldoc_name(""));
        assert!(!is_safe_perldoc_name("foo/bar"));
        assert!(!is_safe_perldoc_name("../etc/passwd"));
        assert!(!is_safe_perldoc_name("foo; rm -rf /"));
        assert!(!is_safe_perldoc_name("foo bar"));
        assert!(!is_safe_perldoc_name("foo::bar")); // :: rejected (colons)
        assert!(!is_safe_perldoc_name(&"a".repeat(65))); // too long
    }

    #[test]
    fn test_parse_perldoc_output_basic() {
        let sample = "chomp VARIABLE\nchomp LIST\nchomp\n\n    This safer version of chop removes any trailing string that\n    corresponds to the current value of $/.";
        let result = parse_perldoc_output(sample);
        assert!(result.is_some(), "Should parse valid perldoc output");
        let r = result.unwrap();
        assert!(r.signature.contains("chomp"), "Signature should contain 'chomp'");
        assert!(
            r.description.contains("trailing") || r.description.contains("chop"),
            "Description should contain meaningful content, got: {}",
            r.description
        );
    }

    #[test]
    fn test_parse_perldoc_output_empty() {
        assert!(parse_perldoc_output("").is_none());
        assert!(parse_perldoc_output("   \n  \n  ").is_none());
    }

    #[test]
    fn test_parse_perldoc_output_strips_ansi() {
        // Simulate ANSI escape sequences that some perldoc versions emit
        let with_ansi = "\x1b[1mprint\x1b[0m LIST\n\nPrints a string.";
        let result = parse_perldoc_output(with_ansi);
        assert!(result.is_some(), "Should parse output with ANSI codes");
        let r = result.unwrap();
        assert!(!r.signature.contains("\x1b"), "Signature should not contain ANSI codes");
    }

    #[test]
    fn test_parse_perldoc_output_no_blank_line() {
        // Some minimal perldoc outputs have no blank separator
        let minimal = "chomp VARIABLE\n    Removes trailing newline.";
        let result = parse_perldoc_output(minimal);
        assert!(result.is_some(), "Should parse minimal perldoc output");
    }
}
