;; Bracket matching for Razor

;; C# and Razor braces
("{" @open "}" @close)
("[" @open "]" @close)
("(" @open ")" @close)

;; HTML/Razor tags
("<" @open ">" @close)

;; HTML comments
("<!--" @open "-->" @close)

;; Razor comments
("@*" @open "*@" @close)
