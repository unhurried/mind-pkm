# Design

## Guessed heights

The heights of the blocks are kept by block (its source) for a width and zoom of the page. A block whose height is not known at the page's width and zoom takes a guess: its height at the width and zoom it was last measured at, scaled as an estimate from its source scales (the estimate counts its lines, wrapped at an average character width, at the font size of the zoom), or that estimate when it was never measured. A block that may show math or a diagram keeps its height from before a render came in until it is measured again.

## Measuring a few at a time

Blocks get widgets when they are near the visible part, when they are being edited or scrolled to - and, of those whose height is not known, the nearest to the visible part, as many as hold about 4 KB of source together (at least one), with every view of the page. While blocks are left to measure, a view asks for another message once it is drawn (a subscription would not do: the subscriptions are asked for before the views are built), so that the page measures a batch for every frame; when a batch measured nothing, it stops. Each message builds the windows again, so measuring takes the windows' usual work plus that of the batch; the page can be scrolled and edited meanwhile.

## Keeping the top in place

The page column tells where each of its widgets was laid out. Right after the page's content is laid out, before it is drawn, the page compares that with the previous layout: the first block (or the gap above it) that reaches below the top of the visible part moved by some amount, and the page scrolls by as much. When that block is gone - an edit gave it another key - the nearest block above it that is still there is used instead, so that an edit at the top of the visible part moves the text below it, as before. After a change of width or zoom, the top stays as far into the block for its height.

The blocks near the visible part are found from the block at its top when the page last told where it was, at its new place: a zoom or a measured block above does not leave the visible part without widgets.

## Not changed

Scrolling to keep the cursor in view, to a heading or to a match is done after the page was kept in place, and takes precedence. The source mode has no blocks and keeps its scroll position as before.
