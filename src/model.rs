use crate::canvas::model::ImageData;
use serde::{Deserialize, Serialize};

pub const IMAGE_MARKER: char = '\u{fffc}';

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageLayout {
    #[default]
    Text,
    Overlay,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteImage {
    pub id: u64,
    pub anchor: usize,
    pub data: ImageData,
    pub name: String,
    #[serde(default)]
    pub layout: ImageLayout,
    pub width: f32,
    #[serde(default)]
    pub offset: [f32; 2],
}

impl NoteImage {
    pub fn display_size(&self, available: f32) -> egui::Vec2 {
        self.data
            .size_at_width(self.width.min(available.max(1.0)))
            .into()
    }
}

#[cfg(test)]
pub fn test_image() -> ImageData {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(320, 160, image::Rgba([242, 211, 150, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    ImageData {
        bytes: std::sync::Arc::new(bytes.into_inner()),
        mime: "image/png".into(),
        width: 320,
        height: 160,
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: 18.0,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct RichDocument {
    pub text: String,
    pub styles: Vec<TextStyle>,
    #[serde(default)]
    pub images: Vec<NoteImage>,
}

impl RichDocument {
    pub fn reconcile(&mut self, new_text: String, typing: TextStyle) {
        let old: Vec<char> = self.text.chars().collect();
        let new: Vec<char> = new_text.chars().collect();
        self.styles.resize(old.len(), TextStyle::default());
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        self.styles.splice(
            prefix..old.len() - suffix,
            std::iter::repeat_n(typing, new.len() - prefix - suffix),
        );
        let removed_end = old.len() - suffix;
        let inserted = new.len() - prefix - suffix;
        self.images.retain_mut(|image| {
            if image.anchor >= removed_end {
                image.anchor = image.anchor - (removed_end - prefix) + inserted;
            } else if image.anchor >= prefix {
                return false;
            }
            new.get(image.anchor) == Some(&IMAGE_MARKER)
        });
        self.text = new_text;
    }

    pub fn insert_image(
        &mut self,
        at: usize,
        data: ImageData,
        name: String,
        typing: TextStyle,
    ) -> u64 {
        let mut chars: Vec<_> = self.text.chars().collect();
        let at = at.min(chars.len());
        let leading = usize::from(at > 0 && chars[at - 1] != '\n');
        let insert = format!("{}{}\n", if leading == 1 { "\n" } else { "" }, IMAGE_MARKER);
        chars.splice(at..at, insert.chars());
        let mut images = self.images.clone();
        for image in &mut images {
            if image.anchor >= at {
                image.anchor += insert.chars().count();
            }
        }
        self.reconcile(chars.into_iter().collect(), typing);
        self.images = images;
        let id = self.images.iter().map(|image| image.id).max().unwrap_or(0) + 1;
        let width = (360.0 / data.width.max(data.height) as f32).min(1.0) * data.width as f32;
        self.images.push(NoteImage {
            id,
            anchor: at + leading,
            data,
            name,
            layout: ImageLayout::Text,
            width,
            offset: [0.0, 0.0],
        });
        id
    }

    pub fn remove_image(&mut self, id: u64) {
        if let Some(image) = self.images.iter().find(|image| image.id == id) {
            let anchor = image.anchor;
            let mut images = self.images.clone();
            images.retain_mut(|image| {
                if image.id == id {
                    return false;
                }
                if image.anchor > anchor {
                    image.anchor -= 1;
                }
                true
            });
            let mut chars: Vec<_> = self.text.chars().collect();
            chars.remove(anchor);
            self.reconcile(chars.into_iter().collect(), TextStyle::default());
            self.images = images;
        }
    }

    pub fn plain_text(&self) -> String {
        self.text.replace(IMAGE_MARKER, "[Gambar]")
    }

    pub fn validate_images(&mut self) -> Result<(), String> {
        let chars: Vec<_> = self.text.chars().collect();
        let mut ids = std::collections::HashSet::new();
        let mut anchors = std::collections::HashSet::new();
        for image in &mut self.images {
            image.data.validate()?;
            if !ids.insert(image.id)
                || !anchors.insert(image.anchor)
                || chars.get(image.anchor) != Some(&IMAGE_MARKER)
                || !image.width.is_finite()
                || !image
                    .offset
                    .iter()
                    .all(|n| n.is_finite() && n.abs() < 1.0e7)
            {
                return Err("Posisi atau ukuran gambar catatan tidak valid.".into());
            }
            image.width = image.width.clamp(30.0, 4000.0);
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn apply(&mut self, range: std::ops::Range<usize>, style: TextStyle) {
        self.styles
            .resize(self.text.chars().count(), TextStyle::default());
        let end = range.end.min(self.styles.len());
        let start = range.start.min(end);
        self.styles[start..end].fill(style);
    }

    pub fn html(&self, title: &str, spacing: f32) -> String {
        let mut body = String::new();
        let mut chars = self.text.chars().enumerate().peekable();
        while let Some((i, ch)) = chars.next() {
            if ch == IMAGE_MARKER {
                if let Some(image) = self.images.iter().find(|image| image.anchor == i) {
                    use base64::{Engine, engine::general_purpose::STANDARD};
                    let src = format!(
                        "data:{};base64,{}",
                        image.data.mime,
                        STANDARD.encode(image.data.bytes.as_slice())
                    );
                    let tag = format!(
                        "<img alt=\"{}\" src=\"{src}\" style=\"width:{}px;max-width:100%;height:auto;vertical-align:top\">",
                        escape(&image.name),
                        image.width
                    );
                    if image.layout == ImageLayout::Overlay {
                        body.push_str(&format!("<span style=\"position:relative;display:inline-block;width:0;height:0\"><span style=\"position:absolute;left:{}px;top:{}px;width:{}px\">{tag}</span></span>", image.offset[0], image.offset[1], image.width));
                    } else {
                        body.push_str(&tag);
                    }
                }
                continue;
            }
            let style = self.styles.get(i).copied().unwrap_or_default();
            let mut run = ch.to_string();
            while let Some(&(j, next)) = chars.peek() {
                if next == IMAGE_MARKER || self.styles.get(j).copied().unwrap_or_default() != style
                {
                    break;
                }
                run.push(next);
                chars.next();
            }
            body.push_str(&format!(
                "<span style=\"font-size:{}px;font-weight:{};font-style:{};text-decoration:{}\">{}</span>",
                style.size, if style.bold { "700" } else { "400" },
                if style.italic { "italic" } else { "normal" },
                if style.underline { "underline" } else { "none" }, escape(&run)));
        }
        format!(
            "<!doctype html><html lang=\"id\"><meta charset=\"utf-8\"><title>{}</title><style>body{{max-width:800px;margin:60px auto;padding:24px;font-family:system-ui;line-height:{spacing}}}article{{white-space:pre-wrap;overflow-wrap:anywhere}}</style><h1>{}</h1><article>{body}</article></html>",
            escape(title),
            escape(title)
        )
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Canvas,
    Note,
    Checklist,
    Kanban,
}

impl Kind {
    pub const NAVIGATION: [Self; 3] = [Self::Canvas, Self::Note, Self::Checklist];

    pub fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Canvas",
            Self::Note => "Notes",
            Self::Checklist => "Checklist",
            Self::Kanban => "Kanban",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: u64,
    pub text: String,
    pub detail: String,
    pub done: bool,
    pub column: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: u64,
    pub title: String,
    pub kind: Kind,
    pub body: RichDocument,
    pub spacing: f32,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub canvas: crate::canvas::CanvasData,
    #[serde(default = "default_note_color")]
    pub note_color: [u8; 3],
    #[serde(default)]
    pub note_position: Option<[f32; 2]>,
}

pub const NOTE_COLORS: [[u8; 3]; 6] = [
    [248, 225, 151],
    [247, 213, 184],
    [240, 203, 213],
    [221, 212, 240],
    [199, 220, 238],
    [210, 225, 199],
];
pub const NOTE_COLOR_NAMES: [&str; 6] =
    ["Kuning", "Peach", "Merah muda", "Lavender", "Biru", "Sage"];

pub fn closest_note_color(color: [u8; 3]) -> [u8; 3] {
    *NOTE_COLORS
        .iter()
        .min_by_key(|candidate| {
            candidate
                .iter()
                .zip(color)
                .map(|(a, b)| (i32::from(*a) - i32::from(b)).pow(2))
                .sum::<i32>()
        })
        .unwrap()
}
fn default_note_color() -> [u8; 3] {
    NOTE_COLORS[0]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub version: u32,
    pub next_id: u64,
    pub documents: Vec<Document>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            documents: vec![],
        }
    }
}

impl Workspace {
    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add(&mut self, kind: Kind) -> u64 {
        let id = self.id();
        let note_color = NOTE_COLORS
            [self.documents.iter().filter(|d| d.kind == kind).count() % NOTE_COLORS.len()];
        self.documents.push(Document {
            id,
            kind,
            title: match kind {
                Kind::Canvas => "Canvas baru",
                Kind::Note => "Catatan baru",
                Kind::Checklist => "Daftar baru",
                Kind::Kanban => "Papan baru",
            }
            .into(),
            body: RichDocument::default(),
            spacing: 1.5,
            tasks: vec![],
            canvas: crate::canvas::CanvasData::default(),
            note_color,
            note_position: None,
        });
        id
    }

    pub fn validate(&mut self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Versi file belum didukung oleh aplikasi ini.".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut max_id = 0;
        for doc in &mut self.documents {
            doc.note_color = closest_note_color(doc.note_color);
            doc.canvas.validate()?;
            doc.body.validate_images()?;
            if doc
                .note_position
                .is_some_and(|p| !p.iter().all(|n| n.is_finite() && *n >= 0.0 && *n < 1.0e7))
            {
                return Err("Posisi kartu catatan tidak valid.".into());
            }
            if !ids.insert(doc.id) {
                return Err("File berisi ID dokumen yang berulang.".into());
            }
            max_id = max_id.max(doc.id);
            if !doc.spacing.is_finite() {
                return Err("Jarak baris tidak valid.".into());
            }
            doc.spacing = doc.spacing.clamp(1.0, 2.5);
            doc.body
                .styles
                .resize(doc.body.text.chars().count(), TextStyle::default());
            for style in &mut doc.body.styles {
                if !style.size.is_finite() {
                    return Err("Ukuran huruf tidak valid.".into());
                }
                style.size = style.size.clamp(12.0, 48.0);
            }
            for task in &mut doc.tasks {
                if !ids.insert(task.id) {
                    return Err("File berisi ID tugas yang berulang.".into());
                }
                max_id = max_id.max(task.id);
                task.column = task.column.min(2);
            }
        }
        self.next_id = self
            .next_id
            .max(max_id.checked_add(1).ok_or("ID terlalu besar.")?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_insert_delete_preserves_neighbor_formatting() {
        let bold = TextStyle {
            bold: true,
            ..Default::default()
        };
        let mut doc = RichDocument::default();
        doc.reconcile("A猫B".into(), bold);
        doc.reconcile("A🌱猫B".into(), TextStyle::default());
        assert_eq!(doc.styles, vec![bold, TextStyle::default(), bold, bold]);
        doc.reconcile("A🌱B".into(), TextStyle::default());
        assert_eq!(doc.styles, vec![bold, TextStyle::default(), bold]);
    }

    #[test]
    fn html_preserves_format_and_escapes_markup() {
        let mut doc = RichDocument::default();
        doc.reconcile(
            "<script>猫&".into(),
            TextStyle {
                bold: true,
                ..Default::default()
            },
        );
        let html = doc.html("<judul>", 1.8);
        assert!(html.contains("&lt;script&gt;猫&amp;"));
        assert!(html.contains("font-weight:700"));
        assert!(html.contains("line-height:1.8"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn images_follow_unicode_edits_and_deleted_anchors_remove_the_image() {
        let mut doc = RichDocument::default();
        doc.reconcile("猫\nSesudah".into(), TextStyle::default());
        let first = doc.insert_image(2, test_image(), "satu.png".into(), TextStyle::default());
        let second = doc.insert_image(2, test_image(), "dua.png".into(), TextStyle::default());
        doc.validate_images().unwrap();
        assert_eq!(doc.images.iter().find(|i| i.id == first).unwrap().anchor, 4);
        assert_eq!(
            doc.images.iter().find(|i| i.id == second).unwrap().anchor,
            2
        );
        doc.reconcile(format!("🌻{}", doc.text), TextStyle::default());
        assert_eq!(doc.images.iter().find(|i| i.id == first).unwrap().anchor, 5);
        doc.remove_image(second);
        doc.validate_images().unwrap();
        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].anchor, 4);
        let saved = doc.clone();
        doc.reconcile(doc.text.replace(IMAGE_MARKER, ""), TextStyle::default());
        assert!(doc.images.is_empty());
        assert_eq!(saved.images.len(), 1);
        assert!(saved.text.contains("猫"));
    }

    #[test]
    fn image_layout_defaults_to_text_and_exports_embedded_data() {
        let mut doc = RichDocument::default();
        doc.insert_image(0, test_image(), "<foto>.png".into(), TextStyle::default());
        assert_eq!(doc.images[0].layout, ImageLayout::Text);
        let html = doc.html("Foto", 1.5);
        assert!(html.contains("data:image/png;base64,"));
        assert!(html.contains("alt=\"&lt;foto&gt;.png\""));
        assert!(html.contains("height:auto"));
        assert!(!html.contains(IMAGE_MARKER));
        doc.images[0].layout = ImageLayout::Overlay;
        doc.images[0].offset = [35.0, -20.0];
        assert!(doc.html("Foto", 1.5).contains("left:35px;top:-20px"));
        let bytes = serde_json::to_vec(&doc).unwrap();
        assert_eq!(serde_json::from_slice::<RichDocument>(&bytes).unwrap(), doc);
        let legacy: RichDocument =
            serde_json::from_str(r#"{"text":"Catatan lama","styles":[]}"#).unwrap();
        assert!(legacy.images.is_empty());
    }

    #[test]
    fn image_resizing_preserves_original_ratio_for_both_axes_and_corner() {
        let image = test_image();
        for size in [
            image.size_at_width(450.0),
            image.size_at_height(300.0),
            image.resize_corner([210.0, 460.0]),
        ] {
            assert!((size[0] / size[1] - 2.0).abs() < 0.001);
        }
    }
}
