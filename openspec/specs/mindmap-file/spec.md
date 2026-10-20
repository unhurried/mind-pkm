# mindmap-file Specification

## Purpose
Reading and writing Freeplane `.mm` files so that maps stay interchangeable with Freeplane and with the former Freeplane PKM add-on: anything this application does not use is preserved, and what it writes is what Freeplane would write.

## Requirements

### Requirement: Lossless round trip
The application SHALL preserve every part of a map it does not edit: XML attributes and their order, non-node elements (styles, hooks, icons, attributes, rich content, comments) and their order relative to child nodes, and node IDs. A map that is opened and saved without edits SHALL be written back byte for byte as Freeplane wrote it.

#### Scenario: Saving an unedited map
- **WHEN** a map written by Freeplane is opened and saved without changes
- **THEN** the saved file is identical to the original file

#### Scenario: Editing one node
- **WHEN** the text of one node is changed and the map is saved
- **THEN** only that node's text and MODIFIED timestamp differ from the original; styles, attributes, hooks and all other nodes are unchanged

### Requirement: Freeplane-compatible encoding
The application SHALL write maps the way Freeplane's writer does: no XML declaration, every non-ASCII character as a hexadecimal character reference (characters outside the Basic Multilingual Plane as a UTF-16 surrogate pair of references), and line breaks and tabs inside attribute values as character references. It SHALL read such references back, including surrogate pairs, as the original characters. A UTF-8 byte-order mark at the start of a file SHALL be accepted.

#### Scenario: Japanese and emoji text
- **WHEN** a node's text is `日本😀` and the map is saved
- **THEN** the TEXT attribute is written as `&#x65e5;&#x672c;&#xd83d;&#xde00;`
- **AND** reopening the map shows the text `日本😀`

#### Scenario: Multi-line node text
- **WHEN** a node's text contains a line break
- **THEN** it is written as `&#xa;` inside the TEXT attribute

### Requirement: Node text
A node's text SHALL be its TEXT attribute, or, when it has none, the plain text of its HTML rich content (block elements become lines, whitespace collapses). Editing a node's text SHALL store it in the TEXT attribute, remove the node's HTML rich content, and update its MODIFIED timestamp.

#### Scenario: Rich-text node
- **WHEN** a node has no TEXT attribute and rich content `<p>Rich &amp; text</p><p>second</p>`
- **THEN** its text is shown as two lines, `Rich & text` and `second`

### Requirement: New nodes
A node created by the application SHALL get a unique `ID_<number>` ID, CREATED and MODIFIED timestamps in milliseconds, and its TEXT attribute first, as Freeplane writes them. A new map SHALL be a valid Freeplane map.

#### Scenario: Creating a node
- **WHEN** a child node is added and the map is saved
- **THEN** the new `node` element has TEXT, ID, CREATED and MODIFIED attributes and an ID no other node of the map has

### Requirement: Links
A node's LINK SHALL be read as a node link (`#ID_...`), a file (a path relative to the map file's directory, percent-encoded, or a `file:` URI), or another URI. Files linked by the application SHALL be written relative to the map file's directory, percent-encoded like `java.net.URI`, and as an absolute `file:` URI only when no relative path exists (another drive). Paths SHALL be compared the way the platform does: normalized, and case-insensitively on Windows.

#### Scenario: Linking a page
- **WHEN** a node in `C:\maps\pkm.mm` is linked to `C:\docs\My Page.md`
- **THEN** its LINK is `../docs/My%20Page.md`

#### Scenario: Linking across drives
- **WHEN** a node in `C:\maps\pkm.mm` is linked to `D:\docs\x.md`
- **THEN** its LINK is `file:/D:/docs/x.md`

### Requirement: Node position attribute
The application SHALL give every node it adds, pastes or moves under the root a POSITION of `right` (or `bottom_or_right` when the map already uses Freeplane 1.12 position values), and SHALL NOT rewrite the POSITION of nodes it does not move.

#### Scenario: Map using Freeplane 1.12 values
- **WHEN** a root child is added to a map whose root children use `top_or_left` / `bottom_or_right`
- **THEN** the new node gets `POSITION="bottom_or_right"`

### Requirement: Safe saving
Saving SHALL write the complete map to a temporary file first and replace the map file only once that succeeded, so a failed save never leaves a truncated map.

#### Scenario: Disk full while saving
- **WHEN** writing the map fails
- **THEN** the existing map file is unchanged and an error is shown
