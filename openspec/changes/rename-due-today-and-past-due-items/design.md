# Design

The refresh fills `ToDo > Due Today` and `ToDo > Past Due` with `<item> (<page>)` nodes linking the page, the same form as the items Add to ToDo (F8) puts directly under ToDo. A rename therefore treats them alike: an item linking the old page file gets the new page name in place of the old one at the end of its text, and links the new file.

- Only the children of the two collections are added to the direct ToDo children already handled. Other nodes deeper under ToDo are the user's and are left alone, as the refresh leaves them alone.
- The collections are not filled again by the rename (that needs the pages read, which the refresh does in the background); renaming their items gives what the next refresh would produce, so that refresh finds them matching and leaves them untouched.
- Links to the old page elsewhere in the map are out of scope; the rename never followed them.
