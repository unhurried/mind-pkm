## Context

The editor's shortcuts are one table (`MENUS`) and one function (`command_for_key`), and a test checks that they agree.

## Decisions

- Ctrl+D is taken by the assets pane, so Select Word has no shortcut; a double click selects a word, and the menu entry stays. Ctrl+L and Ctrl+E are unchanged.
- The special case that recognised Ctrl+Shift+2 by its physical key (the character there differs between US and Japanese keyboards) goes away: Ctrl+O, Ctrl+D and Ctrl+Shift+O are letters.
- The old keys are not kept as aliases; Ctrl+Shift+1 and Ctrl+Shift+2 are free.
