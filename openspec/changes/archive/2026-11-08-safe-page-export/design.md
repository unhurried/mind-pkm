## Decisions

Sanitize the complete rendered fragment using Ammonia's HTML5 parser, rather than filtering individual parser events or matching HTML with regular expressions. This covers raw HTML blocks, inline HTML, Markdown URLs, entity encoding and malformed markup together. Keep passive formatting, table alignment and text colors; remove scripts, event handlers, frames, forms and unsupported URL schemes. Permit image data URLs only on images.

Render math and Mermaid SVGs into image data URLs. Browsers treat SVG images as passive images, so document-controlled diagram links and markup cannot become active document content. This also lets the HTML allowlist exclude SVG and MathML namespaces entirely.

Place a CSP before document-controlled content. Allow inline CSS and supported image resources for HTML; PDF printing permits only local and embedded images/fonts. Disable scripts and background networking in the printing browser, with a fresh temporary profile. No active-content opt-in is added.

## Validation

Exercise raw script/event/frame/form/meta/base markup, obfuscated URLs, data URLs and renderer SVGs, and preserve ordinary formatting, tables, images, headings and generated diagrams. Load real export output in Chromium, inspect script markers and network requests, and print a PDF. Run the repository's format, lint, test and OpenSpec checks.

### Browser evidence and limits

Chromium displayed the actual HTML export with unchanged script markers, working local/remote images, table alignment, colors, disabled task checkboxes, math and Mermaid images. Injecting an inline script independently of sanitization was blocked by CSP. No script, frame, redirect or CSS tracking requests reached the local observer.

The managed browser blocks `file:` navigation (`ERR_BLOCKED_BY_ADMINISTRATOR`); the real file-based PDF invocation timed out. The exact print HTML captured from that invocation was instead loaded directly into Chromium with JavaScript disabled and the print flags applied. It produced a 31,807-byte PDF with unchanged markers, visible text/math/diagram images and zero HTTP requests to the local observer. File-based printing and Windows Edge remain to be verified on an unrestricted desktop.
