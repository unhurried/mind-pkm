# Design

A status message remembers when it was set and is cleared by the first message the application handles after ten seconds (the 20-second tick makes sure one comes). The editor also clears it when a message changed the page's text without setting another one (so Replace All's `2 replaced` stays). The message sits in a clipped box that takes the room left by the counts and the zoom level.
