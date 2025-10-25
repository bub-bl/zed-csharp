;; Outline for csproj files

;; Project elements
(element (tag_name) @name (#eq? @name "Project")) @item

;; PropertyGroup sections
(element (tag_name) @name (#eq? @name "PropertyGroup")) @item

;; ItemGroup sections
(element (tag_name) @name (#eq? @name "ItemGroup")) @item

;; Target sections
(element (tag_name) @name (#eq? @name "Target")) @item
