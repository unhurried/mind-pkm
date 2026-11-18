## Why

Search indexes are generated per-user data. Storing them with the app's settings keeps generated search files out of the document directory.

## What Changes

- Store each document directory's index and modification-time metadata under the app's per-user settings directory.
- Build the cache in its new location on the next index update. Leave existing Tantivy and Freeplane Lucene files in the document directory.

## Capabilities

### Modified Capabilities

- `document-search` stores its cache by document directory under the user's settings.

## Impact

Search and indexing use the new cache location. The first update in that location reindexes the document directory.
