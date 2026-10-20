# Tasks

## 1. Panning

- [x] 1.1 Start a pan when the left button is pressed on empty map space (not on nodes, markers or scrollbars)
- [x] 1.2 While panning, follow pointer moves by scrolling the map by the pointer's movement, clamped to the scrollable area
- [x] 1.3 End the pan on button release or when the pointer leaves the window
- [x] 1.4 Show an open hand over empty map space and a closed hand while panning

## 2. Verification

- [x] 2.1 Run the app under Xvfb and drag the map with xdotool; check that node clicks still select and do not pan
- [x] 2.2 README: mention drag to pan; `openspec validate --all --strict`, fmt, clippy, tests
