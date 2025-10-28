;; Text objects for Razor (Vim navigation/text selection)

;; ============================================================================
;; CODE BLOCKS AND SECTIONS AS FUNCTIONS
;; ============================================================================

(razor_block) @function.around
(razor_section) @function.around

;; ============================================================================
;; CONTROL STRUCTURES AS FUNCTIONS
;; ============================================================================

(razor_if) @function.around
(razor_switch) @function.around
(razor_for) @function.around
(razor_foreach) @function.around
(razor_while) @function.around
(razor_do_while) @function.around
(razor_try) @function.around

;; ============================================================================
;; DIRECTIVES FOR NAVIGATION
;; ============================================================================

(razor_page_directive) @function.around
(razor_model_directive) @function.around
(razor_layout_directive) @function.around
(razor_block) @function.around

;; ============================================================================
;; HTML ELEMENTS
;; ============================================================================

(element) @function.around

;; ============================================================================
;; COMMENTS
;; ============================================================================

(razor_comment) @comment.around
(html_comment) @comment.around

;; ============================================================================
;; EXPRESSIONS FOR FINE-GRAINED SELECTION
;; ============================================================================

(razor_explicit_expression) @function.around
(razor_implicit_expression) @function.around
