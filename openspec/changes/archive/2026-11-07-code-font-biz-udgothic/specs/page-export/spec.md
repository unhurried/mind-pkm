# Spec Delta

## ADDED Requirements

### Requirement: Exported code font
The built-in export style sheet SHALL set code blocks, inline code and keys in BIZ UDGothic, in HTML and PDF alike, falling back to Consolas and then to the browser's monospace font where it is missing.

#### Scenario: Japanese in exported code
- **WHEN** a page with a code block holding `日本 ab` is exported with the built-in style sheet on a system with BIZ UDGothic
- **THEN** the code block is shown in BIZ UDGothic, kanji and ASCII alike
