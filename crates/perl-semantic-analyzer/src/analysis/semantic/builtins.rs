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

/// Normalize a built-in name for lookup.
///
/// Perl allows calling built-ins with a `CORE::` prefix (for example
/// `CORE::length`). The semantic analyzer stores the function call text as-is,
/// so normalize here to keep builtin classification and hover docs consistent.
fn normalized_builtin_name(name: &str) -> &str {
    name.strip_prefix("CORE::").unwrap_or(name)
}

/// Check if a function name is a Perl control-flow keyword.
///
/// Returns `true` if the name is a control-flow keyword like `next`, `last`, etc.
pub(super) fn is_control_keyword(name: &str) -> bool {
    let name = normalized_builtin_name(name);
    matches!(name, "next" | "last" | "redo" | "goto" | "return" | "exit" | "die")
}

/// Check if a function name is a Perl built-in.
///
/// Returns `true` if the name matches a known Perl built-in function.
pub(super) fn is_builtin_function(name: &str) -> bool {
    let name = normalized_builtin_name(name);
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
    let name = normalized_builtin_name(name);
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
        "locked" => Some(BuiltinDoc {
            signature: ":locked",
            description: "Marks a subroutine so concurrent callers are serialized. Useful for thread-safe methods that must not run at the same time.",
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

/// Documentation entry for a Perl pragma.
///
/// Provides a brief summary and description for display in hover tooltips
/// when a user hovers over a `use strict;`, `use warnings;`, etc. statement.
pub struct PragmaDoc {
    /// One-line purpose summary
    pub summary: &'static str,
    /// Detailed description of what the pragma enables or does
    pub description: &'static str,
    /// Minimum Perl version required, if any (e.g. `"v5.10"`)
    pub version_required: Option<&'static str>,
}

/// Get hover documentation for a Perl pragma.
///
/// Returns a [`PragmaDoc`] for known pragmas used in `use <pragma>` or `no <pragma>`
/// statements, or `None` if the name is not a recognized pragma.
///
/// Pragmas covered: `strict`, `warnings`, `utf8`, `feature`, `constant`, `vars`,
/// `autodie`, `encoding`, `locale`, `parent`, `base`, `lib`, `Exporter`.
///
/// Version pragmas (`v5.36`, `5.036`, etc.) are detected by
/// [`crate::analysis::pragma::parse_perl_version`] separately.
pub fn get_pragma_documentation(name: &str) -> Option<PragmaDoc> {
    match name {
        "strict" => Some(PragmaDoc {
            summary: "Enable strict variable/subroutine/reference checking",
            description: "Restricts unsafe Perl constructs. Enables compile-time errors for \
                undeclared variables (`vars`), bareword subroutine names (`subs`), and symbolic \
                references (`refs`). Use `use strict;` to enable all three categories at once, \
                or `use strict 'vars'` for individual categories.\n\n\
                **Common usage**: Always include `use strict;` at the top of every Perl file.",
            version_required: None,
        }),
        "warnings" => Some(PragmaDoc {
            summary: "Enable runtime and compile-time warnings",
            description: "Enables a wide range of optional warnings about potentially dangerous \
                or deprecated code patterns. Categories include: `numeric`, `uninitialized`, \
                `deprecated`, `syntax`, `misc`, and many more.\n\n\
                Use `use warnings;` to enable all warnings, or `use warnings 'uninitialized'` \
                for specific categories. Use `no warnings 'once'` to suppress individual categories.\n\n\
                **Common usage**: Always pair with `use strict;`.",
            version_required: None,
        }),
        "utf8" => Some(PragmaDoc {
            summary: "Treat the source file as UTF-8 encoded",
            description: "Tells the Perl parser that the source code is encoded in UTF-8. \
                Allows Unicode identifiers, string literals, and comments in the source file. \
                Does **not** affect how STDIN/STDOUT/STDERR handle encoding — use \
                `binmode(STDOUT, ':utf8')` or `open` with `:utf8` layer for that.\n\n\
                **Common usage**: `use utf8;` at the top of files with non-ASCII identifiers \
                or string constants.",
            version_required: Some("v5.6"),
        }),
        "feature" => Some(PragmaDoc {
            summary: "Enable experimental or version-specific Perl features",
            description: "Enables named language features that are off by default. Key features:\n\
                - `say` — like `print` but appends a newline (v5.10+)\n\
                - `state` — persistent lexical variables (v5.10+)\n\
                - `signatures` — formal subroutine signatures (stable v5.36+)\n\
                - `try` — `try`/`catch` exception handling (experimental v5.34+)\n\
                - `class` — native OO with `class`/`method`/`field` (v5.38+)\n\
                - `defer` — `defer` blocks run at scope exit (v5.36+)\n\n\
                Features are also enabled implicitly by `use v5.XX;` version declarations.\n\n\
                **Example**: `use feature 'say', 'state';`",
            version_required: Some("v5.10"),
        }),
        "constant" => Some(PragmaDoc {
            summary: "Declare compile-time constants",
            description: "Creates named constants that are inlined at compile time, making them \
                more efficient than regular variables and preventing accidental reassignment.\n\n\
                **Single constant**: `use constant PI => 3.14159;`\n\
                **Multiple constants**: `use constant { MAX => 100, MIN => 0 };`\n\n\
                Constants are accessed without a sigil: `print PI;` or `print MAX;`.\n\
                They cannot be interpolated directly in strings — use `@{[PI]}` as a workaround.",
            version_required: None,
        }),
        "vars" => Some(PragmaDoc {
            summary: "Pre-declare package (global) variables",
            description: "Pre-declares package global variables so they can be used under \
                `use strict 'vars'` without a full package-qualified name. This is a legacy \
                pragma — prefer `our $var;` in modern code.\n\n\
                **Example**: `use vars qw($VERSION @EXPORT);`\n\n\
                **Modern alternative**: `our $VERSION; our @EXPORT;`",
            version_required: None,
        }),
        "autodie" => Some(PragmaDoc {
            summary: "Automatic exception throwing on system call failures",
            description: "Replaces built-in functions (`open`, `close`, `read`, `write`, \
                `system`, `exec`, etc.) with versions that automatically `die` on failure, \
                eliminating boilerplate `or die` checks.\n\n\
                **Example**: `use autodie;` — all builtins now throw on error.\n\
                **Selective**: `use autodie qw(open close);` — only specified functions.\n\n\
                Exceptions are `autodie::exception` objects with detailed failure information.",
            version_required: Some("v5.10.1"),
        }),
        "encoding" => Some(PragmaDoc {
            summary: "Set source encoding (legacy — prefer utf8 pragma)",
            description: "Specifies the character encoding of the Perl source file and optionally \
                sets default I/O encoding. This pragma is **deprecated** — prefer `use utf8;` \
                for source encoding.\n\n\
                **Example**: `use encoding 'utf8';`\n\n\
                **Preferred alternative**: `use utf8;` for source + explicit `binmode` calls \
                for I/O encoding.",
            version_required: Some("v5.6"),
        }),
        "locale" => Some(PragmaDoc {
            summary: "Enable locale-aware string operations",
            description: "Makes string comparisons, case conversion, and character classification \
                functions use the current system locale settings (LC_CTYPE, LC_COLLATE, etc.).\n\n\
                **Note**: Locale handling can cause subtle encoding issues. Prefer Unicode \
                semantics with `use utf8;` and `use feature 'unicode_strings';` where possible.\n\n\
                **Example**: `use locale;`",
            version_required: None,
        }),
        "parent" => Some(PragmaDoc {
            summary: "Establish ISA relationship with parent classes",
            description: "Sets up inheritance by loading the listed modules and pushing them \
                into `@ISA`. The modern replacement for `use base`.\n\n\
                **Example**: `use parent 'Animal';` or `use parent qw(Animal Printable);`\n\n\
                Unlike `use base`, `parent` always `require`s the parent modules and does not \
                set `$VERSION` or `@EXPORT` by default.",
            version_required: Some("v5.10.1"),
        }),
        "base" => Some(PragmaDoc {
            summary: "Establish ISA relationship (legacy — prefer parent pragma)",
            description: "Sets up inheritance by loading parent modules and updating `@ISA`. \
                Legacy alternative to `use parent`.\n\n\
                **Example**: `use base 'Animal';` or `use base qw(Animal Printable);`\n\n\
                **Preferred alternative**: `use parent qw(...);` — cleaner semantics \
                without the `$VERSION`/`@EXPORT` side-effects of `use base`.",
            version_required: None,
        }),
        "lib" => Some(PragmaDoc {
            summary: "Add directories to @INC at compile time",
            description: "Prepends directories to `@INC` so that subsequent `use` and `require` \
                statements can find modules there.\n\n\
                **Example**: `use lib '/path/to/modules';`\n\
                **Relative path**: `use lib 'lib';` — adds `./lib` to `@INC`.\n\n\
                Often used in test files: `use lib 't/lib';`",
            version_required: None,
        }),
        "Exporter" => Some(PragmaDoc {
            summary: "Default symbol exporter for Perl modules",
            description: "Provides the standard mechanism for modules to export symbols into \
                the caller's namespace. Configure `@EXPORT` and `@EXPORT_OK` to control \
                what gets exported.\n\n\
                **Typical usage**:\n\
                ```perl\n\
                use Exporter 'import';\n\
                our @EXPORT_OK = qw(helper_fn);\n\
                ```\n\n\
                Or via inheritance: `use parent 'Exporter';`",
            version_required: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_builtin_documentation, get_pragma_documentation, is_builtin_function,
        is_control_keyword,
    };

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

    #[test]
    fn test_core_prefixed_builtin_lookups() -> Result<(), Box<dyn std::error::Error>> {
        assert!(is_builtin_function("CORE::length"), "CORE::length should be recognized");
        assert!(is_control_keyword("CORE::die"), "CORE::die should be recognized as control");

        let doc =
            get_builtin_documentation("CORE::length").ok_or("CORE::length should have docs")?;
        assert!(
            doc.signature.contains("length"),
            "CORE::length should resolve to length docs, got: {}",
            doc.signature
        );
        Ok(())
    }

    // ── pragma documentation tests ──────────────────────────────────────────

    #[test]
    fn test_get_pragma_documentation_strict() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("strict").ok_or("strict should have docs")?;
        assert!(
            doc.description.contains("strict") || doc.description.contains("variable"),
            "strict doc should describe variable checking, got: {}",
            doc.description
        );
        assert!(
            doc.summary.contains("strict"),
            "strict summary should mention strict, got: {}",
            doc.summary
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_warnings() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("warnings").ok_or("warnings should have docs")?;
        assert!(
            doc.description.contains("warning"),
            "warnings doc should describe warnings, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_utf8() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("utf8").ok_or("utf8 should have docs")?;
        assert!(
            doc.description.contains("UTF-8") || doc.description.contains("Unicode"),
            "utf8 doc should mention UTF-8 or Unicode, got: {}",
            doc.description
        );
        assert_eq!(
            doc.version_required,
            Some("v5.6"),
            "utf8 requires v5.6, got: {:?}",
            doc.version_required
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_feature() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("feature").ok_or("feature should have docs")?;
        assert!(
            doc.description.contains("say") || doc.description.contains("feature"),
            "feature doc should mention specific features, got: {}",
            doc.description
        );
        assert!(doc.version_required.is_some(), "feature pragma should have a version requirement");
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_constant() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("constant").ok_or("constant should have docs")?;
        assert!(
            doc.description.contains("constant") || doc.description.contains("compile"),
            "constant doc should mention constants, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_autodie() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("autodie").ok_or("autodie should have docs")?;
        assert!(
            doc.description.contains("die") || doc.description.contains("exception"),
            "autodie doc should mention die or exceptions, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_parent() -> Result<(), Box<dyn std::error::Error>> {
        let doc = get_pragma_documentation("parent").ok_or("parent should have docs")?;
        assert!(
            doc.description.contains("ISA") || doc.description.contains("inherit"),
            "parent doc should mention ISA or inheritance, got: {}",
            doc.description
        );
        Ok(())
    }

    #[test]
    fn test_get_pragma_documentation_unknown_returns_none() {
        assert!(
            get_pragma_documentation("SomeArbitraryModule").is_none(),
            "Non-pragma module should return None"
        );
        assert!(
            get_pragma_documentation("Moose").is_none(),
            "Moose is not a pragma and should return None"
        );
    }
}
