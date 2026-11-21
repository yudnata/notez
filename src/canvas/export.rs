use super::{geometry as geo, model::*};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::fmt::Write;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn color(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
fn points(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| format!("{:.2},{:.2}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn text_lines(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    let limit = (max_width / (font_size * 0.46)).floor().max(1.0) as usize;
    let mut lines = vec![];
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if !line.is_empty() && line.chars().count() + word.chars().count() + 1 > limit {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    lines
}

impl CanvasData {
    pub fn content_bounds(&self) -> egui::Rect {
        self.elements
            .iter()
            .map(|e| geo::bounds(self, e))
            .reduce(|a, b| a.union(b))
            .unwrap_or(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 640.0),
            ))
            .expand(50.0)
    }

    pub fn svg(&self, title: &str, embed_font: bool) -> String {
        let rect = self.content_bounds();
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"{} {} {} {}\"><title>{}</title>",
            rect.width().ceil(),
            rect.height().ceil(),
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height(),
            escape(title)
        );
        if embed_font {
            let _ = write!(
                out,
                "<metadata id=\"patrick-hand-license\">{}</metadata>",
                escape(include_str!("../../assets/fonts/OFL-PatrickHand.txt"))
            );
            let data =
                STANDARD.encode(include_bytes!("../../assets/fonts/PatrickHand-Regular.ttf"));
            let _ = write!(
                out,
                "<style>@font-face{{font-family:'Patrick Hand';src:url(data:font/ttf;base64,{data}) format('truetype')}}</style>"
            );
        }
        out.push_str("<defs><pattern id=\"grid\" width=\"24\" height=\"24\" patternUnits=\"userSpaceOnUse\"><path d=\"M 24 0 L 0 0 0 24\" fill=\"none\" stroke=\"#e9e9e7\" stroke-width=\"0.7\"/></pattern><pattern id=\"ruled\" width=\"24\" height=\"32\" patternUnits=\"userSpaceOnUse\"><path d=\"M 0 32 H 24\" stroke=\"#e6e6e4\" stroke-width=\"0.8\"/></pattern></defs>");
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>",
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height()
        );
        if self.paper != Paper::Blank {
            let _ = write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"url(#{})\"/>",
                rect.min.x,
                rect.min.y,
                rect.width(),
                rect.height(),
                if self.paper == Paper::Grid {
                    "grid"
                } else {
                    "ruled"
                }
            );
        }
        for element in &self.elements {
            let stroke = color(element.style.stroke);
            let alpha = f32::from(element.style.stroke[3]) / 255.0;
            let width = element.style.width;
            if element.kind == ElementKind::Image {
                if let Some(image) = &element.image {
                    let _ = write!(
                        out,
                        "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" xlink:href=\"data:{};base64,{}\"/>",
                        element.pos[0],
                        element.pos[1],
                        element.size[0],
                        element.size[1],
                        image.mime,
                        STANDARD.encode(image.bytes.as_slice())
                    );
                }
            } else if element.kind.has_path() {
                let path = if element.kind == ElementKind::Arrow {
                    geo::arrow_path(self, element)
                } else {
                    element.points.clone()
                };
                let _ = write!(
                    out,
                    "<polyline points=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-opacity=\"{alpha}\" stroke-width=\"{width}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
                    points(&path)
                );
                if element.kind == ElementKind::Arrow {
                    let _ = write!(
                        out,
                        "<polyline points=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
                        points(&geo::arrowhead(&path, width))
                    );
                }
            } else if element.kind != ElementKind::Text {
                if element.kind == ElementKind::Sticky {
                    let _ = write!(
                        out,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#000000\" opacity=\"0.07\"/>",
                        element.pos[0] + 3.0,
                        element.pos[1] + 4.0,
                        element.size[0],
                        element.size[1]
                    );
                }
                let _ = write!(
                    out,
                    "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{stroke}\" stroke-opacity=\"{alpha}\" stroke-width=\"{width}\" stroke-linejoin=\"round\"/>",
                    points(&geo::outline(element)),
                    color(element.style.fill),
                    f32::from(element.style.fill[3]) / 255.0
                );
            }
            if !element.text.is_empty() {
                let size = element.style.font_size;
                let (center, max_width) = if element.kind.has_path() {
                    let path = if element.kind == ElementKind::Arrow {
                        geo::arrow_path(self, element)
                    } else {
                        element.points.clone()
                    };
                    (path[path.len() / 2], 200.0)
                } else {
                    (element.center(), (element.size[0] - 20.0).max(20.0))
                };
                let lines = text_lines(&element.text, max_width, size);
                let first_y = center[1] - (lines.len() as f32 - 1.0) * size * 0.625 + size * 0.3;
                if element.kind.has_path() {
                    let _ = write!(
                        out,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"#ffffff\"/>",
                        center[0] - 100.0,
                        first_y - size,
                        200,
                        lines.len() as f32 * size * 1.25
                    );
                }
                let _ = write!(
                    out,
                    "<text text-anchor=\"middle\" font-family=\"Patrick Hand\" font-size=\"{size}\" fill=\"#29292d\">"
                );
                for (i, line) in lines.iter().enumerate() {
                    let _ = write!(
                        out,
                        "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                        center[0],
                        first_y + i as f32 * size * 1.25,
                        escape(line)
                    );
                }
                out.push_str("</text>");
            }
        }
        out.push_str("</svg>");
        out
    }

    pub fn png(&self, title: &str) -> Result<Vec<u8>, String> {
        let svg = self.svg(title, false);
        let mut options = resvg::usvg::Options::default();
        options
            .fontdb_mut()
            .load_font_data(include_bytes!("../../assets/fonts/PatrickHand-Regular.ttf").to_vec());
        let tree = resvg::usvg::Tree::from_str(&svg, &options)
            .map_err(|e| format!("Gagal menyiapkan ekspor: {e}"))?;
        let size = tree.size();
        let scale = 2.0_f32
            .min(8192.0 / size.width().max(size.height()))
            .min((24_000_000.0 / (size.width() * size.height())).sqrt());
        let mut pixmap = resvg::tiny_skia::Pixmap::new(
            (size.width() * scale).ceil().max(1.0) as u32,
            (size.height() * scale).ceil().max(1.0) as u32,
        )
        .ok_or("Canvas terlalu besar untuk diekspor sebagai PNG.")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        pixmap
            .encode_png()
            .map_err(|e| format!("Gagal membuat PNG: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_escape_text_preserve_font_and_render_png() {
        let mut data = CanvasData::default();
        let mut element =
            Element::new(data.id(), ElementKind::Sticky, [0.0, 0.0], Style::default());
        element.text = "<Halo> & dunia".into();
        element.style.fill = [255, 227, 124, 255];
        data.elements.push(element);
        let svg = data.svg("<judul>", true);
        assert!(svg.contains("&lt;Halo&gt;") && svg.contains("&amp;") && svg.contains("dunia"));
        assert!(svg.contains("data:font/ttf;base64,"));
        assert!(svg.contains("Copyright (c) 2010-2012 Patrick Wagesreiter"));
        assert!(svg.contains("SIL OPEN FONT LICENSE Version 1.1"));
        assert!(data.png("Test").unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
