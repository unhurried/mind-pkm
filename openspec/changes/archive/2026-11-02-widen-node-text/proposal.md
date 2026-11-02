# Proposal

## Why

Node texts wrapped at 360 px, which breaks short sentences into several lines. Items under ToDo are copied from pages' Next Steps and are usually long sentences, so they need more room than other nodes.

## What Changes

- Node text wraps at 400 px at 100% zoom (was 360 px).
- Nodes below the root's ToDo node (its items, Due Today / Past Due and their children) wrap at 800 px instead. The ToDo node itself keeps 400 px.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `mindmap-editing`: the wrap width of node texts (requirement "Fonts").

## Impact

- `src/app/view.rs` (the maximum width depends on the node), `src/app/viewport.rs` and `src/app/doc.rs` (the measured-size cache is keyed by the width too).
