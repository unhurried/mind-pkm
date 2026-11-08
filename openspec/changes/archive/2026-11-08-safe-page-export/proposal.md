## Why

SEC-02 in the security assessment shows that untrusted Markdown can execute JavaScript in exported HTML and in the browser used to print PDFs.

## What Changes

- Sanitize rendered HTML with a tag, attribute, CSS property and URL scheme allowlist, retaining supported passive formatting.
- Embed generated math and diagram SVGs as passive image data rather than active inline markup.
- Add a restrictive Content Security Policy, with local-only resources and JavaScript disabled for PDF printing.
- Cover malicious content and ordinary rendering with regression tests and a Chromium smoke test.

## Capabilities

### Modified Capabilities
- `page-export`: passive HTML/PDF export and restrictions on print-time resource loading.

## Impact

HTML generation (also used by rich-text copying), PDF browser invocation, the export documentation, and the Ammonia sanitizer dependency. The Markdown source is preserved.
