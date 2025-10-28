;; Comprehensive Razor syntax highlighting
;; Based on tree-sitter-razor grammar

;; ============================================================================
;; COMMENTS
;; ============================================================================

(razor_comment) @comment
(html_comment) @comment
(comment) @comment

;; ============================================================================
;; DIRECTIVES - These are Razor-specific, always highlight as keywords
;; ============================================================================

;; Page-level directives
(razor_page_directive) @keyword
(razor_using_directive) @keyword
(razor_model_directive) @keyword
(razor_inject_directive) @keyword
(razor_layout_directive) @keyword
(razor_inherits_directive) @keyword
(razor_attribute_directive) @keyword
(razor_implements_directive) @keyword
(razor_namespace_directive) @keyword
(razor_typeparam_directive) @keyword
(razor_preservewhitespace_directive) @keyword
(razor_rendermode_directive) @keyword

;; ============================================================================
;; CODE BLOCKS AND SECTIONS
;; ============================================================================

(razor_block) @keyword
(razor_section) @keyword

;; ============================================================================
;; CONTROL STRUCTURES - Highlight as keywords since they're Razor-specific
;; ============================================================================

;; Conditionals
(razor_if) @keyword
(razor_else_if) @keyword
(razor_else) @keyword

;; Switches
(razor_switch) @keyword
(razor_switch_case) @keyword
(razor_switch_default) @keyword

;; Loops
(razor_for) @keyword
(razor_foreach) @keyword
(razor_while) @keyword
(razor_do_while) @keyword

;; Try/Catch/Finally
(razor_try) @keyword
(razor_catch) @keyword
(razor_finally) @keyword

;; Other control structures
(razor_lock) @keyword
(razor_compound_using) @keyword

;; ============================================================================
;; EXPRESSIONS AND TRANSITIONS
;; ============================================================================

;; Razor expression markers
(razor_explicit_expression) @punctuation.bracket
(razor_implicit_expression) @punctuation.bracket
(razor_await_expression) @keyword
(explicit_line_transition) @keyword

;; Escape sequences
(razor_escape) @string.escape

;; ============================================================================
;; HTML ELEMENTS
;; ============================================================================

(element) @tag

;; ============================================================================
;; C# LITERALS AND KEYWORDS (inherited from C# grammar through injection)
;; ============================================================================

;; Strings
(string_literal) @string
(verbatim_string_literal) @string
(raw_string_literal) @string

;; Numbers
(integer_literal) @number
(real_literal) @number

;; Constants
(boolean_literal) @constant
(null_literal) @constant
(character_literal) @string
