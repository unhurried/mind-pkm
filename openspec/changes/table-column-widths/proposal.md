# Proposal

## Why

The page editor gives every column of a table the same width (the page's width divided by the number of columns), so a short column such as a number or a status takes as much room as a column of sentences, which then wraps after a few words. Tables with many columns cannot be wider than the page at all, and their cells get squeezed.

## What Changes

- Each column of a rendered table takes the width of its widest cell when that cell fits on one line within a fixed maximum column width; a column whose cell would wrap takes the maximum width and its text wraps inside it.
- A table narrower than the page is not shrunk: its columns are widened in proportion to those widths until the table fills the page.
- A table wider than the page keeps its columns and can be scrolled sideways, with a horizontal scroll bar under it; the page itself stays as wide as before.
- The table being edited has the same column widths as when it is rendered, worked out again after every edit, so a column grows or shrinks as its text is typed.
- While a table that scrolls sideways is edited, the cell with the cursor is scrolled into view, and the table keeps the scroll position it had when it was clicked.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `markdown-rendering`: the Tables requirement gains column widths taken from the cells' contents and sideways scrolling for wide tables.
- `table-editing`: a new requirement for the column widths and scrolling of the table being edited.

## Impact

- Page editor rendering of tables (rendered and edited), the measuring of cell text, and the scrolling that keeps the cursor in view.
- No change to the Markdown written to the file, the HTML/PDF export, or the map window.
