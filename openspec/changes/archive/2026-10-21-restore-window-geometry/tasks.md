# Tasks

## 1. Settings

- [x] 1.1 Read and write `settings.txt` as `key=value` lines, replacing only the changed key
- [x] 1.2 `window=`: size, position and maximized state; skip unreadable or too small values
- [x] 1.3 Tests: replacing a key keeps the others; the window reads back as written; bad values are skipped

## 2. Window

- [x] 2.1 Open the window with the remembered size, position and maximized state
- [x] 2.2 Track moves and resizes; take them as the window to restore only when neither maximized nor minimized
- [x] 2.3 Remember the window on exit
- [x] 2.4 Test: maximizing keeps the normal size and position, minimizing changes nothing

## 3. Verification

- [x] 3.1 fmt, clippy, tests, `openspec validate --all --strict`
- [x] 3.2 Run the app under Xvfb with a window manager: resize, move, quit, start again
