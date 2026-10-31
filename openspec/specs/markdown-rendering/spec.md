# markdown-rendering Specification

## Purpose
What the built-in editor understands of Markdown and HTML and how it shows each element when it is not being edited: CommonMark with GitHub's extensions and Typora's additions, including math, Mermaid diagrams and images.

## Requirements

### Requirement: Block elements
The editor SHALL render paragraphs, headings of levels 1-6 (ATX and setext), block quotes (nested ones included), bullet lists (`*`, `+`, `-`), ordered lists (numbered from their start number), task lists (`- [ ]`, `- [x]`, with a box), code blocks (fenced and indented), thematic breaks (`***`, `---`, `___`), footnote definitions and HTML blocks. An empty list item (also a task item) SHALL be as tall as an item with a line of text.

#### Scenario: Ordered list numbering
- **WHEN** a list is written `3.`, `1.`, `1.`
- **THEN** its items are shown numbered 3, 4, 5

#### Scenario: Nested quote
- **WHEN** a block reads `> a` and `> > b`
- **THEN** `b` is shown in a quote inside the quote holding `a`

#### Scenario: Empty task item
- **WHEN** a task list has an empty item (`- [ ]`) between items with text
- **THEN** the empty item is shown with its box, in a row as tall as the others, so the items stay evenly spaced

### Requirement: Code highlighting
A fenced code block with a language SHALL be shown in the code font with syntax highlighting for that language; an unknown or missing language SHALL be shown without highlighting.

#### Scenario: Rust code
- **WHEN** a ` ```rust ` block contains `fn main() {}`
- **THEN** `fn` is colored as a keyword

### Requirement: Tables
GFM tables SHALL be rendered as a grid with a header row, each column aligned left, center or right as its delimiter row says (`:---`, `:---:`, `---:`).

#### Scenario: Centered column
- **WHEN** a table's delimiter row is `| --- | :---: |`
- **THEN** the second column's cells are centered

### Requirement: Front matter and table of contents
YAML front matter (`---` ... `---` at the start of the file) SHALL be shown as a code-styled box. A paragraph consisting of `[toc]` (any case) SHALL be shown as a table of contents listing the document's headings, each clickable to scroll to it.

#### Scenario: Table of contents
- **WHEN** a page contains `[toc]` and the headings `A` and `B`
- **THEN** a list of `A` and `B` is shown where `[toc]` is

### Requirement: Span elements
The editor SHALL render italic (`*a*`, `_a_`), bold (`**a**`, `__a__`), strikethrough (`~~a~~`), inline code, subscript (`~a~`) and superscript (`^a^`) as smaller text, highlight (`==a==`) with a highlight background, underline (`<u>a</u>`), emoji shortcodes (`:smile:`, GitHub's names) as the emoji, footnote references as superscript numbers linked to their definition, and line breaks inside a paragraph (a single newline included) as line breaks.

#### Scenario: Emoji shortcode
- **WHEN** a paragraph contains `:smile:`
- **THEN** it is shown as 😄

#### Scenario: Unknown shortcode
- **WHEN** a paragraph contains `:not-an-emoji:`
- **THEN** it is shown as written

### Requirement: Links
Inline links (`[t](url "title")`), reference links (`[t][id]` with `[id]: url`), links to headings (`[t](#anchor)`, the anchor being the heading text in lower case with spaces as `-`), autolinks (`<url>`) and bare `http://`, `https://` and `www.` URLs SHALL be shown as links. A link destination containing spaces without angle brackets (as in `[assets](My Page.assets)`) SHALL be accepted as a link. Link reference definitions SHALL be shown dimmed as written.

#### Scenario: Destination with spaces
- **WHEN** a page starts with `[assets](My Page.assets)`
- **THEN** `assets` is shown as a link to `My Page.assets`

#### Scenario: Bare URL
- **WHEN** a paragraph contains `see https://example.com now`
- **THEN** `https://example.com` is shown as a link

### Requirement: Inline HTML
Inline HTML SHALL be kept in the file as written. `<u>`, `<s>`, `<del>`, `<b>`, `<strong>`, `<i>`, `<em>`, `<mark>`, `<sub>`, `<sup>`, `<kbd>`, `<br>`, `<span style="color: ...">` and `<img src="..." width="...">` SHALL be rendered; `<iframe>`, `<video>`, `<audio>` and `<embed>` SHALL be shown as a placeholder naming the element with a link to its source; any other tag SHALL be shown dimmed as written.

#### Scenario: Colored text
- **WHEN** a paragraph contains `<span style="color:red">warning</span>`
- **THEN** `warning` is shown in red

#### Scenario: Embedded video
- **WHEN** a page contains `<iframe src="https://www.youtube.com/embed/x"></iframe>`
- **THEN** a placeholder reading iframe with a link to that URL is shown, and the tag stays in the file unchanged

### Requirement: Mermaid diagrams
A ` ```mermaid ` code block SHALL be shown as the diagram it describes. A diagram that cannot be rendered SHALL show the error below the block's source instead, keeping the source. While the block is being edited, its diagram (or the error) SHALL be shown below its source, following the source as it changes.

#### Scenario: Flowchart
- **WHEN** a block reads ` ```mermaid `, `flowchart LR`, `A --> B`, ` ``` `
- **THEN** a diagram with boxes A and B joined by an arrow is shown

#### Scenario: Syntax error
- **WHEN** a mermaid block contains `flowchart LR` and `A -->`
- **THEN** its source is shown with an error message below it

#### Scenario: Editing a diagram
- **WHEN** the cursor is in a block reading ` ```mermaid `, `graph TD`, `A-->B`, ` ``` `
- **THEN** the diagram with A and B is shown below the source being edited

### Requirement: Math
Inline math (`$...$`) and math blocks (`$$` ... `$$`) SHALL be typeset from their LaTeX. An expression that cannot be typeset SHALL show its source with the error, inline as code-styled text and for blocks below the source.

#### Scenario: Math block
- **WHEN** a math block contains `\frac{1}{2}`
- **THEN** a fraction with 1 over 2 is shown

### Requirement: Images
`![alt](src "title")` and `<img src>` SHALL show the image file (PNG, JPEG, GIF, BMP, WebP, ICO, TIFF, SVG), scaled down to the width of the document when wider. A missing file SHALL show the alt text with a broken-image mark; `http`/`https` images SHALL NOT be downloaded but shown as their alt text with a link to the URL.

#### Scenario: Image in the assets folder
- **WHEN** the page `Trip.md` contains `![](Trip.assets/map.png)` and that file exists
- **THEN** the image is shown

#### Scenario: Remote image
- **WHEN** a page contains `![logo](https://example.com/logo.png)`
- **THEN** `logo` is shown with a link to the URL and nothing is downloaded

### Requirement: Image locations
A relative image `src` SHALL be resolved against the page's directory with percent-escapes decoded, a `src` starting with `/` against the front matter's `typora-root-url` (itself relative to the page's directory or absolute), and `file:` URIs and absolute paths as they are.

#### Scenario: Escaped name
- **WHEN** the page contains `![](My%20Page.assets/a.png)` and `My Page.assets/a.png` exists
- **THEN** the image is shown

#### Scenario: Root-relative image
- **WHEN** the front matter has `typora-root-url: ../site` and the page contains `![](/img/a.png)`
- **THEN** `../site/img/a.png` (relative to the page) is shown
