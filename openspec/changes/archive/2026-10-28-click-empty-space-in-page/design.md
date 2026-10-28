# Design

A press only reaches a block when it lands on the block's own area, and two kinds of area were too small: the spacing between blocks was an empty `space` (a press there only took the focus from the active text editor), and a block's box shrank to its content, so the right of an empty item or of a line made of several widgets (text and math) was outside it.

The column now makes each block's box as wide as the page, and the spacing is two halves that press at the end of the entry above and the start of the entry below (the active region and a new paragraph count as entries). A line of a paragraph with math, images or embeds is as wide as the page too and presses at the end of its last inline; the pieces of text, links and images inside keep their own handlers.
