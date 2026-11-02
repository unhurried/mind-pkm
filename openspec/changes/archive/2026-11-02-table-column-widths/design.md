# Design

## Context

A rendered table (`view/blocks.rs`, `table_view`) gives every cell `FillPortion(1)`, and the table being edited (`view/active.rs`, `table_editor`) gives every cell `width / columns` (at least 40 px). Both fill the page's column exactly, so they agree with each other, but nothing about a column's contents affects its width. Rendered cells show only text runs (and footnote numbers) through `inline_text`; the cell with the cursor is a `LiveText` widget laid out in a width given by the view. iced rebuilds every view after every message, so whatever the view measures must be cached.

## Goals / Non-Goals

**Goals:**
- One pure function decides the column widths from each column's natural (one-line) width, the maximum column width and the page's width; the rendered table and the edited one both use it, so entering a table moves nothing.
- Measuring a cell is cached, so a view rebuilt for an unrelated message shapes no text again.

**Non-Goals:**
- Column widths chosen by the user, or stored in the file.
- The HTML/PDF export (the browser lays out its tables itself) and the map window.

## Decisions

- **Widths.** For each column, `natural` is the widest line (cells split at `<br>`) of its cells' text, header cells in bold. The column's text width is `natural` if that is at most the maximum, else the maximum; the column is that plus the cell padding. If the columns add up to less than the page's width, each is multiplied by `page / sum`; otherwise they stay and the table scrolls. A tiny allowance (1 px) is added to a fitting column so that rounding never wraps a line that was measured to fit. Alternative considered: the browser's auto table layout (distributing the spare width to wrapping columns first); rejected because the user asked for a fixed maximum, and proportional widening keeps the columns' relative sizes predictable.
- **Maximum column width.** 320 px of text at 100% zoom, scaled by the zoom like every other size of the editor (about 21 full-width characters at the body size). A constant beside the other sizes in `view/theme.rs`.
- **Measuring.** A line is measured as one paragraph of its spans (text, size, font) with unbounded width and no wrapping, the way the rich text widget shapes it. The cell with the cursor is measured from the runs its `LiveText` shows (what it shows includes the Markdown of the inline elements at the cursor, so its column fits what is typed). Results are cached in the editor (`RefCell<HashMap<line key, width>>`, cleared when it grows past a few thousand entries); a key is the line's spans, so zooming or editing a cell measures only the lines that changed.
- **Sideways scrolling.** A table wider than the page is put in a horizontal `scrollable` whose scroll bar takes its own room under the table (not drawn over the last row). The wheel still scrolls the page: a horizontal-only scrollable leaves vertical wheel movement to the page because it changes nothing in it (checked in iced 0.14's scrollable: the event is only captured when the offset moved); Shift+wheel scrolls the table.
- **Keeping the scroll position.** A rendered table and the edited one are different widgets, so iced does not carry the scroll state from one to the other. Scrolling tables report their offset (`on_scroll`) with the source offset where the table starts; the editor keeps the last one. When a table becomes the active region and the kept offset is for it, the operation that keeps the cursor in view starts from that offset. Alternative considered: no tracking (the edited table starts at the left); rejected because clicking a cell of a table scrolled to the right would make the table jump under the pointer.
- **Cell in view.** After the cursor moves in a table that scrolls, one widget operation finds the cursor's cell (`active-cell`) and the table's scrollable (`table-scroll`) and scrolls it sideways only when the cell is not fully in view, the same way the page is scrolled vertically for the caret (`scroll.rs`).

## Risks / Trade-offs

- [Columns move while typing] → Chosen by the user: the widths follow what is typed; only the table with the cursor is affected.
- [Shaping cost on large tables] → Cached per line; only edited cells are measured again. Checked with the benchmark.
- [`on_scroll` messages] → A scrollable reports its viewport when it is first drawn and when its bounds change, which happens only for tables wider than the page; each report is a cheap update.
- [Rendered and live cell measured slightly differently (one paragraph vs. separately shaped runs)] → The 1 px allowance; a test checks that a cell measured to fit is laid out on one line.
