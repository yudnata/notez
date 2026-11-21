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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImageData {
    #[serde(with = "image_bytes")]
    pub bytes: Arc<Vec<u8>>,
    pub mime: String,
    pub width: u32,
    pub height: u32,
}

impl ImageData {
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

mod image_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::Arc;
    pub fn serialize<S: Serializer>(
        bytes: &Arc<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes.as_slice()))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Arc<Vec<u8>>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Encoded(String),
            Array(Vec<u8>),
        }
        match Data::deserialize(deserializer)? {
            Data::Encoded(text) => {
                if text.len() > 28 * 1024 * 1024 {
                    return Err(serde::de::Error::custom("Gambar terlalu besar"));
                }
                STANDARD
                    .decode(text)
                    .map(Arc::new)
                    .map_err(serde::de::Error::custom)
            }
            Data::Array(bytes) => Ok(Arc::new(bytes)),
        }
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
