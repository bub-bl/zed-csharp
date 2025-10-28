;; Text objects for Razor (Vim navigation/text selection)

;; Code blocks and sections as functions
(razor_block) @function.around
(razor_section) @function.around

;; Control structures as functions
(razor_if) @function.around
(razor_switch) @function.around
(razor_for) @function.around
(razor_foreach) @function.around
(razor_while) @function.around
(razor_do_while) @function.around
(razor_try) @function.around

;; Directives for navigation
(razor_page_directive) @function.around
(razor_model_directive) @function.around
(razor_layout_directive) @function.around

;; HTML elements
(element) @function.around

;; Comments
(razor_comment) @comment.around
(html_comment) @comment.around

;; Expressions for fine-grained selection
(razor_explicit_expression) @function.around
(razor_implicit_expression) @function.around
