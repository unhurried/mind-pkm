# Design

## Context

See proposal.md - Why. Today the application is an `iced::daemon` whose boot always opens the map window and passes the first command-line argument to the map loader. Editor windows are opened only by the app on behalf of the map (F1, New Page, search results, link markers) or of another editor (a followed link, the assets pane), and every open goes through one place that reports its failure as a dialog of the map window. Ending the application (closing the map window, File → Exit, the unsaved pages question) remembers the map's view and the map window's geometry in the per-user settings.

## Goals / Non-Goals

**Goals:**
- A run with editor windows only: no map window is created at all, and the run ends when its last editor window closes.
- Every way a user explicitly asks the application to open a `.md` file (command line, File → Open in either window) ends in the built-in editor.
- Errors of opens started from an editor reach the user in that editor.

**Non-Goals:**
- Opening the map window later in a run that started without it.
- A single-instance application (forwarding files to a running process).
- Any change to how the map, the PKM commands or the document directory work.

## Decisions

### No map window, rather than a hidden one
A run started with a `.md` file never opens the map window; the app records that it runs with editor windows only. Closing an editor window then checks whether it was the last one and, if so, ends the application.
- Alternative: open the map window hidden or minimized. Rejected: iced would still build and lay out its view after every message, it would show up in Alt+Tab or the taskbar on some setups, and ending the run would still need special handling.

### One process per file opened from Explorer
Each `.md` file opened from Explorer starts its own process, as `.mm` files do today.
- Alternative: hand the file to a running instance over a named pipe. Rejected for now: platform-specific IPC, races at start, and not asked for. Two processes editing the same page are already handled by the editor's check for files changed on disk.

### Explicit opens ignore Use Built-in Editor
The command-line argument and both File → Open dialogs open the file in the built-in editor, whatever Use Built-in Editor says. The setting keeps governing the opens it names (F1, New Page, link markers, search results) and followed links.
- Reason: the user chose this application for the file. With `.md` associated with Mind PKM and the setting off, handing the file to "the default application" would start Mind PKM again, endlessly.

### A start that cannot open its file falls back to a normal start
If the `.md` argument cannot be opened (missing, not UTF-8), the app starts as without arguments (the map window, the last map) and queues the error as a map window dialog.
- Alternative: a native message box, then exit. Rejected: it would be the only native dialog, and it could not be tested by driving the app with its messages.
- Alternative: open a missing file as a new, empty page. Rejected: it would create files implicitly, and File → New is out of scope.

### Ending without a map window leaves the map's settings alone
Ending a run without a map window does not write the map window's geometry or a map view, and nothing in such a run sets the last opened map. The editor's own settings (window size, zoom, theme) are saved when they change, as now. The app also records that it is ending, so that the tests can check that it ends.

### Opens report to the window they came from
The single open path returns its error instead of showing it. The map window's callers show it as a map dialog (as now). The editor's callers show it as a message in the requesting editor: File → Open..., a followed `.md` link, and the default application failing for a link while Use Built-in Editor is off. A new editor effect asks the app to open a chosen file in the built-in editor. The existing effect for followed links keeps honouring the setting.

### Editor File → Open...
A new editor command, Open... (Ctrl+O, free in the editor's bindings), runs an asynchronous file dialog filtered to Markdown (`*.md`) and starting in the page's folder, the same way Export picks its file. Its result comes back as an editor message, which turns a chosen path into the new effect; a cancelled dialog does nothing.

### Map File → Open
The Open dialog offers "Mind map or Markdown" (`*.mm`, `*.md`) first, then "Mind map" and "Markdown". Save As keeps "Mind map" only. A chosen `.md` file opens in an editor window; anything else is loaded as a map, as now. The question about the map's unsaved changes stays before the dialog, so the existing flow is unchanged. Answering Don't Save and then choosing a `.md` file leaves the map open with its changes, so nothing is lost.
- Alternative: ask only after a map file is chosen. Rejected: it reorders the existing flow (and its tests) for a case where asking first does no harm.

### What counts as a Markdown file
A path counts as a Markdown file when it has a `.md` extension in any case and is not a directory. This is the rule the app already uses to decide what the built-in editor opens, so the argument, the dialogs and links agree.

## Risks / Trade-offs

- [The same page open in a map run and in an editors-only run] → each window reloads changes from disk when it is activated and asks when it has unsaved changes of its own (existing behavior).
- [Two runs writing the editor settings] → the last writer wins, as with two map runs today. These are window size, zoom and theme only.
- [Assets pane without a map] → rename and delete are not offered, because there is no document directory: the existing rule for files outside it applies, and the guardrail on removing files stays intact. Inserting images, which copies into `<name>.assets/`, works as before.
- [Errors queued for a map window that does not exist] → every open an editor starts reports in that editor. No map-only action can run in an editors-only run.
