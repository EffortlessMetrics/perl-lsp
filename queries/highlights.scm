; Syntax highlighting queries for perl-parser output
; Compatible with Tree-sitter highlighting system

; Comments
(comment) @comment

; Variables
(variable sigil: "$" @punctuation.special)
(variable sigil: "@" @type.builtin) 
(variable sigil: "%" @type.builtin)
(variable sigil: "&" @function.builtin)
(variable sigil: "*" @type.builtin)
(variable name: (_) @variable)

; Special variables
(variable name: ["_" "!" "@" "$" "?" "." "^" "~" "=" ">" "<" "," "\\" "/" "|" "+" "-" "%" ":" "&" "`" "'" "\""] @variable.builtin)

; Declarations
[(my_declaration) (our_declaration) (local_declaration) (state_declaration)] @keyword

; Literals
(number) @number
(string) @string
(string_interpolated) @string
(regex) @string.regex
(heredoc) @string
(identifier) @variable

; Keywords
["if" "elsif" "else" "unless"] @conditional
["while" "until" "for" "foreach" "do"] @repeat
["given" "when" "default"] @conditional
["try" "catch" "finally"] @exception
["return" "last" "next" "redo" "goto"] @keyword.return
["my" "our" "local" "state"] @keyword
["sub" "method"] @keyword.function
["package" "use" "no" "require"] @include
["BEGIN" "CHECK" "INIT" "END" "UNITCHECK"] @keyword

; Functions and methods
(call function: (identifier) @function.call)
(method_call method: (identifier) @method.call)
(sub name: (identifier) @function)
(method name: (identifier) @method)

; Operators
; Arithmetic
[(binary_+) (binary_-) (binary_*) (binary_/) (binary_%) (binary_**)] @operator

; Comparison  
[(binary_==) (binary_!=) (binary_<) (binary_>) (binary_<=) (binary_>=)] @operator
[(binary_lt) (binary_gt) (binary_le) (binary_ge) (binary_eq) (binary_ne)] @operator
[(binary_cmp) (binary_<=>)] @operator

; Logical
[(binary_&&) (binary_||) (binary_//) (binary_and) (binary_or) (binary_xor)] @operator
[(unary_!) (unary_not)] @operator

; Bitwise
[(binary_&) (binary_|) (binary_^) (binary_<<) (binary_>>)] @operator
[(unary_~)] @operator

; String operators
[(binary_.) (binary_x)] @operator

; Regex operators
[(binary_=~) (binary_!~)] @operator

; Assignment operators
[(assignment_assign) (assignment_+=) (assignment_-=) (assignment_*=)] @operator
[(assignment_/=) (assignment_%=) (assignment_.=) (assignment_x=)] @operator
[(assignment_&=) (assignment_|=) (assignment_^=)] @operator
[(assignment_<<=) (assignment_>>=) (assignment_&&=) (assignment_||=)] @operator
[(assignment_//=)] @operator

; Other operators
[(binary_=>) (binary_,)] @punctuation.delimiter
[(binary_->) (binary_::)] @punctuation.delimiter
[(ternary)] @operator

; Special operators
[(binary_~~)] @operator  ; Smart match
[(binary_isa)] @operator ; Type check

; Array/hash access
(binary_[] @punctuation.bracket)
(binary_{} @punctuation.bracket)

; Blocks and statements
(block) @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket

; Delimiters
";" @punctuation.delimiter
"," @punctuation.delimiter

; Package names
(package name: (identifier) @namespace)
(use module: (identifier) @namespace)
(no module: (identifier) @namespace)

; Attributes
(attributes) @attribute

; Prototypes
(prototype) @type

; Labels
(labeled_statement label: (identifier) @label)

; Error nodes
(ERROR) @error

; POD documentation
(pod) @comment.documentation

; Special constructs
(ellipsis) @punctuation.special
(diamond) @function.builtin
(undef) @constant.builtin

; Interpolation in strings
(string_interpolated
  (interpolation) @embedded)

; Phase blocks
[(BEGIN) (CHECK) (INIT) (END) (UNITCHECK)] @keyword.directive

; Modern Perl
(class name: (identifier) @type)
(defer) @keyword
(field) @variable.member

; File test operators
[(unary_-r) (unary_-w) (unary_-x) (unary_-o) (unary_-R) (unary_-W) (unary_-X) (unary_-O)] @function.builtin
[(unary_-e) (unary_-z) (unary_-s) (unary_-f) (unary_-d) (unary_-l) (unary_-p)] @function.builtin
[(unary_-S) (unary_-b) (unary_-c) (unary_-t) (unary_-u) (unary_-g) (unary_-k)] @function.builtin
[(unary_-T) (unary_-B) (unary_-M) (unary_-A) (unary_-C)] @function.builtin

; Quote-like operators
(qw) @string
(qr) @string.regex
(qx) @string.special

; Glob
(glob) @string.special

; Format
(format) @keyword

; Special literals
[(true) (false)] @constant.builtin
(v_string) @string.special