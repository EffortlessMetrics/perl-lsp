; an injections.scm file for nvim-treesitter
((comment) @injection.content
 (#set! injection.language "comment"))
 
((pod) @injection.content
 (#set! injection.language "pod"))

((substitution_regexp
  (replacement) @injection.content
  (substitution_regexp_modifiers) @_modifiers)
    ; match if there's a single `e` in the modifiers list
  (#match? @_modifiers "e")
  (#not-match? @_modifiers "e.*e")
  (#set! injection.language "perl"))

; Inline::C / Inline::CPP heredocs embed C or C++ source in a Perl file.
;
; Keep the detection explicit and narrow: only the existing Inline package
; form with a bareword language argument and heredoc body is recognized here.
((source_file
  (use_statement
    (package) @inline.package
    (list_expression
      (autoquoted_bareword) @inline.language
      (heredoc_token)))
  (heredoc_content) @injection.content)
 (#eq? @inline.package "Inline")
 (#eq? @inline.language "C")
 (#set! injection.language "c"))

((source_file
  (use_statement
    (package) @inline.package
    (list_expression
      (autoquoted_bareword) @inline.language
      (heredoc_token)))
  (heredoc_content) @injection.content)
 (#eq? @inline.package "Inline")
 (#eq? @inline.language "CPP")
 (#set! injection.language "cpp"))
