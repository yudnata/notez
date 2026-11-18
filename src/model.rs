use serde::{Deserialize, Serialize};

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
        self.text = new_text;
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
            let style = self.styles.get(i).copied().unwrap_or_default();
            let mut run = ch.to_string();
            while let Some(&(j, next)) = chars.peek() {
                if self.styles.get(j).copied().unwrap_or_default() != style {
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
    Note,
    Checklist,
    Kanban,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
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
        self.documents.push(Document {
            id,
            kind,
            title: match kind {
                Kind::Note => "Catatan baru",
                Kind::Checklist => "Daftar baru",
                Kind::Kanban => "Papan baru",
            }
            .into(),
            body: RichDocument::default(),
            spacing: 1.5,
            tasks: vec![],
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
}
