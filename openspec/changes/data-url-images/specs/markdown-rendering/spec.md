## MODIFIED Requirements

### Requirement: Images
`![alt](src "title")` and `<img src>` SHALL show the image file (PNG, JPEG, GIF, BMP, WebP, ICO, TIFF, SVG), scaled down to the width of the document when wider. A `src` that is a `data:` URL of an image media type, base64- or percent-encoded, SHALL show the image it holds in the same way. A missing file, and a `data:` URL that holds no image or cannot be decoded, SHALL show the alt text with a broken-image mark; `http`/`https` images SHALL NOT be downloaded but shown as their alt text with a link to the URL.

#### Scenario: Image in the assets folder
- **WHEN** the page `Trip.md` contains `![](Trip.assets/map.png)` and that file exists
- **THEN** the image is shown

#### Scenario: Remote image
- **WHEN** a page contains `![logo](https://example.com/logo.png)`
- **THEN** `logo` is shown with a link to the URL and nothing is downloaded

#### Scenario: Embedded image
- **WHEN** a page contains `![dot](data:image/png;base64,iVBORw0KGgo...)` holding a PNG
- **THEN** the PNG is shown

#### Scenario: Broken embedded image
- **WHEN** a page contains `![dot](data:text/plain,hello)`
- **THEN** `dot` is shown with a broken-image mark
