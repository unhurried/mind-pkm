## Why

Dropping a document into the page editor currently links to the original file. Moving that file later breaks the link.

## What Changes

- Copy dropped non-image files into the page's assets folder and insert a relative Markdown link.
- Reuse files already in the folder and give copies a numbered name when the name is taken.
- Keep existing image, clipboard, and folder-drop behavior.

## Capabilities

### Modified Capabilities

- `image-insertion`: Change how the editor handles non-image file drops.

## Impact

The page editor's external file-drop handler and attachment storage.
