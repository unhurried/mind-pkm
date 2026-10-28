# Proposal

## Why

While a dialog of the page editor was shown (unsaved changes, file changed on disk, an error), the keyboard still edited the page behind it: Enter inserted a paragraph instead of choosing the default button, and typed text went into the page, to be saved with it (issue #21). Enter also did not choose the default button of the dialogs that have a Save or Reload button.

## What Changes

- While a dialog is open, no key or input-method text reaches the page; the dialog gets the keys instead.
- Enter chooses the default (highlighted) button: OK for a message, Save for the unsaved-changes question, Reload for a file changed on disk.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `page-editor`: a new requirement on the keyboard while an editor dialog is open.

## Impact

- `src/app/editor/modal.rs` (new: the layer that keeps the keyboard from the page), `src/app/editor/view.rs` (the dialog is wrapped in it), `src/app/editor/mod.rs` (Enter's answer per dialog).
- Tests in `src/app/editor/tests.rs` and `src/app/editor/modal.rs`.
