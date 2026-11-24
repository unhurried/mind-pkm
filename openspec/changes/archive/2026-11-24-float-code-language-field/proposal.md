## Why

Entering or leaving a code block shifts the rest of the page because the language field takes up a row in the block.

## What Changes

- Float the language field below the right edge of the active code block so nearby blocks stay in place.

## Capabilities

### New Capabilities

### Modified Capabilities

- `page-editor` specifies the placement and layout behavior of the code language field.

## Impact

The page editor and its regression tests are affected.
