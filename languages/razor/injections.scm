; Inject C# for Razor code blocks and control structures
(razor_block) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_if) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_switch) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_for) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_foreach) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_while) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_do_while) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_try) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

; Inject C# for expressions
(razor_explicit_expression) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)

(razor_implicit_expression) @injection.content
(#set! injection.language "c_sharp")
(#set! injection.include-children)
