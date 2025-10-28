;; Language injection for Razor - C# and HTML hybrid support

;; Inject C# for code blocks and control structures
(razor_block) @injection.content
(#set! injection.language "c_sharp")

(razor_if) @injection.content
(#set! injection.language "c_sharp")

(razor_else_if) @injection.content
(#set! injection.language "c_sharp")

(razor_else) @injection.content
(#set! injection.language "c_sharp")

(razor_switch) @injection.content
(#set! injection.language "c_sharp")

(razor_for) @injection.content
(#set! injection.language "c_sharp")

(razor_foreach) @injection.content
(#set! injection.language "c_sharp")

(razor_while) @injection.content
(#set! injection.language "c_sharp")

(razor_do_while) @injection.content
(#set! injection.language "c_sharp")

(razor_try) @injection.content
(#set! injection.language "c_sharp")

(razor_catch) @injection.content
(#set! injection.language "c_sharp")

(razor_finally) @injection.content
(#set! injection.language "c_sharp")

(razor_lock) @injection.content
(#set! injection.language "c_sharp")

(razor_compound_using) @injection.content
(#set! injection.language "c_sharp")

;; Inject C# for directives
(razor_page_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_using_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_model_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_inject_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_attribute_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_namespace_directive) @injection.content
(#set! injection.language "c_sharp")

;; Inject C# for expressions
(razor_explicit_expression) @injection.content
(#set! injection.language "c_sharp")

(razor_implicit_expression) @injection.content
(#set! injection.language "c_sharp")

(razor_await_expression) @injection.content
(#set! injection.language "c_sharp")
