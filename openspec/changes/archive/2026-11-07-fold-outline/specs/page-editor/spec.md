## ADDED Requirements

### Requirement: Folding the outline
An outline entry followed by headings of a lower level (up to the next heading of the same or a higher level) SHALL have an arrow that folds and unfolds the entries under it, without scrolling the document. By default the outline SHALL be unfolded down to H3: entries of H1 and H2 unfolded, those of H3 and below folded. What was folded or unfolded SHALL be kept while the page is edited, as long as the heading's text stays the same.

#### Scenario: Unfolded down to H3 by default
- **WHEN** the outline is shown for a page with `# A`, `## B`, `### C` and `#### D`
- **THEN** the outline lists `A`, `B` and `C`, with `C` folded and `D` hidden under it

#### Scenario: Unfolding an entry
- **WHEN** the arrow of the folded entry `C` is clicked
- **THEN** `D` is listed under `C` and the document does not scroll

#### Scenario: Folding an entry
- **WHEN** the arrow of the unfolded entry `A` is clicked
- **THEN** only `A` is listed, the headings under it hidden

#### Scenario: Editing the page
- **WHEN** `C` was unfolded and text is typed into a paragraph of the page
- **THEN** `C` stays unfolded
