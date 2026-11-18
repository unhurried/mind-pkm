## 1. Implementation

- [x] 1.1 Share the app's settings directory lookup with search and use a separate cache per document directory.
- [x] 1.2 Preserve cache path and ownership checks and keep index updates consistent across retries and multiple app instances.
- [x] 1.3 Document the new location and the treatment of existing files.

## 2. Verification

- [x] 2.1 Check placement, isolation, first-time rebuild, and existing search and filesystem behavior, including F6 in the Linux app with a read-only document directory.
- [x] 2.2 Run formatting, lint, tests and strict OpenSpec validation, then archive the change.
