## MODIFIED Requirements

### Requirement: HTML export
File → Export HTML SHALL ask for a file name (the page's name with `.html`, next to the page, by default) and write a self-contained HTML document of the page: its title, a style sheet, the rendered Markdown, math and Mermaid diagrams embedded as SVG images, and images referenced relative to the HTML file's location (absolute `file:` URIs where no relative path exists).

#### Scenario: Exporting a page
- **WHEN** `Trip.md` with a heading, a math block and an image in `Trip.assets/` is exported next to it
- **THEN** `Trip.html` shows the heading, the typeset formula and the image when opened in a browser, without network access

## ADDED Requirements

### Requirement: Passive exported content
HTML and PDF exports SHALL retain passive formatting while removing scripts, event handlers, embedded browsing contexts, forms, document redirects and unsupported URL schemes from page content. Generated math and diagram SVGs SHALL be embedded as passive images. Exported documents SHALL include a Content Security Policy that blocks scripts, objects, frames, connections, forms and base URL changes.

#### Scenario: Untrusted page content
- **WHEN** a page containing script elements, event handlers and JavaScript links is exported
- **THEN** the exported page does not execute that content and safe text, formatting and images remain visible

#### Scenario: PDF resource loading
- **WHEN** a page containing remote images is exported to PDF
- **THEN** the printing browser disables JavaScript and does not load remote page resources, while local and embedded images remain available
