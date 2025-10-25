;; XML/csproj syntax highlighting

;; Tags
(tag_name) @tag
(element) @tag

;; Attributes
(attribute_name) @property
(attribute_value) @string

;; Comments
(comment) @comment

;; Text content
(text) @text

;; Special elements
(element (tag_name) @namespace (#match? @namespace "^[A-Z]"))
