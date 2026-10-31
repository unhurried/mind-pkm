# Design

The margins are outside the column, so no widget there takes the press. The widget that already wraps the column (it tells where a press landed) now also recognizes a left press beside the column, at a height inside it and inside the visible page, and replays it - a pointer move, then the press - at the column's left edge at that height. Everything below handles it as a click on that edge: text puts the cursor at the start of the line, a gap goes to the nearer block, the active block's editor places the caret at the start of the line. A drag started in the margin continues as a drag from there.

Only the rendered page is affected; in source mode the text editor fills the page.
