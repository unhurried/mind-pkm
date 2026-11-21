# Design

## Context

The text in another cell of the active table is rendered from a cell-local Markdown document. Its source offsets cannot identify positions in the page.

## Goals

- Keep the active table and place the caret in the clicked cell.
- Keep cell range gestures and link navigation available.

## Decisions

Let the cell frame handle ordinary presses across the cell. Place the caret at the end of the cell, as for a press in the padding of a rendered table. Route modified link clicks through link navigation without a source offset. The active text widget handles text drags that start in it. A drag from a newly activated cell uses the table's range handler.

## Verification

Drive the rendered widgets with a click, slight movement during a second press in another cell, and a release. Check the caret, selection, and an intentional cell drag.
