# Tasks

## 1. Zoom

- [x] 1.1 Zoom level in the application state, with the fixed steps and Zoom In / Zoom Out / Reset Zoom commands
- [x] 1.2 Measure and lay out nodes at the zoomed size (text, padding, icons, gaps, markers, edges); drop the measured sizes when the zoom changes
- [x] 1.3 Ctrl+Plus / Ctrl+Minus / Ctrl+0 key bindings and a View menu
- [x] 1.4 Catch Ctrl+wheel over the map and zoom around the pointer
- [x] 1.5 Show the zoom level in the status bar

## 2. Verification

- [x] 2.1 Unit tests: zoom steps, layout gaps scale with the zoom, key bindings
- [x] 2.2 Run the app under Xvfb, zoom with the keys and Ctrl+wheel, check node clicks, drag and drop and panning at a zoom other than 100%
- [x] 2.3 README: the shortcuts; `openspec validate --all --strict`, fmt, clippy, tests
