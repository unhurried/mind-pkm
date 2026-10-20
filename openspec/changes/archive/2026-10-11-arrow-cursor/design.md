# Design

## Decisions

- **One wrapper at the root** instead of removing interactions widget by widget: built-in widgets (buttons, text inputs, pick lists) set their own pointers. A pass-through widget around the whole view delegates layout, drawing, events and operations to its content but answers every pointer query with the arrow, and wraps the content's pop-up overlays the same way.
- **Idle vs None**: both map to the default arrow, but the shell only applies the pointer when the reported value changes and starts from `None`. The wrapper reports `Idle` while the pointer is over the window (else `None`), so the arrow is applied when the pointer enters.
- The explicit `Pointer` / `Grab` / `Grabbing` interactions in the map view are removed as dead.
