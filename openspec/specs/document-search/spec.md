# document-search Specification

## Purpose
Indexed full-text search over the document directory (F6): finding pages and attachments by keywords in their content or file name, with Japanese-aware matching, and jumping from a result to its mind map node.

## Requirements

### Requirement: Indexed formats
The index SHALL cover every file under the document directory except those inside directories whose name starts with `.`: the content of Markdown and plain-text files (`.md`, `.markdown`, `.txt`), PDF files, and `.docx`, `.xlsx` and `.pptx` files, and the file name of every file. A file that cannot be parsed SHALL be indexed by name only, and content beyond two million characters SHALL be ignored.

#### Scenario: Broken PDF
- **WHEN** the document directory holds a corrupt `report.pdf`
- **THEN** indexing completes and `report.pdf` is still found by the keyword `report`

### Requirement: Keyword matching
A query SHALL be split into keywords at whitespace, including the full-width space; a file SHALL match when every keyword matches its content or its file name (AND). Keywords SHALL be literal text, never query syntax, and a keyword of several words SHALL match as a phrase.

#### Scenario: AND across keywords
- **WHEN** the query is `東京　空港`
- **THEN** only files containing both `東京` and `空港` are listed

### Requirement: Japanese-aware tokenization
Matching SHALL be case-insensitive, fold full-width and half-width forms, and use Japanese morphological analysis, so that a word matches only as a word and inflected forms match their dictionary form. File names SHALL also be split at case changes and between letters and digits.

#### Scenario: No partial-word match
- **WHEN** the query is `京都` and a page mentions only `東京都`
- **THEN** the page is not listed

#### Scenario: Inflected form
- **WHEN** the query is `読む` and a page contains `読んだ`
- **THEN** the page is listed

#### Scenario: File name parts
- **WHEN** the query is `report` and the directory holds `QuarterlyReport2024.txt`
- **THEN** that file is listed

### Requirement: Scope and order
The search SHALL be limited to pages (the default), attachments, or both, a page being a `.md` file outside any `<page>.assets/` directory and everything else an attachment. Results SHALL be ordered by last-modified time, newest first (the default), or by relevance, file-name matches ranking above content-only matches. At most 100 results SHALL be listed. Changing the scope or order SHALL re-run the current query.

#### Scenario: Attachments only
- **WHEN** the scope is Attachments and the keyword appears in `P.assets/memo.txt` and in `P.md`
- **THEN** only `P.assets/memo.txt` is listed

### Requirement: Result list
Each result SHALL show the file's path relative to the document directory, its last-modified time (`yy/MM/dd HH:mm`), and a one-line snippet of up to about 160 characters around the match with the matched terms highlighted, or the start of the content when only the name matched.

#### Scenario: Highlighted snippet
- **WHEN** the query `tokyo` matches `notes about 空港 and tokyo`
- **THEN** the snippet shows that text with `tokyo` highlighted

### Requirement: Opening a result
Double-click, or Enter on the selected result, SHALL open the file with its default application, and SHALL report a file that no longer exists instead.

#### Scenario: Deleted since indexing
- **WHEN** a result's file was deleted and the result is opened
- **THEN** `file no longer exists: <path>` is shown in the panel

### Requirement: Jumping to the node
Ctrl+Enter or Select Node SHALL select and center the node linking the result's file, else the node linking its nearest containing directory, else - for a file under `<page>.assets/` - the page's node, unfolding its ancestors and switching to the show-all filter when the current filter hides it.

#### Scenario: Jumping to the page of an attachment
- **WHEN** Select Node is used on the result `P.assets/memo.txt` and node `P` links `P.md`
- **THEN** node `P` is selected and centered

### Requirement: Index location
The index SHALL be kept in `<docDir>/.search-index/tantivy/`, with `<docDir>/.search-index/tantivy.meta` recording a schema version and each indexed file's modification time. The Freeplane add-on's Lucene index (`.search-index/lucene/`, `.search-index/files.meta`) SHALL be left untouched.

#### Scenario: Used next to the add-on
- **WHEN** the index is updated in a document directory that also holds the add-on's index
- **THEN** `.search-index/lucene/` and `.search-index/files.meta` are unchanged

### Requirement: Index updates
Index updates SHALL be incremental (re-indexing only new and modified files and dropping deleted ones), SHALL rebuild everything when the recorded schema version differs or the index is missing, and SHALL run in the background when the search panel opens, on F12, and with the periodic refresh.

#### Scenario: Deleted file
- **WHEN** an indexed file is deleted and the index is updated
- **THEN** the file is no longer found

#### Scenario: Index removed by hand
- **WHEN** `.search-index/tantivy/` is deleted and the index is updated
- **THEN** every file is indexed again

### Requirement: Search panel
F6 SHALL show the search panel under the map with the keyword box focused, keeping the previous query and results when it is shown again; Esc or Close SHALL hide it. Searching while the index is being updated SHALL work against the index on disk.

#### Scenario: Reopening the panel
- **WHEN** the panel is closed after a search and F6 is pressed again
- **THEN** the panel shows the same query and results
