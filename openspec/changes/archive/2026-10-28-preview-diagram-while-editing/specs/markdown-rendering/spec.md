## MODIFIED Requirements

### Requirement: Mermaid diagrams
A ` ```mermaid ` code block SHALL be shown as the diagram it describes. A diagram that cannot be rendered SHALL show the error below the block's source instead, keeping the source. While the block is being edited, its diagram (or the error) SHALL be shown below its source, following the source as it changes.

#### Scenario: Flowchart
- **WHEN** a block reads ` ```mermaid `, `flowchart LR`, `A --> B`, ` ``` `
- **THEN** a diagram with boxes A and B joined by an arrow is shown

#### Scenario: Syntax error
- **WHEN** a mermaid block contains `flowchart LR` and `A -->`
- **THEN** its source is shown with an error message below it

#### Scenario: Editing a diagram
- **WHEN** the cursor is in a block reading ` ```mermaid `, `graph TD`, `A-->B`, ` ``` `
- **THEN** the diagram with A and B is shown below the source being edited
