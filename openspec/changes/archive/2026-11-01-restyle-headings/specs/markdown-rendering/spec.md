## ADDED Requirements

### Requirement: Heading design
Headings SHALL be set in bold at 2, 1.5, 1.25, 1.125, 1 and 0.85 times the body text size (levels 1 to 6) in the colors of the theme, level 6 in the muted text color, and decorated by level as the scenarios say. A heading being edited SHALL look as it does rendered, so that its text stays where it is when the cursor enters or leaves it.

#### Scenario: Level-1 heading
- **WHEN** a page contains `# Plan`
- **THEN** a 3px rule in the text color runs below `Plan` across the page's width

#### Scenario: Level-2 heading
- **WHEN** a page contains `## Plan`
- **THEN** `Plan` is shown in a light band across the page's width, with an 8px chip in the text color at the band's left edge and the text after it

#### Scenario: Level-3 heading
- **WHEN** a page contains `### Plan`
- **THEN** an 8px chip in the text color stands at the left of `Plan` and a 1px gray rule runs below it

#### Scenario: Level-4 heading
- **WHEN** a page contains `#### Plan`
- **THEN** a 4px gray chip stands at the left of `Plan`

#### Scenario: Levels 5 and 6
- **WHEN** a page contains `##### Plan` and `###### Plan`
- **THEN** both are shown without decoration, the level-6 one in the muted text color

#### Scenario: Heading being edited
- **WHEN** the cursor enters the heading `### Plan`
- **THEN** its chip, its rule and its text stay where they were
