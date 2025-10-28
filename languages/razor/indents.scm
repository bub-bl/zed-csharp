;; Indentation rules for Razor

;; Indent after opening braces in Razor blocks
(razor_block "{" @indent)
(razor_if "{" @indent)
(razor_switch "{" @indent)
(razor_for "{" @indent)
(razor_foreach "{" @indent)
(razor_while "{" @indent)
(razor_do_while "{" @indent)
(razor_try "{" @indent)
(razor_catch "{" @indent)
(razor_finally "{" @indent)

;; Dedent at closing braces
(razor_block "}" @end)
(razor_if "}" @end)
(razor_switch "}" @end)
(razor_for "}" @end)
(razor_foreach "}" @end)
(razor_while "}" @end)
(razor_do_while "}" @end)
(razor_try "}" @end)
(razor_catch "}" @end)
(razor_finally "}" @end)
