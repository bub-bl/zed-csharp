;; Outline for Razor files

;; Functions
(csharp_function_declaration name: (identifier) @name) @item

;; Types/Classes
(csharp_class_declaration name: (identifier) @name) @item

;; Components (Razor components)
(component_definition name: (identifier) @name) @item
