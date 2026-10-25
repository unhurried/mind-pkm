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

### Requirement: PDF export
File → Export PDF SHALL ask for a file name (the page's name with `.pdf` by default) and write a PDF of the page as exported to HTML, printed by Microsoft Edge (or Google Chrome) in the background without opening a browser window. When neither browser is found, it SHALL report `PDF export needs Microsoft Edge or Google Chrome.` and write nothing.

#### Scenario: Exporting to PDF
- **WHEN** a page is exported to PDF on Windows 11
- **THEN** the PDF file is written and the status bar reports where

#### Scenario: No browser
- **WHEN** PDF export is chosen on a system without Edge or Chrome
- **THEN** `PDF export needs Microsoft Edge or Google Chrome.` is shown and no file is written
