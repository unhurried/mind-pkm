# Tasks

## 1. Settings

- [x] 1.1 `views.txt`: read and write per-map views (zoom, scroll position, selected IDs), most recent first, at most 50, skipping unreadable lines
- [x] 1.2 Keep tests away from the real settings directory
- [x] 1.3 Tests: round trip, unreadable lines, replacing a map's view, the limit

## 2. Remembering and restoring

- [x] 2.1 Remember the open map's view on exit, before another map replaces it, and after saving
- [x] 2.2 Restore a remembered view when a map is opened: zoom, selection (gone nodes left out), scroll position
- [x] 2.3 Tests: a map reopens as it was shown; gone nodes leave the root selected; folded ones hand the selection to the shown ancestor; untitled maps remember nothing

## 3. Verification

- [x] 3.1 fmt, clippy, tests, `openspec validate --all --strict`
- [x] 3.2 Run the app under Xvfb: zoom, select, scroll, quit, start again
