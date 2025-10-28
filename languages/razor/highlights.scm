;; Razor syntax highlighting based on tree-sitter-razor grammar

;; Comments - always highlight
(razor_comment) @comment
(html_comment) @comment
(comment) @comment

;; Razor @ symbols and markers
(razor_escape) @string.escape

;; Razor directives (Page-level) - these are keywords specific to Razor
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

;; Razor sections and blocks - highlight the structure
(razor_section) @keyword
(razor_block) @keyword

;; Razor control structures - structure keywords
(razor_if) @keyword
(razor_else_if) @keyword
(razor_else) @keyword
(razor_switch) @keyword
(razor_for) @keyword
(razor_foreach) @keyword
(razor_while) @keyword
(razor_do_while) @keyword
(razor_try) @keyword
(razor_catch) @keyword
(razor_finally) @keyword

;; Razor expressions - mark as punctuation to show Razor context
(razor_explicit_expression) @punctuation.bracket
(razor_implicit_expression) @punctuation.bracket
(razor_await_expression) @keyword
(razor_lock) @keyword
(razor_compound_using) @keyword
(explicit_line_transition) @punctuation
