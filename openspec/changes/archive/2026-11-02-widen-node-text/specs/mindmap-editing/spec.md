## MODIFIED Requirements

### Requirement: Fonts
Every node, the root included, SHALL use the same font and font size. The application font SHALL be Meiryo UI, falling back to the system fonts where it is missing. Node text wider than 400 px at 100% zoom SHALL wrap; for the nodes below the root's ToDo node (not ToDo itself) the width SHALL be 800 px.

#### Scenario: Root node
- **WHEN** a map is shown
- **THEN** the root's text has the same size as every other node's text

#### Scenario: Long text
- **WHEN** a node outside ToDo has a text that is 600 px wide on one line at 100% zoom
- **THEN** it wraps into lines no wider than 400 px

#### Scenario: Long text below ToDo
- **WHEN** a node below the root's ToDo node has a text that is 600 px wide on one line at 100% zoom
- **THEN** it is shown on one line
