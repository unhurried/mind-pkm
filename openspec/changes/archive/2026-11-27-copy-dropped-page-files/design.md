## Context

Window file drops share the editor's insertion path with clipboard files. That path copies images according to their settings and links to other paths in place.

## Goals / Non-Goals

- Store dropped non-image files alongside their page and link to the stored copy.
- Keep the current insertion position and image preferences.

## Decisions

Handle non-image file drops separately in the background and pass their Markdown through the existing editor insertion result. Derive the destination from the page path. Stage the copy in the assets folder and publish it without replacing existing files, retrying with a numbered name on a collision.

## Risks / Trade-offs

Copying takes disk space. A failed copy is reported by the editor and does not insert a link.
