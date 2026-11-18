## MODIFIED Requirements

### Requirement: Index location
Each document directory SHALL have a cache in `<settings folder>/.search-index/<document directory ID>/`. The ID SHALL be derived from its resolved absolute path. The cache SHALL hold `tantivy/` and `tantivy.meta`, which records the schema version and file modification times. The first update without this cache SHALL build it from the documents and preserve the files under the document directory's `.search-index/`.

#### Scenario: Used next to the add-on
- **WHEN** the index is updated in a document directory that holds an old Tantivy index and the Freeplane add-on's index
- **THEN** a new index is built from the documents under the settings folder, and the old Tantivy and Lucene files remain in the document directory

#### Scenario: Document directories with the same name
- **WHEN** two document directories have the same name under different parent folders
- **THEN** each uses its own cache and searches return results from the chosen directory

### Requirement: Cache link boundaries and ownership
Cache reads, writes and rebuilds SHALL refuse links and reparse points below the app's settings directory in the cache path, metadata, and Tantivy contents. Rebuild SHALL remove a nonempty Tantivy cache only when it carries the application's ownership marker or is an earlier index with the application's schema. Unrecognized nonempty folders SHALL be preserved and an error returned. Index version 1 SHALL be rebuilt before results are exposed, discarding content previously collected through links.

#### Scenario: The cache path links to another folder
- **WHEN** the settings folder's `.search-index` links to another folder
- **THEN** update and search return an error and the linked folder is preserved

#### Scenario: Cache root links outside the document directory
- **WHEN** `.search-index` in the document directory links to an external folder
- **THEN** update and search use the settings-folder cache and preserve the external folder

#### Scenario: Unrecognized cache contents
- **WHEN** the new cache's `tantivy/` folder contains unrelated files and is not an application-owned index
- **THEN** rebuild returns an error and preserves those files

### Requirement: Index updates
Index updates SHALL be incremental (re-indexing only new and modified files and dropping deleted ones), SHALL rebuild everything when the recorded schema version differs or the index is missing, and SHALL run in the background when the search panel opens, on F12, and with the periodic refresh.

#### Scenario: Deleted file
- **WHEN** an indexed file is deleted and the index is updated
- **THEN** the file is no longer found

#### Scenario: Index removed by hand
- **WHEN** a document directory's `tantivy/` folder under the settings folder is deleted and the index is updated
- **THEN** every file is indexed again
