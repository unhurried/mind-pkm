# Proposal

## Why

Enter at the start of a heading currently puts subsequent typing in a new paragraph above the heading. Keeping the caret with the text after a split makes heading editing consistent with Typora and with splitting a paragraph.

## What Changes

- Pressing Enter at the start of a nonempty rendered heading inserts an empty paragraph above it and keeps the caret at the start of the heading.
- The empty paragraph is visible, can be entered with Up or a click, and can be undone.

## Capabilities

### Modified Capabilities

- `page-editor` defines where Enter places the caret at the start and at the end of a heading.

## Impact

The page editor's Markdown editing rules, blank paragraph layout and caret navigation will change.
