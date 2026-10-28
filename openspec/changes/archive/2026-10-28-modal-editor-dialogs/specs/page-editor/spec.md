## ADDED Requirements

### Requirement: Dialogs of an editor are modal
While a dialog (a message, the question about unsaved changes, the question about a file changed on disk) is shown in an editor window, no key and no text from an input method SHALL change the page behind it. Enter SHALL choose the dialog's default button (OK, Save, Reload respectively) and Escape SHALL cancel it.

#### Scenario: Typing while the unsaved changes question is shown
- **WHEN** an edited page's window is closed with Ctrl+W and `LEAK` is typed while the question is shown
- **THEN** the page does not change and the question stays

#### Scenario: Enter saves
- **WHEN** an edited page's window is closed with Ctrl+W and Enter is pressed
- **THEN** the page is saved and the window closes
