## ADDED Requirements

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
