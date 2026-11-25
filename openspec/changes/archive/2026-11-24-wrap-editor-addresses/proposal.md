# Proposal

## Why

Long URLs in the page view can wrap at separators and leave unused space at the end of a line. The page uses different layouts for rendered text and active editing, so a URL can also wrap differently when its Markdown is revealed.

## What Changes

- Wrap visible URLs at character boundaries in paragraphs, revealed link destinations, and table cells.
- Keep those breaks in layout so the stored Markdown and link targets retain their original bytes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `markdown-rendering` covers the placement of line breaks in visible URLs in the page view.

## Impact

The page view's rendered and editable text layouts and their source-offset mapping are affected.
