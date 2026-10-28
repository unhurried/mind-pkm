# Design

`Editor::pasted` converted `content.html` with htmd whenever it was present and the text differed from the editor's own last copy. It now asks the active block first: in a code-like block (`BlockKind::is_code`) neither the table nor the Markdown conversion runs, and the text flavor goes in as it is. A clipboard with an HTML flavor only (the text flavor missing) goes through a small HTML-to-text conversion (`html::to_plain_text`): the end of a block element and `<br>` become line breaks, `style` / `script` / comments are dropped, character references are decoded.

Files and images are handled as before. The whole-page source mode is unchanged.
