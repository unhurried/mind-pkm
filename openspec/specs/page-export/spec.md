# page-export Specification

## Purpose
Exporting a page from the built-in editor to HTML and PDF, the only export formats offered, so that a page can be shared or printed outside the application.

## Requirements

### Requirement: HTML export
File → Export HTML SHALL ask for a file name (the page's name with `.html`, next to the page, by default) and write a self-contained HTML document of the page: its title, a style sheet, the rendered Markdown, math and Mermaid diagrams as inline SVG, and images referenced relative to the HTML file's location (absolute `file:` URIs where no relative path exists).

#### Scenario: Exporting a page
- **WHEN** `Trip.md` with a heading, a math block and an image in `Trip.assets/` is exported next to it
- **THEN** `Trip.html` shows the heading, the typeset formula and the image when opened in a browser, without network access

### Requirement: Export style sheet
The exported HTML SHALL use the built-in style sheet unless `%APPDATA%\mind-pkm\export.css` exists, which SHALL replace it for HTML and PDF export.

#### Scenario: Custom style sheet
- **WHEN** `export.css` sets `body { font-family: serif }` and a page is exported
- **THEN** the HTML embeds that style sheet instead of the built-in one

### Requirement: Exported heading design
The built-in export style sheet SHALL draw headings as the page view does (in its light theme), in HTML and PDF alike: the same sizes relative to the body text, in bold, and decorated by level as the scenarios say. The decorations SHALL be printed in PDF export.

#### Scenario: Level-1 heading
- **WHEN** a page containing `# Plan` is exported
- **THEN** a 3px rule in the heading color runs below `Plan` across the page's width

#### Scenario: Level-2 heading
- **WHEN** a page containing `## Plan` is exported
- **THEN** `Plan` is shown in a light band across the page's width, with an 8px chip in the heading color at the band's left edge

#### Scenario: Level-3 heading
- **WHEN** a page containing `### Plan` is exported
- **THEN** an 8px chip in the heading color stands at the left of `Plan` and a 1px gray rule runs below it

#### Scenario: Level-4 heading
- **WHEN** a page containing `#### Plan` is exported
- **THEN** a 4px gray chip stands at the left of `Plan`

#### Scenario: Levels 5 and 6
- **WHEN** a page containing `##### Plan` and `###### Plan` is exported
- **THEN** both are shown without decoration at the body text size, the level-6 one in the muted text color

#### Scenario: Band in a PDF
- **WHEN** a page containing `## Plan` is exported to PDF
- **THEN** the PDF shows the light band and the chip behind and beside `Plan`

### Requirement: PDF export
File → Export PDF SHALL ask for a file name (the page's name with `.pdf` by default) and write a PDF of the page as exported to HTML, printed by Microsoft Edge (or Google Chrome) in the background without opening a browser window. When neither browser is found, it SHALL report `PDF export needs Microsoft Edge or Google Chrome.` and write nothing.

#### Scenario: Exporting to PDF
- **WHEN** a page is exported to PDF on Windows 11
- **THEN** the PDF file is written and the status bar reports where

#### Scenario: No browser
- **WHEN** PDF export is chosen on a system without Edge or Chrome
- **THEN** `PDF export needs Microsoft Edge or Google Chrome.` is shown and no file is written

### Requirement: Exported code font
The built-in export style sheet SHALL set code blocks, inline code and keys in BIZ UDGothic, in HTML and PDF alike, falling back to Consolas and then to the browser's monospace font where it is missing.

#### Scenario: Japanese in exported code
- **WHEN** a page with a code block holding `日本 ab` is exported with the built-in style sheet on a system with BIZ UDGothic
- **THEN** the code block is shown in BIZ UDGothic, kanji and ASCII alike
