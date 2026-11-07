# Design

## Context

The code font is one constant in the editor's theme, `Consolas` on Windows and iced's `MONOSPACE` elsewhere, used wherever the editor sets code: rendered code blocks, the code block being edited (its `text_editor` and the highlighter's formats for code-like sources), inline code and keys, front matter and math sources. iced 0.14 renders text with cosmic-text, which picks a fallback font per script for characters the requested font lacks; there is no way to give iced a list of fonts, font features or letter spacing.

## Goals / Non-Goals

**Goals:**
- Japanese and ASCII in code set in one monospace font whose full-width characters are exactly two half-width characters wide.
- No change where BIZ UDGothic is missing.

**Non-Goals:**
- Bundling a font.
- A setting for the code font.
- Changing the application font (Meiryo UI) or a user's own `export.css`.

## Decisions

- **One font for every character.** Splitting runs into Consolas (half-width) and a Japanese font (full-width) was considered; Consolas' advance (about 0.55 em) is not half of a full-width character (1 em), so columns drift, and the editor's highlighter can change only the font of a range, not its size, so it cannot be compensated while editing. BIZ UDGothic's half-width characters are 0.5 em, so one font gives the exact 1:2 ratio and the same widths rendered and edited.
- **Chosen once, from the installed fonts.** The code font becomes a function that looks BIZ UDGothic up by family name in iced's font database the first time it is asked and keeps the answer (`OnceLock`); if it is missing it returns the current font. A font named but not installed would otherwise fall back per script (the problem this change fixes) for every character, ASCII included. The lookup uses the same database iced draws with, so the answer matches what is drawn.
- **Family name `BIZ UDGothic`.** The fixed-width family of the font (the file also holds the proportional `BIZ UDPGothic`); the English name is the one every system reports.
- **Export:** `code, kbd, pre { font-family: "BIZ UDGothic", Consolas, "Courier New", monospace }` - browsers fall back through the list themselves.

## Risks / Trade-offs

- [BIZ UDGothic's Latin letters look different from Consolas'] → accepted; it was asked for, for the aligned columns.
- [The lookup runs on the UI thread the first time code is shown] → it scans the font database's face list once, which cosmic-text has loaded already for the first text drawn.
- [Tests on machines with and without the font measure code differently] → no test depends on a particular code font's widths; they measure with whatever the code font is.
