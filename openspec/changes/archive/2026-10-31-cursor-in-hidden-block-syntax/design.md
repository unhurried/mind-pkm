# Design

Placing the cursor kept the source offset it was given, and only the paths that start editing with no block active (the fix for issue #20) moved it to where the block shows it. The other paths - entering a neighbouring block, a press on the block beside its text, leaving selected blocks - pass the block's first source byte, which in a heading is before the `#`s.

- Placing the cursor in a live block now ends with the same move to the shown position as the no-block-active path: a caret in hidden source goes to the next shown position (the start of the text); a caret that shows where it is stays, and so does a selection.
- The move is not a break of the typing: placement also runs after every typed character, and a break there would split the undo steps of typing.
