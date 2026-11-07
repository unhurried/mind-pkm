# Tasks

## 1. Remembering the outline and the assets pane

- [x] 1.1 `editor_outline=` / `editor_assets=` in the settings and two fields of the shared editor settings, saved when they change. Verify that the editor tests see the shared settings change (tests never write the settings file).
- [x] 1.2 Toggling the outline or the assets pane sets the shared setting; a new editor window opens with the outline and the pane as the settings say, open windows keep theirs. Verify with editor tests.
- [x] 1.3 README: the outline and the pane remembered. Verify that the `keys.rs` README tests still pass, and look at the app under Xvfb.
