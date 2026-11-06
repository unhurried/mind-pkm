## ADDED Requirements

### Requirement: Embedded images
An image whose `src` is a `data:` URL of an image media type, base64- or percent-encoded, SHALL show the image the URL holds, like an image file. A `data:` URL that holds no image or cannot be decoded SHALL show the alt text with a broken-image mark.

#### Scenario: Embedded image
- **WHEN** a page contains `![dot](data:image/png;base64,iVBORw0KGgo...)` holding a PNG
- **THEN** the PNG is shown

#### Scenario: Broken embedded image
- **WHEN** a page contains `![dot](data:text/plain,hello)`
- **THEN** `dot` is shown with a broken-image mark
