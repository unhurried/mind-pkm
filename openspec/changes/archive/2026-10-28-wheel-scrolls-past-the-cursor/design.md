# Design

Before handling a message that may move the cursor (a text editor action, a key binding, a key no widget took, find, the rendered text widget), the editor notes the selection, the selected blocks, the active region and table cell, the source mode's cursor and the text's revision; it scrolls to the cursor afterwards only when one of them changed. Menu commands still always scroll to it. The trigger seen on Windows could not be reproduced under Xvfb; the change removes every way a message that leaves the cursor where it is can scroll the page.
