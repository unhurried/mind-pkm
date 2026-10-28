# Design

The page's text widget keeps its keyboard focus while a dialog is shown, and iced offers a key to the widgets in the window first; the editor's own key handler only gets keys that no widget took. So the page consumed every key before the handler that answers dialogs saw it.

- The dialog layer is wrapped in a widget that, after its own buttons, captures every keyboard and input-method event, so the layers below never see them. It publishes the key events as the editor's key message, so the dialog is still answered with the keyboard.
- Enter answers with the dialog's default button, the one drawn highlighted: OK, Save, Reload. Escape keeps closing the dialog (cancelling) as before.
