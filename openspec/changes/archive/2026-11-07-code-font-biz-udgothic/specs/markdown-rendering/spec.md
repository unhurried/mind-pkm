# Spec Delta

## ADDED Requirements

### Requirement: Code font
Text shown in the code font - code blocks, inline code, keys (`<kbd>`) and the source of code-like blocks being edited - SHALL be set in BIZ UDGothic, half-width and full-width characters alike, so that a full-width character takes exactly the width of two half-width characters. Where BIZ UDGothic is not installed, the code font SHALL be Consolas on Windows and the system's monospace font elsewhere.

#### Scenario: Japanese in a code block
- **WHEN** a code block holds the lines `ab` and `日本` and BIZ UDGothic is installed
- **THEN** both lines are shown in BIZ UDGothic, kanji included, and `日本` is exactly twice as wide as `ab`

#### Scenario: Inline code
- **WHEN** a paragraph contains `` `値 = 1` `` and BIZ UDGothic is installed
- **THEN** the inline code, `値` included, is shown in BIZ UDGothic, while the text around it keeps the application font

#### Scenario: Editing a code block
- **WHEN** the cursor is in a code block holding Japanese text
- **THEN** the code being edited is shown in BIZ UDGothic, as wide as when it is rendered

#### Scenario: Font not installed
- **WHEN** BIZ UDGothic is not installed on Windows
- **THEN** code is shown in Consolas
