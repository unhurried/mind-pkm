# Design

The HTML is split into its tables and the HTML between them. If what is left besides one table has no text (a spreadsheet's `<style>`, `<meta>`, empty wrappers), the table's cells are pasted as before: into the cells of a table, or as a new table. Otherwise every piece is converted on its own (HTML to Markdown, a table to a Markdown table) and the pieces are joined by blank lines.

A grid of tab-separated text needs equal row lengths because spreadsheets copy every row with all its cells (empty ones as empty fields); code and prose with an incidental tab don't. Lines that all start with a tab are indented text, not a grid with an empty first column.
