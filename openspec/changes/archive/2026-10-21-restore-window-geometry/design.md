# Design

## Context

The window is created from `window::Settings` (size, position, maximized) before the application state exists. Resize events already arrive; move events are not listened to. Exiting goes through one helper that also remembers the map's view.

## Goals / Non-Goals

**Goals:**
- The next start shows the window where and as large as it was, maximized if it was.

**Non-Goals:**
- Moving a window back onto the screen when the monitor it was on is gone (the application cannot see the monitor layout).
- Remembering the window per map.

## Decisions

- **One window, in `settings.txt`**: `window=width,height,x,y,maximized` in logical pixels. `settings.txt` was rewritten whole for the last opened map; it is now read as `key=value` lines and only the changed key is replaced, so the existing `last_file=` line keeps working. Sizes below 200 pixels are not restored.
- **The size to restore is the normal one**: after every resize or move the application asks whether the window is maximized or minimized. Minimized: nothing is taken (on Windows a minimized window reports position −32000 and size 0). Maximized: only that is noted, keeping the size and position from before. Otherwise the current size and position are taken. The window then opens with the normal size and position and the maximized flag, so un-maximizing returns to them.
- **Remembered on exit** (through the same helper as the map's view), whether or not a map is open.
- **Tests** drive the resize, move and state messages without a window; the settings format is tested as pure functions.

## Risks / Trade-offs

- A window remembered on a monitor that is no longer connected opens off-screen; removing the `window=` line from `settings.txt` returns to the default.
- On X11 without a window manager no move events arrive, so the position is not remembered there; Windows reports every move.
