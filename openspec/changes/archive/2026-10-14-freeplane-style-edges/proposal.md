# Proposal

## Why

The edges are thick and all leave the parent horizontally from its middle, so the edges to several children overlap near the parent. Freeplane draws thin edges that turn steeply and run on under each node, which keeps them apart and reads better.

## What Changes

- Edges are thinner (1 px at 100% zoom).
- Every node is underlined; an edge runs from the right end of the parent's underline to the left end of the child's.
- Edges turn steeply near the parent, so the edges to several children part right away.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `mindmap-editing`: the right-only layout requirement no longer describes the edges; a new edges requirement does.

## Impact

- `src/app/view.rs`: edge drawing.
