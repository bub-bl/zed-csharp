;; Indentation rules for Razor based on tree-sitter-razor grammar

;; ============================================================================
;; INDENT FOR RAZOR STRUCTURES
;; ============================================================================

;; Code blocks and sections
(razor_block) @indent
(razor_section) @indent

;; Control structures
(razor_if) @indent
(razor_else_if) @indent
(razor_else) @indent
(razor_switch) @indent
(razor_for) @indent
(razor_foreach) @indent
(razor_while) @indent
(razor_do_while) @indent
(razor_try) @indent
(razor_catch) @indent
(razor_finally) @indent
(razor_lock) @indent
(razor_compound_using) @indent

;; ============================================================================
;; INDENT FOR HTML ELEMENTS
;; ============================================================================

(element) @indent
