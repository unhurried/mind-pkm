//! Math and Mermaid diagrams as SVG, by pure-Rust renderers. Slow enough
//! (milliseconds) to run in the background; the editor caches the results.

use std::sync::LazyLock;

static MATH_FONT: LazyLock<Option<latex_rust::MathFont>> = LazyLock::new(|| latex_rust::MathFont::stix_two_math().ok());

/// The size math is typeset at, in points (the editor scales the SVG).
pub const MATH_POINTS: i64 = 12;

/// Typesets LaTeX math (`display` for a math block), drawn in `rgb`.
pub fn math_svg(tex: &str, display: bool, rgb: (u8, u8, u8)) -> Result<String, String> {
    let font = MATH_FONT.as_ref().ok_or("the math font is missing")?;
    let mut options = latex_rust::SvgOptions::new();
    options.display = display;
    options.font_size_pt = latex_rust::Dim::from_i64(MATH_POINTS);
    options.color = latex_rust::Color::rgb(rgb.0, rgb.1, rgb.2);
    latex_rust::latex_to_svg(tex, font, &options).map_err(|e| format!("{e}"))
}

/// Draws a Mermaid diagram.
pub fn mermaid_svg(source: &str) -> Result<String, String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("empty diagram".into());
    }
    mermaid_rs_renderer::render_strict(source, Default::default()).map_err(|e| e.to_string())
}

/// The width and height an SVG document declares (points converted to pixels).
pub fn svg_size(svg: &str) -> Option<(f32, f32)> {
    let start = svg.find("<svg")?;
    let end = start + svg[start..].find('>')?;
    let tag = &svg[start..end];
    let attr = |name: &str| -> Option<f32> {
        let key = format!(" {name}=\"");
        let s = tag.find(&key)? + key.len();
        let value = &tag[s..s + tag[s..].find('"')?];
        let (number, factor) = match value.strip_suffix("pt") {
            Some(n) => (n, 4.0 / 3.0),
            None => (value.strip_suffix("px").unwrap_or(value), 1.0),
        };
        number.trim().parse::<f32>().ok().map(|n| n * factor)
    };
    Some((attr("width")?, attr("height")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_is_typeset() {
        let svg = math_svg(r"\frac{1}{2}", true, (0, 0, 0)).unwrap();
        let (w, h) = svg_size(&svg).unwrap();
        assert!(w > 0.0 && h > 0.0);
        assert!(math_svg(r"\frac{1}{", false, (0, 0, 0)).is_err());
    }

    #[test]
    fn mermaid_is_drawn_and_errors_are_reported() {
        let svg = mermaid_svg("flowchart LR\n  A --> B").unwrap();
        assert!(svg_size(&svg).is_some());
        assert!(mermaid_svg("flowchart LR\n  A -->").is_err());
    }

    #[test]
    fn sizes() {
        assert_eq!(svg_size(r#"<?xml?><svg width="30pt" height="15pt">"#), Some((40.0, 20.0)));
        assert_eq!(svg_size(r#"<svg x="1" width="10" height="20" viewBox="0 0 1 1">"#), Some((10.0, 20.0)));
        assert_eq!(svg_size("<p>"), None);
    }
}
