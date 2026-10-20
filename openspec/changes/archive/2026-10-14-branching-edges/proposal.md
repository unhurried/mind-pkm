# Proposal

## Why

The edges to a node's children all start at the end of its underline and part right away, so with many children they lie on top of each other near the parent and look like one thick line.

## What Changes

- The edges to a node's children run as a single line from the parent and branch toward each child only near the children: horizontally from the parent's underline, vertically along the children, and with a rounded corner into each child's underline.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the edges requirement describes the shared, branching edges.

## Impact

- `src/app/view.rs`: edge drawing.
