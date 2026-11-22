## MODIFIED Requirements

### Requirement: Front matter and table of contents
YAML front matter (`---` ... `---` at the start of the file) SHALL be shown as a code-styled box. Blank lines before a YAML mapping inside the delimiters SHALL be part of the front matter. A paragraph consisting of `[toc]` (any case) SHALL be shown as a table of contents listing only the document's top-level headings, each clickable to scroll to it.

#### Scenario: Table of contents
- **WHEN** a page contains `[toc]`, the top-level headings `A` and `B`, and headings inside a list item and a quote
- **THEN** a list containing only `A` and `B` is shown where `[toc]` is

#### Scenario: Leading blank line inside YAML front matter
- **WHEN** the file starts with `---`, a blank line, `title: Plan`, and `---`
- **THEN** the delimiters, blank line, and mapping are shown together as one front matter block

### Requirement: Math
Inline math (`$...$`) and math blocks (`$$` ... `$$`) SHALL be typeset from their LaTeX. Blank lines in the body of a standalone display math block SHALL stay inside its delimiters. An expression that cannot be typeset SHALL show its source with the error, inline as code-styled text and for blocks below the source.

#### Scenario: Math block
- **WHEN** a math block contains `\frac{1}{2}`
- **THEN** a fraction with 1 over 2 is shown

#### Scenario: Blank lines inside a math block
- **WHEN** two standalone `$$` lines enclose a LaTeX expression with an empty line before or inside it
- **THEN** the expression is displayed as one math block
