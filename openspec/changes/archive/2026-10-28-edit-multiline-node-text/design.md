# Design

The input dialogs whose value is a node's text use a text editor widget instead of a single-line input; its content lives in the application while the dialog is shown (set from the dialog's value when the dialog comes to the front) and every edit writes the text back to the dialog's value, so OK and Cancel work as before. Its key binding turns Enter without Shift into OK; everything else, Shift+Enter included, is the editor's own. The other input dialogs (renaming a page or directory, the filter, links) keep the single-line input, as their values can't hold line breaks.
