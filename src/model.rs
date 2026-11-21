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
    #[serde(default)]
    pub align: TextAlign,
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
        encoded_cache: std::sync::Arc::new(std::sync::OnceLock::new()),
        mime: "image/png".into(),
        width: 320,
        height: 160,
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl TextAlign {
    pub const ALL: [Self; 4] = [Self::Left, Self::Center, Self::Right, Self::Justify];

    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Rata kiri",
            Self::Center => "Rata tengah",
            Self::Right => "Rata kanan",
            Self::Justify => "Rata kiri-kanan",
        }
    }

    pub fn shortcut_hint(self) -> &'static str {
        match self {
            Self::Left => "Rata kiri · Ctrl+L",
            Self::Center => "Rata tengah · Ctrl+E",
            Self::Right => "Rata kanan · Ctrl+R",
            Self::Justify => "Rata kiri-kanan (Justify) · Ctrl+J",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Justify => "justify",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    Bullet,
    NumberDot,
    NumberParen,
    LowerDot,
    LowerParen,
    UpperDot,
    UpperParen,
}

fn alpha_label(mut step: usize, upper: bool) -> String {
    let base = if upper { b'A' } else { b'a' };
    let mut chars = Vec::new();
    while step > 0 {
        let rem = (step - 1) % 26;
        chars.push((base + rem as u8) as char);
        step = (step - 1) / 26;
    }
    chars.into_iter().rev().collect()
}

impl ListKind {
    pub const TOOLBAR: [Self; 5] = [
        Self::Bullet,
        Self::NumberDot,
        Self::NumberParen,
        Self::LowerDot,
        Self::LowerParen,
    ];

    pub fn button_label(self) -> &'static str {
        match self {
            Self::Bullet => "•",
            Self::NumberDot => "1.",
            Self::NumberParen => "1)",
            Self::LowerDot => "a.",
            Self::LowerParen => "a)",
            Self::UpperDot => "A.",
            Self::UpperParen => "A)",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Bullet => "Daftar poin (•) · ketik - atau * lalu spasi",
            Self::NumberDot => "Penomoran 1. 2. 3. · ketik 1. lalu spasi",
            Self::NumberParen => "Penomoran 1) 2) 3) · ketik 1) lalu spasi",
            Self::LowerDot => "Penomoran a. b. c. · ketik a. lalu spasi",
            Self::LowerParen => "Penomoran a) b) c) · ketik a) lalu spasi",
            Self::UpperDot => "Penomoran A. B. C.",
            Self::UpperParen => "Penomoran A) B) C)",
        }
    }

    pub fn prefix(self, step: usize) -> String {
        let n = step.max(1);
        match self {
            Self::Bullet => "• ".into(),
            Self::NumberDot => format!("{n}. "),
            Self::NumberParen => format!("{n}) "),
            Self::LowerDot => format!("{}. ", alpha_label(n, false)),
            Self::LowerParen => format!("{}) ", alpha_label(n, false)),
            Self::UpperDot => format!("{}. ", alpha_label(n, true)),
            Self::UpperParen => format!("{}) ", alpha_label(n, true)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedListPrefix {
    pub kind: ListKind,
    pub step: usize,
    pub indent: String,
    pub indent_chars: usize,
    pub prefix_chars: usize,
    pub content_empty: bool,
}

pub fn parse_list_prefix(line: &str) -> Option<ParsedListPrefix> {
    let chars: Vec<char> = line.chars().collect();
    let indent_chars = chars.iter().take_while(|&&c| c == ' ' || c == '\t').count();
    let indent: String = chars[..indent_chars].iter().collect();
    let rest = &chars[indent_chars..];
    if rest.len() >= 2 && matches!(rest[0], '•' | '-' | '*') && rest[1] == ' ' {
        let prefix_chars = indent_chars + 2;
        let content_empty = chars[prefix_chars..].iter().all(|c| c.is_whitespace());
        return Some(ParsedListPrefix {
            kind: ListKind::Bullet,
            step: 1,
            indent,
            indent_chars,
            prefix_chars,
            content_empty,
        });
    }
    let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if (1..=6).contains(&digits) && rest.len() >= digits + 2 && rest[digits + 1] == ' ' {
        let kind = match rest[digits] {
            '.' => Some(ListKind::NumberDot),
            ')' => Some(ListKind::NumberParen),
            _ => None,
        };
        if let Some(kind) = kind {
            let num_str: String = rest[..digits].iter().collect();
            if let Ok(step) = num_str.parse::<usize>()
                && step > 0
            {
                let prefix_chars = indent_chars + digits + 2;
                let content_empty = chars[prefix_chars..].iter().all(|c| c.is_whitespace());
                return Some(ParsedListPrefix {
                    kind,
                    step,
                    indent,
                    indent_chars,
                    prefix_chars,
                    content_empty,
                });
            }
        }
    }
    if rest.len() >= 3 && rest[0].is_ascii_alphabetic() && rest[2] == ' ' {
        let ch = rest[0];
        let upper = ch.is_ascii_uppercase();
        let step = if upper {
            (ch as u8 - b'A' + 1) as usize
        } else {
            (ch as u8 - b'a' + 1) as usize
        };
        let kind = match (upper, rest[1]) {
            (false, '.') => Some(ListKind::LowerDot),
            (false, ')') => Some(ListKind::LowerParen),
            (true, '.') => Some(ListKind::UpperDot),
            (true, ')') => Some(ListKind::UpperParen),
            _ => None,
        };
        if let Some(kind) = kind {
            let prefix_chars = indent_chars + 3;
            let content_empty = chars[prefix_chars..].iter().all(|c| c.is_whitespace());
            return Some(ParsedListPrefix {
                kind,
                step,
                indent,
                indent_chars,
                prefix_chars,
                content_empty,
            });
        }
    }
    None
}

pub fn visual_list_prefix(line: &str) -> Option<ParsedListPrefix> {
    let parsed = parse_list_prefix(line)?;
    if parsed.kind == ListKind::Bullet && line.chars().nth(parsed.indent_chars) != Some('•') {
        return None;
    }
    Some(parsed)
}

pub fn auto_list_transform(
    old_text: &str,
    new_text: &str,
    cursor: usize,
) -> Option<(String, usize)> {
    let old_chars: Vec<char> = old_text.chars().collect();
    let mut new_chars: Vec<char> = new_text.chars().collect();
    if new_chars.len() != old_chars.len() + 1 || cursor == 0 || cursor > new_chars.len() {
        return None;
    }
    let inserted_at = cursor - 1;
    let inserted_char = new_chars[inserted_at];
    if old_chars[..inserted_at] != new_chars[..inserted_at]
        || old_chars[inserted_at..] != new_chars[cursor..]
    {
        return None;
    }
    if inserted_char == ' ' {
        let mut line_start = inserted_at;
        while line_start > 0 && new_chars[line_start - 1] != '\n' {
            line_start -= 1;
        }
        let mut marker_pos = line_start;
        while marker_pos < inserted_at && matches!(new_chars[marker_pos], ' ' | '\t') {
            marker_pos += 1;
        }
        if inserted_at == marker_pos + 1 && matches!(new_chars[marker_pos], '-' | '*') {
            new_chars[marker_pos] = '•';
            return Some((new_chars.into_iter().collect(), cursor));
        }
        return None;
    }
    if inserted_char == '\n' {
        let mut prev_start = inserted_at;
        while prev_start > 0 && new_chars[prev_start - 1] != '\n' {
            prev_start -= 1;
        }
        let prev_line: String = new_chars[prev_start..inserted_at].iter().collect();
        let parsed = parse_list_prefix(&prev_line)?;
        if parsed.content_empty {
            new_chars.drain(prev_start..cursor);
            return Some((new_chars.into_iter().collect(), prev_start));
        }
        if parsed.kind == ListKind::Bullet {
            let bullet_pos = prev_start + parsed.indent_chars;
            if matches!(new_chars.get(bullet_pos), Some('-' | '*')) {
                new_chars[bullet_pos] = '•';
            }
        }
        let next_prefix = format!("{}{}", parsed.indent, parsed.kind.prefix(parsed.step + 1));
        let next_chars: Vec<char> = next_prefix.chars().collect();
        let next_len = next_chars.len();
        new_chars.splice(cursor..cursor, next_chars);
        return Some((new_chars.into_iter().collect(), cursor + next_len));
    }
    None
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    #[serde(default)]
    pub align: TextAlign,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: 18.0,
            bold: false,
            italic: false,
            underline: false,
            align: TextAlign::Left,
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
        if self.text == new_text && self.styles.len() == self.text.chars().count() {
            return;
        }
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
        let mut line_start = 0;
        for (i, ch) in new.iter().enumerate() {
            if *ch == '\n' {
                let align = self.styles[line_start].align;
                for style in &mut self.styles[line_start..=i] {
                    style.align = align;
                }
                line_start = i + 1;
            }
        }
        if line_start < self.styles.len() {
            let align = self.styles[line_start].align;
            for style in &mut self.styles[line_start..] {
                style.align = align;
            }
        }
        for image in &mut self.images {
            if let Some(style) = self.styles.get(image.anchor) {
                image.align = style.align;
            }
        }
        self.text = new_text;
    }

    pub fn paragraph_range(&self, selection: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let chars: Vec<char> = self.text.chars().collect();
        let len = chars.len();
        let mut start = selection.start.min(len);
        while start > 0 && chars[start - 1] != '\n' {
            start -= 1;
        }
        let mut end = selection.end.min(len);
        if !selection.is_empty() && end > start && chars.get(end - 1) == Some(&'\n') {
            return start..end;
        }
        while end < len && chars[end] != '\n' {
            end += 1;
        }
        if end < len {
            end += 1;
        }
        start..end
    }

    pub fn align_at(&self, cursor: usize, fallback: TextAlign) -> TextAlign {
        if self.styles.is_empty() {
            return fallback;
        }
        let range = self.paragraph_range(cursor..cursor);
        self.styles
            .get(range.start)
            .map_or(fallback, |style| style.align)
    }

    pub fn apply_align(&mut self, selection: std::ops::Range<usize>, align: TextAlign) -> bool {
        self.styles
            .resize(self.text.chars().count(), TextStyle::default());
        let range = self.paragraph_range(selection);
        let mut changed = false;
        for style in &mut self.styles[range.clone()] {
            if style.align != align {
                style.align = align;
                changed = true;
            }
        }
        for image in &mut self.images {
            if range.contains(&image.anchor) && image.align != align {
                image.align = align;
                changed = true;
            }
        }
        changed
    }

    pub fn list_at(&self, cursor: usize) -> Option<ListKind> {
        if self.text.is_empty() {
            return None;
        }
        let chars: Vec<char> = self.text.chars().collect();
        let range = self.paragraph_range(cursor..cursor);
        if range.start >= chars.len() {
            return None;
        }
        let mut end = range.end;
        if end > range.start && chars.get(end - 1) == Some(&'\n') {
            end -= 1;
        }
        let line: String = chars[range.start..end].iter().collect();
        parse_list_prefix(&line).map(|p| p.kind)
    }

    pub fn toggle_list(
        &mut self,
        selection: std::ops::Range<usize>,
        kind: ListKind,
        typing: TextStyle,
    ) -> std::ops::Range<usize> {
        let had_selection = !selection.is_empty();
        let mut chars: Vec<char> = self.text.chars().collect();
        if chars.is_empty() {
            let prefix = kind.prefix(1);
            let count = prefix.chars().count();
            self.reconcile(prefix, typing);
            return count..count;
        }
        let range = self.paragraph_range(selection);
        if range.start >= chars.len() {
            let mut start_step = 1;
            if range.start > 0 {
                let prev_range = self.paragraph_range((range.start - 1)..(range.start - 1));
                let mut prev_end = prev_range.end;
                if prev_end > prev_range.start && chars.get(prev_end - 1) == Some(&'\n') {
                    prev_end -= 1;
                }
                let prev_line: String = chars[prev_range.start..prev_end].iter().collect();
                if let Some(parsed) = parse_list_prefix(&prev_line)
                    && parsed.kind == kind
                {
                    start_step = parsed.step + 1;
                }
            }
            let prefix = kind.prefix(start_step);
            chars.extend(prefix.chars());
            let new_cursor = chars.len();
            self.reconcile(chars.into_iter().collect(), typing);
            return new_cursor..new_cursor;
        }
        let mut line_ranges = Vec::new();
        let mut line_start = range.start;
        for i in range.start..range.end {
            if chars[i] == '\n' {
                if !chars[line_start..i].contains(&IMAGE_MARKER) {
                    line_ranges.push(line_start..i);
                }
                line_start = i + 1;
            }
        }
        if line_start < range.end && !chars[line_start..range.end].contains(&IMAGE_MARKER) {
            line_ranges.push(line_start..range.end);
        } else if line_ranges.is_empty() && range.start == range.end {
            line_ranges.push(range.start..range.end);
        }
        if line_ranges.is_empty() {
            return range.end..range.end;
        }
        let all_match = line_ranges.iter().all(|r| {
            let line: String = chars[r.clone()].iter().collect();
            parse_list_prefix(&line).is_some_and(|p| p.kind == kind)
        });
        let mut start_step = 1;
        if !all_match && line_ranges.len() == 1 && range.start > 0 {
            let prev_range = self.paragraph_range((range.start - 1)..(range.start - 1));
            let mut prev_end = prev_range.end;
            if prev_end > prev_range.start && chars.get(prev_end - 1) == Some(&'\n') {
                prev_end -= 1;
            }
            let prev_line: String = chars[prev_range.start..prev_end].iter().collect();
            if let Some(parsed) = parse_list_prefix(&prev_line)
                && parsed.kind == kind
            {
                start_step = parsed.step + 1;
            }
        }
        let mut delta_total: isize = 0;
        let last_line_end = line_ranges.last().map_or(range.end, |r| r.end);
        for (idx, line_range) in line_ranges.into_iter().enumerate().rev() {
            let line: String = chars[line_range.clone()].iter().collect();
            if all_match {
                if let Some(parsed) = parse_list_prefix(&line) {
                    let remove_start = line_range.start + parsed.indent_chars;
                    let remove_end = line_range.start + parsed.prefix_chars;
                    let removed = remove_end - remove_start;
                    chars.drain(remove_start..remove_end);
                    delta_total -= removed as isize;
                    self.reconcile(chars.iter().collect(), typing);
                }
            } else {
                let replacement = kind.prefix(start_step + idx);
                let rep_chars: Vec<char> = replacement.chars().collect();
                let rep_len = rep_chars.len();
                if let Some(parsed) = parse_list_prefix(&line) {
                    let replace_start = line_range.start + parsed.indent_chars;
                    let replace_end = line_range.start + parsed.prefix_chars;
                    let old_len = replace_end - replace_start;
                    chars.splice(replace_start..replace_end, rep_chars);
                    delta_total += rep_len as isize - old_len as isize;
                } else {
                    let indent_chars = line.chars().take_while(|&c| c == ' ' || c == '\t').count();
                    let insert_at = line_range.start + indent_chars;
                    chars.splice(insert_at..insert_at, rep_chars);
                    delta_total += rep_len as isize;
                }
                self.reconcile(chars.iter().collect(), typing);
            }
        }
        let new_end = (last_line_end as isize + delta_total).max(0) as usize;
        if had_selection {
            range.start.min(new_end)..new_end
        } else {
            new_end..new_end
        }
    }

    pub fn apply_image_align(&mut self, image_id: u64, align: TextAlign, available: f32) -> bool {
        let Some(index) = self.images.iter().position(|image| image.id == image_id) else {
            return false;
        };
        let anchor = self.images[index].anchor;
        let mut changed = self.apply_align(anchor..anchor, align);
        let image = &mut self.images[index];
        if image.align != align {
            image.align = align;
            changed = true;
        }
        if image.layout == ImageLayout::Overlay && available > 1.0 {
            let size = image.display_size(available);
            let target_x = match align {
                TextAlign::Left | TextAlign::Justify => 0.0,
                TextAlign::Center => ((available - size.x) * 0.5).max(0.0),
                TextAlign::Right => (available - size.x).max(0.0),
            };
            if (image.offset[0] - target_x).abs() > 0.5 {
                image.offset[0] = target_x;
                changed = true;
            }
        }
        changed
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
            align: typing.align,
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
                    let encoded = image
                        .data
                        .encoded_cache
                        .get_or_init(|| STANDARD.encode(image.data.bytes.as_slice()));
                    let src = format!("data:{};base64,{encoded}", image.data.mime);
                    let align_style = if image.layout == ImageLayout::Text {
                        match image.align {
                            TextAlign::Center => ";display:block;margin:0 auto",
                            TextAlign::Right => ";display:block;margin-left:auto",
                            TextAlign::Left | TextAlign::Justify => "",
                        }
                    } else {
                        ""
                    };
                    let tag = format!(
                        "<img alt=\"{}\" src=\"{src}\" style=\"width:{}px;max-width:100%;height:auto;vertical-align:top{align_style}\">",
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
                "<span style=\"font-size:{}px;font-weight:{};font-style:{};text-decoration:{}{}\">{}</span>",
                style.size,
                if style.bold { "700" } else { "400" },
                if style.italic { "italic" } else { "normal" },
                if style.underline { "underline" } else { "none" },
                if style.align != TextAlign::Left {
                    format!(";display:block;text-align:{}", style.align.css())
                } else {
                    String::new()
                },
                escape(&run)
            ));
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
    pub const NAVIGATION: [Self; 3] = [Self::Note, Self::Canvas, Self::Checklist];

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

    #[test]
    fn auto_list_numbering_alpha_bullet_and_toolbar_toggle() {
        for (before, after_enter, expected, expected_cursor) in [
            ("1. Satu", "1. Satu\n", "1. Satu\n2. ", 11),
            ("1) Satu", "1) Satu\n", "1) Satu\n2) ", 11),
            ("a. Satu", "a. Satu\n", "a. Satu\nb. ", 11),
            ("a) Satu", "a) Satu\n", "a) Satu\nb) ", 11),
            ("• Poin", "• Poin\n", "• Poin\n• ", 9),
            ("1. Satu\n2. ", "1. Satu\n2. \n", "1. Satu\n", 8),
        ] {
            let cursor = after_enter.chars().count();
            let (rewritten, new_cursor) =
                auto_list_transform(before, after_enter, cursor).expect(before);
            assert_eq!(rewritten, expected);
            assert_eq!(new_cursor, expected_cursor);
        }
        let (bullet, bullet_cursor) = auto_list_transform("-", "- ", 2).unwrap();
        assert_eq!(bullet, "• ");
        assert_eq!(bullet_cursor, 2);

        let mut doc = RichDocument::default();
        doc.reconcile("Pertama\nKedua".into(), TextStyle::default());
        doc.toggle_list(0..13, ListKind::NumberDot, TextStyle::default());
        assert_eq!(doc.text, "1. Pertama\n2. Kedua");
        assert_eq!(doc.list_at(0), Some(ListKind::NumberDot));
        doc.toggle_list(
            0..doc.text.len(),
            ListKind::LowerParen,
            TextStyle::default(),
        );
        assert_eq!(doc.text, "a) Pertama\nb) Kedua");
        doc.toggle_list(
            0..doc.text.len(),
            ListKind::LowerParen,
            TextStyle::default(),
        );
        assert_eq!(doc.text, "Pertama\nKedua");
    }
}
