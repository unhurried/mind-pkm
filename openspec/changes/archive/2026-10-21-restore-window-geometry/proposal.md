# Proposal

## Why

The window always opens at 1280×800 where the system puts it, so the user has to resize and move it again after every start.

## What Changes

- On exit the application remembers the window's size and position and whether it is maximized; the next start opens the window that way.
- A maximized window opens maximized, and un-maximizing it returns to the size and position it had before it was maximized. Sizes and positions while minimized are never remembered.
- Without a remembered window, it opens at 1280×800 as before.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `application-shell`: A new requirement for restoring the window; the per-user settings keep the window's size, position and maximized state.

## Impact

- `src/app/settings.rs`: `settings.txt` becomes a set of `key=value` lines, gaining `window=`.
- `src/app/mod.rs`, `src/app/viewport.rs`, `src/app/file.rs`: tracking the window's size, position and state; opening the window from the remembered one; remembering it on exit.
