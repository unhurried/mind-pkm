## Decisions

Use `<settings folder>/.search-index/<document directory ID>/` for each document directory. Derive the ID from the canonical absolute path so that two directories with the same name have separate indexes and aliases of the same directory can share an index. A moved document directory gets a new cache.

Apply the existing link and ownership checks to the new cache. Hold an update lock in the per-directory cache folder through the index commit and metadata write. When rebuilding, clear outdated metadata before replacing the index so the next update will retry an interrupted rebuild.

## Validation

Check that searches for two document directories with the same name use separate caches, a new cache builds from documents while old indexes are preserved, and linked or unowned cache contents are rejected. Run the repository's formatting, lint, test and OpenSpec checks.
