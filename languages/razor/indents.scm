;; Indentation rules for Razor based on tree-sitter-razor grammar

;; Indent after Razor control structures (they have braces)
(razor_block) @indent
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
(razor_section) @indent
(razor_lock) @indent
(razor_compound_using) @indent

;; Indent in HTML elements
(element) @indent
