# Design

## Decoding

`data:[<media type>][;base64],<data>` (RFC 2397). The data is percent-decoded first, then, with `;base64`, base64-decoded leniently: ASCII whitespace is dropped and padding is optional (the WHATWG "forgiving" decode), since such URLs are often wrapped or copied without padding. The media type must start with `image/` (case-insensitive); `image/svg+xml` is drawn as SVG, every other one as a raster image whose format is guessed from the bytes. A `data:` URL that fails any of this is an invalid image.

## Caching

iced gives an image handle made from bytes a new id each time (`image::Handle::from_bytes`), and the renderer keys its decoded textures by that id: a handle made in every `view` would decode and upload the image on every redraw. The editor keeps the handles it made, keyed by a hash of the `src`, in a `RefCell` like the image files it looked up. When the page's revision changes, only the handles used during the previous revision are kept, so images removed from the page are dropped, and images still on it keep their handle.

## Not changed

The image file lookup, remote images and export: a `data:` URL is not a local file (`local_path` already returns none for it), and HTML/PDF export writes it as it is.
