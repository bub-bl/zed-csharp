;; Razor syntax highlighting

;; Comments
(razor_comment) @comment
(html_comment) @comment

;; HTML elements
(element) @tag

;; Razor sections and blocks
(razor_section) @keyword
(razor_block) @keyword

;; Razor control structures
(razor_if) @keyword
(razor_switch) @keyword
(razor_for) @keyword
(razor_foreach) @keyword
(razor_while) @keyword
(razor_do_while) @keyword
(razor_try) @keyword
(razor_catch) @keyword
(razor_finally) @keyword

;; Razor expressions and directives
(razor_explicit_expression) @variable
(razor_implicit_expression) @variable
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

;; C# inherited highlighting (strings, numbers, etc.)
(string_literal) @string
(integer_literal) @number
(real_literal) @number
(boolean_literal) @constant
