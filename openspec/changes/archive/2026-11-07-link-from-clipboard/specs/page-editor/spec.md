## MODIFIED Requirements

### Requirement: Format shortcuts
Ctrl+B, Ctrl+I, Ctrl+U, Ctrl+Shift+` and Alt+Shift+5 SHALL toggle bold (`**`), italic (`*`), underline (`<u>`), inline code (`` ` ``) and strikethrough (`~~`) around the selection, or around the word at the cursor when nothing is selected; applying one where it already applies SHALL remove it. Ctrl+K SHALL insert a link (see Inserting links); Ctrl+Shift+I SHALL insert images (see image-insertion).

#### Scenario: Bold and back
- **WHEN** `word` is selected and Ctrl+B is pressed twice
- **THEN** the text is `**word**` after the first press and `word` after the second

## ADDED Requirements

### Requirement: Inserting links
Ctrl+K SHALL insert a link `[text](url)` around the selection with the cursor in the URL. When the clipboard holds a URL (`scheme://...`, `mailto:`, `file:`) or a file path (`C:\...`, `\\server\...`, `/...`, `./...`, `../...`), Ctrl+K SHALL make it the URL instead, with the cursor after the link, or in the empty link text when nothing is selected. A path in double quotes SHALL be taken without them, its backslashes written as `/`.

#### Scenario: Link from a selection
- **WHEN** `docs` is selected, the clipboard holds no URL or path and Ctrl+K is pressed
- **THEN** the text becomes `[docs]()` with the cursor between the parentheses

#### Scenario: Link to a URL on the clipboard
- **WHEN** `docs` is selected, the clipboard holds `https://example.com/docs` and Ctrl+K is pressed
- **THEN** the text becomes `[docs](https://example.com/docs)` with the cursor after it

#### Scenario: Link to a URL without a selection
- **WHEN** nothing is selected, the clipboard holds `https://example.com` and Ctrl+K is pressed
- **THEN** `[](https://example.com)` is inserted with the cursor between the brackets

#### Scenario: Link to a copied path
- **WHEN** `report` is selected, the clipboard holds `"C:\Work\report.pdf"` and Ctrl+K is pressed
- **THEN** the text becomes `[report](C:/Work/report.pdf)`
