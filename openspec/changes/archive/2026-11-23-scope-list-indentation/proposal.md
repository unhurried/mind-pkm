# Proposal

## Why

Tab currently moves the subtree of the item at the caret. A selection across list items does not have a corresponding indentation command.

## What Changes

- Scope Tab to one item at the caret or to the expressly selected items.
- Move selected siblings together by one level in the rendered editor.

## Capabilities

### New Capabilities

### Modified Capabilities

- `page-editor` covers the caret and selection rules for list indentation.

## Impact

The rendered Markdown editor, its keyboard bindings, and the page editor specification are affected.
