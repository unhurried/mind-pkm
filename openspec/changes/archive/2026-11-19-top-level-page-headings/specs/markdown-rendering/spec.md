## MODIFIED Requirements

### Requirement: Front matter and table of contents
YAML front matter (`---` ... `---` at the start of the file) SHALL be shown as a code-styled box. A paragraph consisting of `[toc]` (any case) SHALL be shown as a table of contents listing only the document's top-level headings, each clickable to scroll to it.

#### Scenario: Table of contents
- **WHEN** a page contains `[toc]`, the top-level headings `A` and `B`, and headings inside a list item and a quote
- **THEN** a list containing only `A` and `B` is shown where `[toc]` is
