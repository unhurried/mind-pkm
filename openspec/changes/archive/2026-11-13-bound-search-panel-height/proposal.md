## Why

The search panel shrinks too far in short windows and grows too far in tall windows, making results or the map harder to use.

## What Changes

- Bound the search panel to heights showing approximately four to ten result rows.
- Give the map window a minimum height that preserves room for the map and the search controls, including when restoring an older saved size.

## Capabilities

### Modified Capabilities

- `document-search`: bounded search panel height.
- `application-shell`: minimum map window height.

## Impact

Search panel sizing and map window startup settings. Editor window sizing is unaffected.
