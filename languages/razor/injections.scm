;; Language injection for Razor - C# and HTML hybrid support
;; Injects C#, HTML, CSS, and JavaScript into appropriate Razor constructs

;; ============================================================================
;; C# INJECTION FOR CODE BLOCKS
;; ============================================================================

;; Code block: @{ ... }
(razor_block) @injection.content
(#set! injection.language "c_sharp")

;; ============================================================================
;; C# INJECTION FOR DIRECTIVES
;; ============================================================================

;; @page, @model, @layout, @inherits, @implements, @namespace, @typeparam, @rendermode
(razor_page_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_model_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_layout_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_inherits_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_implements_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_namespace_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_typeparam_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_rendermode_directive) @injection.content
(#set! injection.language "c_sharp")

;; @using, @inject, @attribute
(razor_using_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_inject_directive) @injection.content
(#set! injection.language "c_sharp")

(razor_attribute_directive) @injection.content
(#set! injection.language "c_sharp")

;; @preservewhitespace
(razor_preservewhitespace_directive) @injection.content
(#set! injection.language "c_sharp")

;; ============================================================================
;; C# INJECTION FOR CONTROL STRUCTURES
;; ============================================================================

;; @if / @else if / @else
(razor_if) @injection.content
(#set! injection.language "c_sharp")

(razor_else_if) @injection.content
(#set! injection.language "c_sharp")

(razor_else) @injection.content
(#set! injection.language "c_sharp")

;; @switch
(razor_switch) @injection.content
(#set! injection.language "c_sharp")

(razor_switch_case) @injection.content
(#set! injection.language "c_sharp")

(razor_switch_default) @injection.content
(#set! injection.language "c_sharp")

;; @for
(razor_for) @injection.content
(#set! injection.language "c_sharp")

;; @foreach
(razor_foreach) @injection.content
(#set! injection.language "c_sharp")

;; @while
(razor_while) @injection.content
(#set! injection.language "c_sharp")

;; @do ... while
(razor_do_while) @injection.content
(#set! injection.language "c_sharp")

;; @try / @catch / @finally
(razor_try) @injection.content
(#set! injection.language "c_sharp")

(razor_catch) @injection.content
(#set! injection.language "c_sharp")

(razor_finally) @injection.content
(#set! injection.language "c_sharp")

;; @lock
(razor_lock) @injection.content
(#set! injection.language "c_sharp")

;; @using (scope)
(razor_compound_using) @injection.content
(#set! injection.language "c_sharp")

;; ============================================================================
;; C# INJECTION FOR EXPRESSIONS
;; ============================================================================

;; @( ... ) - explicit expression
(razor_explicit_expression) @injection.content
(#set! injection.language "c_sharp")

;; @property - implicit expression
(razor_implicit_expression) @injection.content
(#set! injection.language "c_sharp")

;; @await expression
(razor_await_expression) @injection.content
(#set! injection.language "c_sharp")

;; ============================================================================
;; HTML INJECTION FOR ELEMENTS
;; ============================================================================

;; HTML elements and content
(element) @injection.content
(#set! injection.language "html")

;; Razor section content (treated as HTML/markup)
(razor_section) @injection.content
(#set! injection.language "html")
