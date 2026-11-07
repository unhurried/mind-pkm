## Context

Ctrl+K runs the editing rule that wraps the selection in `[...]()` with the cursor between the parentheses, in the active block or in the source mode. Pasting reads the system clipboard in the background (`blocking`) and hands its content to the editor window in a message; the editor never reads the clipboard in `update`.

## Decisions

- Ctrl+K asks the app to read the clipboard's text (only the text: no image or HTML is fetched) in the background, and the link is inserted when the text arrives, with the selection as it is then - the same way as pasting. If nothing can be read, the link is inserted as before.
- What counts as a link destination (the text trimmed, on one line, outer double quotes removed):
  - a URL: a scheme (a letter, then letters, digits, `+`, `-`, `.`) followed by `://`, or `mailto:` / `file:`; without spaces;
  - an absolute path: `C:\` or `C:/` (any drive letter), `\\server\...` or `//server/...`, `/...`;
  - a path relative to the page that says so: `./`, `../`, `.\`, `..\`.
  Plain words (`notes.md`, `foo/bar`) are not taken: they are as likely to be text the user copied. A text with `<`, `>` or unbalanced parentheses is not taken either, as it would end the link early.
- Backslashes of a path are written as `/`: CommonMark takes a backslash before punctuation as an escape (`C:\_work` would name `C:_work`), and the editor resolves `/` to the platform's separator when a link is opened. Paths with spaces are written as they are: the editor already takes destinations with spaces as links.
- With a destination: a selection becomes `[selection](destination)` with the cursor after the link (it is complete); without a selection, `[](destination)` with the cursor in the link text, to type it.

## Risks / Trade-offs

- The selection may change between Ctrl+K and the arrival of the clipboard's text (a few milliseconds); the link goes where the selection is then, as with pasting.
