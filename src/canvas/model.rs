use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub type Point = [f32; 2];
pub type Color = [u8; 4];

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElementKind {
    #[default]
    Rectangle,
    Ellipse,
    Diamond,
    Sticky,
    Text,
    Pen,
    Marker,
    Line,
    Arrow,
    Image,
}

impl ElementKind {
    pub fn bindable(self) -> bool {
        matches!(
            self,
            Self::Rectangle | Self::Ellipse | Self::Diamond | Self::Sticky | Self::Image
        )
    }
    pub fn has_path(self) -> bool {
        matches!(self, Self::Pen | Self::Marker | Self::Line | Self::Arrow)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArrowRoute {
    #[default]
    Straight,
    Elbow,
    Curve,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Paper {
    Blank,
    #[default]
    Grid,
    Ruled,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Style {
    pub stroke: Color,
    pub fill: Color,
    pub width: f32,
    pub font_size: f32,
    pub rough: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            stroke: [0, 0, 0, 255],
            fill: [255, 255, 255, 0],
            width: 2.0,
            font_size: 24.0,
            rough: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub target: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Point>,
}

#[derive(Clone, Debug)]
pub struct ImageData {
    pub bytes: Arc<Vec<u8>>,
    pub encoded_cache: Arc<std::sync::OnceLock<String>>,
    pub mime: String,
    pub width: u32,
    pub height: u32,
}

impl Serialize for ImageData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use serde::ser::SerializeStruct;
        let encoded = self
            .encoded_cache
            .get_or_init(|| STANDARD.encode(self.bytes.as_slice()));
        let mut state = serializer.serialize_struct("ImageData", 4)?;
        state.serialize_field("bytes", encoded)?;
        state.serialize_field("mime", &self.mime)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("height", &self.height)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ImageData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawBytes {
            Encoded(String),
            Array(Vec<u8>),
        }
        #[derive(Deserialize)]
        struct RawImageData {
            bytes: RawBytes,
            mime: String,
            width: u32,
            height: u32,
        }
        let raw = RawImageData::deserialize(deserializer)?;
        let encoded_cache = Arc::new(std::sync::OnceLock::new());
        let bytes = match raw.bytes {
            RawBytes::Encoded(text) => {
                if text.len() > 28 * 1024 * 1024 {
                    return Err(serde::de::Error::custom("Gambar terlalu besar"));
                }
                let decoded = STANDARD.decode(&text).map_err(serde::de::Error::custom)?;
                let _ = encoded_cache.set(text);
                Arc::new(decoded)
            }
            RawBytes::Array(bytes) => Arc::new(bytes),
        };
        Ok(Self {
            bytes,
            encoded_cache,
            mime: raw.mime,
            width: raw.width,
            height: raw.height,
        })
    }
}

impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.mime == other.mime
            && (Arc::ptr_eq(&self.bytes, &other.bytes) || self.bytes == other.bytes)
    }
}

const MAX_DISPLAY_TEXTURE_SIDE: u32 = 1600;

pub type PendingDecode = Arc<std::sync::Mutex<Option<Result<egui::ColorImage, String>>>>;

pub enum TextureSlot {
    Loading(PendingDecode),
    Ready(egui::TextureHandle),
    Failed(String),
}

impl TextureSlot {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading(_))
    }
}

pub fn paint_image_loading(painter: &egui::Painter, ctx: &egui::Context, rect: egui::Rect) {
    let bg = egui::Color32::from_rgb(245, 244, 240);
    let border = egui::Color32::from_rgb(214, 210, 202);
    let track = egui::Color32::from_rgb(220, 216, 208);
    let accent = egui::Color32::from_rgb(113, 79, 171);
    let muted = egui::Color32::from_rgb(108, 105, 114);
    painter.rect_filled(rect, 6.0, bg);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );
    let show_caption = rect.height() >= 72.0 && rect.width() >= 120.0;
    let radius = (rect.width().min(rect.height()) * 0.16).clamp(8.0, 16.0);
    let center = if show_caption {
        rect.center() - egui::vec2(0.0, 10.0)
    } else {
        rect.center()
    };
    painter.circle_stroke(center, radius, egui::Stroke::new(2.2_f32, track));
    let time = ctx.input(|i| i.time) as f32;
    let start_angle = time * 5.2;
    let sweep = std::f32::consts::TAU * 0.68;
    let steps = 20;
    let points: Vec<egui::Pos2> = (0..=steps)
        .map(|step| {
            let t = step as f32 / steps as f32;
            let angle = start_angle + t * sweep;
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.5_f32, accent),
    ));
    if show_caption {
        painter.text(
            center + egui::vec2(0.0, radius + 14.0),
            egui::Align2::CENTER_TOP,
            "Memuat gambar...",
            egui::FontId::proportional(13.0),
            muted,
        );
    }
}

impl ImageData {
    pub fn cache_key(&self) -> (usize, usize) {
        (self.bytes.as_ptr() as usize, self.bytes.len())
    }

    pub fn decode_color_image(bytes: &[u8]) -> Result<egui::ColorImage, String> {
        let decoded = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
        let display = if decoded.width() > MAX_DISPLAY_TEXTURE_SIDE
            || decoded.height() > MAX_DISPLAY_TEXTURE_SIDE
        {
            decoded.thumbnail(MAX_DISPLAY_TEXTURE_SIDE, MAX_DISPLAY_TEXTURE_SIDE)
        } else {
            decoded
        };
        let rgba = display.to_rgba8();
        Ok(egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ))
    }

    pub fn poll_texture<'a>(
        &self,
        cache: &'a mut std::collections::HashMap<(usize, usize), TextureSlot>,
        ctx: &egui::Context,
        name: &str,
    ) -> &'a TextureSlot {
        let key = self.cache_key();
        let entry = cache.entry(key).or_insert_with(|| {
            let pending: PendingDecode = Arc::new(std::sync::Mutex::new(None));
            let target = pending.clone();
            let bytes = self.bytes.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let decoded = Self::decode_color_image(&bytes);
                if let Ok(mut guard) = target.lock() {
                    *guard = Some(decoded);
                }
                ctx.request_repaint();
            });
            TextureSlot::Loading(pending)
        });
        if let TextureSlot::Loading(pending) = entry {
            let finished = pending.lock().ok().and_then(|mut guard| guard.take());
            match finished {
                Some(Ok(color_image)) => {
                    let texture = ctx.load_texture(
                        name.to_owned(),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    *entry = TextureSlot::Ready(texture);
                }
                Some(Err(err)) => {
                    *entry = TextureSlot::Failed(err);
                }
                None => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(16));
                }
            }
        }
        entry
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 20 * 1024 * 1024 {
            return Err("Gambar maksimal 20 MB.".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let format = image::guess_format(&bytes).map_err(|e| e.to_string())?;
        let mime = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            _ => return Err("Pilih gambar PNG atau JPEG.".into()),
        };
        let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format)
            .into_dimensions()
            .map_err(|e| e.to_string())?;
        let data = Self {
            bytes: Arc::new(bytes),
            encoded_cache: Arc::new(std::sync::OnceLock::new()),
            mime: mime.into(),
            width,
            height,
        };
        data.validate()?;
        Ok(data)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.mime.as_str(), "image/png" | "image/jpeg") {
            return Err("Format gambar tidak didukung.".into());
        }
        if self.bytes.len() > 20 * 1024 * 1024
            || self.width == 0
            || self.height == 0
            || u64::from(self.width) * u64::from(self.height) > 40_000_000
        {
            return Err("Gambar maksimal 20 MB dan 40 megapiksel.".into());
        }
        Ok(())
    }

    pub fn size_at_width(&self, width: f32) -> Point {
        [width, width * self.height as f32 / self.width as f32]
    }

    pub fn size_at_height(&self, height: f32) -> Point {
        self.size_at_width(height * self.width as f32 / self.height as f32)
    }

    pub fn resize_corner(&self, corner: Point) -> Point {
        let ratio = self.height as f32 / self.width as f32;
        let width = ((corner[0] + corner[1] * ratio) / (1.0 + ratio * ratio)).max(30.0);
        self.size_at_width(width)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Element {
    pub id: u64,
    pub kind: ElementKind,
    pub pos: Point,
    pub size: Point,
    pub points: Vec<Point>,
    pub text: String,
    pub style: Style,
    pub route: ArrowRoute,
    pub bend: Option<Point>,
    pub start_binding: Option<Binding>,
    pub end_binding: Option<Binding>,
    pub image: Option<ImageData>,
    pub locked: bool,
}

impl Element {
    pub fn new(id: u64, kind: ElementKind, pos: Point, style: Style) -> Self {
        Self {
            id,
            kind,
            pos,
            size: [160.0, 100.0],
            points: vec![],
            text: String::new(),
            style,
            route: ArrowRoute::Straight,
            bend: None,
            start_binding: None,
            end_binding: None,
            image: None,
            locked: false,
        }
    }

    pub fn center(&self) -> Point {
        [
            self.pos[0] + self.size[0] * 0.5,
            self.pos[1] + self.size[1] * 0.5,
        ]
    }

    pub fn translate(&mut self, delta: Point) {
        self.pos[0] += delta[0];
        self.pos[1] += delta[1];
        for point in &mut self.points {
            point[0] += delta[0];
            point[1] += delta[1];
        }
        if let Some(bend) = &mut self.bend {
            bend[0] += delta[0];
            bend[1] += delta[1];
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CanvasData {
    pub next_id: u64,
    pub elements: Vec<Element>,
    pub paper: Paper,
    pub pan: Point,
    pub zoom: f32,
}

impl Default for CanvasData {
    fn default() -> Self {
        Self {
            next_id: 1,
            elements: vec![],
            paper: Paper::Grid,
            pan: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

impl CanvasData {
    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn element(&self, id: u64) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }
    pub fn element_mut(&mut self, id: u64) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn reroute_connections(&mut self, transformed: &[u64]) -> bool {
        let mut changed = false;
        for arrow in self
            .elements
            .iter_mut()
            .filter(|e| e.kind == ElementKind::Arrow)
        {
            if [&arrow.start_binding, &arrow.end_binding]
                .into_iter()
                .flatten()
                .any(|binding| transformed.contains(&binding.target))
            {
                for binding in [&mut arrow.start_binding, &mut arrow.end_binding]
                    .into_iter()
                    .flatten()
                {
                    changed |= binding.anchor.take().is_some();
                }
            }
        }
        changed
    }

    pub fn remove(&mut self, ids: &[u64]) {
        let ends: Vec<_> = self
            .elements
            .iter()
            .filter(|e| e.kind == ElementKind::Arrow)
            .map(|e| (e.id, super::geometry::endpoints(self, e)))
            .collect();
        for (id, (start, end)) in ends {
            if let Some(arrow) = self.element_mut(id) {
                if arrow
                    .start_binding
                    .as_ref()
                    .is_some_and(|b| ids.contains(&b.target))
                {
                    arrow.start_binding = None;
                    if let Some(point) = arrow.points.first_mut() {
                        *point = start;
                    }
                }
                if arrow
                    .end_binding
                    .as_ref()
                    .is_some_and(|b| ids.contains(&b.target))
                {
                    arrow.end_binding = None;
                    if let Some(point) = arrow.points.last_mut() {
                        *point = end;
                    }
                }
            }
        }
        self.elements.retain(|e| !ids.contains(&e.id));
    }

    pub fn duplicate(&mut self, ids: &[u64], offset: Point) -> Vec<u64> {
        let originals: Vec<_> = self
            .elements
            .iter()
            .filter(|e| ids.contains(&e.id))
            .cloned()
            .collect();
        self.insert_copies(originals, offset)
    }

    pub fn insert_copies(&mut self, originals: Vec<Element>, offset: Point) -> Vec<u64> {
        let mut mapping = std::collections::HashMap::new();
        for element in &originals {
            mapping.insert(element.id, self.id());
        }
        let mut selected = vec![];
        for mut element in originals {
            element.id = mapping[&element.id];
            element.translate(offset);
            for binding in [&mut element.start_binding, &mut element.end_binding]
                .into_iter()
                .flatten()
            {
                if let Some(id) = mapping.get(&binding.target) {
                    binding.target = *id;
                }
            }
            selected.push(element.id);
            self.elements.push(element);
        }
        selected
    }

    pub fn validate(&mut self) -> Result<(), String> {
        if !self.zoom.is_finite() || !self.pan.iter().all(|n| n.is_finite()) {
            return Err("Posisi canvas tidak valid.".into());
        }
        self.zoom = self.zoom.clamp(0.15, 4.0);
        let mut ids = HashSet::new();
        let mut max_id = 0;
        for e in &mut self.elements {
            if !ids.insert(e.id) {
                return Err("ID objek canvas berulang.".into());
            }
            max_id = max_id.max(e.id);
            if !e
                .pos
                .iter()
                .chain(&e.size)
                .chain(e.points.iter().flatten())
                .all(|n| n.is_finite() && n.abs() < 1.0e7)
                || e.bend.is_some_and(|p| !p.iter().all(|n| n.is_finite()))
            {
                return Err("Koordinat objek canvas tidak valid.".into());
            }
            if !e.style.width.is_finite() || !e.style.font_size.is_finite() {
                return Err("Format canvas tidak valid.".into());
            }
            e.style.width = e.style.width.clamp(0.5, 24.0);
            e.style.font_size = e.style.font_size.clamp(12.0, 96.0);
            e.size[0] = e.size[0].max(1.0);
            e.size[1] = e.size[1].max(1.0);
            if e.kind.has_path() && e.points.len() < 2 {
                return Err("Garis canvas tidak lengkap.".into());
            }
            if let Some(image) = &e.image {
                image.validate()?;
                e.size = image.size_at_width(e.size[0]);
            }
        }
        let bindable: HashSet<_> = self
            .elements
            .iter()
            .filter(|e| e.kind.bindable())
            .map(|e| e.id)
            .collect();
        for e in &mut self.elements {
            for binding in [&mut e.start_binding, &mut e.end_binding] {
                if binding
                    .as_ref()
                    .and_then(|b| b.anchor)
                    .is_some_and(|anchor| {
                        anchor
                            .iter()
                            .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
                    })
                {
                    return Err("Titik sambungan canvas tidak valid.".into());
                }
                if binding
                    .as_ref()
                    .is_some_and(|b| !bindable.contains(&b.target) || b.target == e.id)
                {
                    *binding = None;
                }
            }
        }
        self.next_id = self
            .next_id
            .max(max_id.checked_add(1).ok_or("ID canvas terlalu besar.")?);
        if self.next_id == u64::MAX {
            return Err("ID canvas terlalu besar.".into());
        }
        Ok(())
    }
}
