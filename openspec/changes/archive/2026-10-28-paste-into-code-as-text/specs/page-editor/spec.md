## ADDED Requirements

### Requirement: Pasting into code
Inside a code block (and the other code-like blocks: math, front matter, HTML) Ctrl+V SHALL paste the clipboard's plain text as it is, without converting an HTML flavor to Markdown. A clipboard holding HTML only SHALL paste the text of that HTML, one line per block element.

#### Scenario: Pasting code into a code block
- **WHEN** code copied from VS Code (HTML and plain text) is pasted into a code block
- **THEN** the plain text is inserted without backslash escapes or extra blank lines

#### Scenario: Pasting HTML only into a code block
- **WHEN** a clipboard holding only `<div><span>let a_b = c*d;</span></div><div><span>x &lt; y</span></div>` is pasted into a code block
- **THEN** `let a_b = c*d;` and `x < y` are inserted on two lines
