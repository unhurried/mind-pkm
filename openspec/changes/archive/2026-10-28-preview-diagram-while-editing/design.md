# Design

Diagrams are rendered in the background and cached by their source. The renders were requested with the block's code as the parser gives it (ending with a line break), but the preview of the block being edited looked its diagram up by the code cut from the block's text without that line break, so it never found it. The preview now takes the code of the block from the parser too, the same way the requests do.
