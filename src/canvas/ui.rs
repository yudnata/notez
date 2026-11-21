use super::{geometry as geo, model::*};
use egui::{
    Color32, FontFamily, FontId, Key, PointerButton, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use std::collections::HashMap;

const INK: Color32 = Color32::from_rgb(43, 43, 48);
const PAPER: Color32 = Color32::WHITE;
const FOCUS: Color32 = Color32::from_rgb(113, 79, 171);
const STROKES: [Color; 6] = [
    [0, 0, 0, 255],
    [52, 105, 189, 255],
    [196, 53, 59, 255],
    [100, 65, 160, 255],
    [46, 113, 71, 255],
    [128, 84, 39, 255],
];
const FILLS: [Color; 6] = [
    [255, 255, 255, 0],
    [255, 228, 129, 255],
    [247, 201, 210, 255],
    [195, 222, 246, 255],
    [204, 233, 200, 255],
    [239, 231, 248, 255],
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tool {
    #[default]
    Select,
    Lasso,
    Pen,
    Marker,
    Eraser,
    Sticky,
    Text,
    Rectangle,
    Ellipse,
    Diamond,
    Arrow,
    Line,
    Pan,
}

impl Tool {
    fn kind(self) -> Option<ElementKind> {
        Some(match self {
            Self::Pen => ElementKind::Pen,
            Self::Marker => ElementKind::Marker,
            Self::Sticky => ElementKind::Sticky,
            Self::Text => ElementKind::Text,
            Self::Rectangle => ElementKind::Rectangle,
            Self::Ellipse => ElementKind::Ellipse,
            Self::Diamond => ElementKind::Diamond,
            Self::Arrow => ElementKind::Arrow,
            Self::Line => ElementKind::Line,
            _ => return None,
        })
    }
    fn name(self) -> &'static str {
        match self {
            Self::Select => "Pilih",
            Self::Lasso => "Lasso",
            Self::Pen => "Pena",
            Self::Marker => "Stabilo",
            Self::Eraser => "Penghapus",
            Self::Sticky => "Sticky",
            Self::Text => "Teks",
            Self::Rectangle => "Kotak",
            Self::Ellipse => "Oval",
            Self::Diamond => "Belah ketupat",
            Self::Arrow => "Panah",
            Self::Line => "Garis",
            Self::Pan => "Geser",
        }
    }
}

enum Drag {
    Draw { id: u64, start: Point },
    Move { last: Point },
    Resize { id: u64 },
    Endpoint { id: u64, start: bool },
    Bend { id: u64 },
    Marquee { start: Point, current: Point },
    Lasso { points: Vec<Point> },
    Pan,
    Erase { last: Point, saved: bool },
}

#[derive(Default)]
pub struct CanvasEditor {
    tool: Tool,
    style: Style,
    creation_style: Style,
    eraser_size: Option<f32>,
    route: ArrowRoute,
    selected: Vec<u64>,
    undo: Vec<CanvasData>,
    redo: Vec<CanvasData>,
    clipboard: Vec<Element>,
    drag: Option<Drag>,
    editing: Option<u64>,
    focus_text: bool,
    textures: HashMap<u64, egui::TextureHandle>,
    ruler: bool,
    last_rect: Option<Rect>,
    shape_geometry: Vec<(u64, Point, Point)>,
    pub error: Option<String>,
}

#[derive(Clone, Copy)]
struct View {
    rect: Rect,
    pan: Point,
    zoom: f32,
}

impl View {
    fn screen(self, point: Point) -> Pos2 {
        self.rect.min
            + Vec2::new(
                point[0] * self.zoom + self.pan[0],
                point[1] * self.zoom + self.pan[1],
            )
    }
    fn world(self, point: Pos2) -> Point {
        [
            (point.x - self.rect.min.x - self.pan[0]) / self.zoom,
            (point.y - self.rect.min.y - self.pan[1]) / self.zoom,
        ]
    }
    fn screen_rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(self.screen(rect.min.into()), self.screen(rect.max.into()))
    }
}

fn color(value: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(value[0], value[1], value[2], value[3])
}
fn hand(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("hand".into()))
}

fn shape_geometry(data: &CanvasData) -> Vec<(u64, Point, Point)> {
    data.elements
        .iter()
        .filter(|e| e.kind.bindable())
        .map(|e| (e.id, e.pos, e.size))
        .collect()
}

fn text_content_rect(data: &CanvasData, element: &Element, view: View) -> Rect {
    if element.kind.has_path() {
        let path = if element.kind == ElementKind::Arrow {
            geo::arrow_path(data, element)
        } else {
            element.points.clone()
        };
        Rect::from_center_size(
            view.screen(path[path.len() / 2]),
            Vec2::new(200.0, 85.0) * view.zoom,
        )
    } else {
        let rect = view.screen_rect(geo::bounds(data, element));
        Rect::from_center_size(
            rect.center(),
            Vec2::new(
                (rect.width() - 16.0 * view.zoom).max(15.0),
                (rect.height() - 16.0 * view.zoom).max(element.style.font_size * view.zoom),
            ),
        )
    }
}

impl CanvasEditor {
    fn update_connections(&mut self, data: &mut CanvasData) -> bool {
        let transformed: Vec<_> = self
            .shape_geometry
            .iter()
            .filter_map(|(id, pos, size)| {
                data.element(*id)
                    .filter(|e| e.pos != *pos || e.size != *size)
                    .map(|e| e.id)
            })
            .collect();
        self.shape_geometry = shape_geometry(data);
        data.reroute_connections(&transformed)
    }

    fn choose_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.style = self.creation_style;
        self.selected.clear();
        self.editing = None;
        self.focus_text = false;
        self.drag = None;
    }

    fn eraser_size(&self) -> f32 {
        self.eraser_size.unwrap_or(32.0)
    }

    fn erase(&mut self, data: &mut CanvasData, from: Point, to: Point, saved: &mut bool) -> bool {
        let before = (!*saved).then(|| data.clone());
        if !super::eraser::erase(data, from, to, self.eraser_size() * 0.5 / data.zoom) {
            return false;
        }
        if let Some(before) = before {
            self.checkpoint(&before);
            *saved = true;
        }
        self.selected.retain(|id| data.element(*id).is_some());
        true
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn checkpoint(&mut self, data: &CanvasData) {
        self.undo.push(data.clone());
        if self.undo.len() > 60 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub(crate) fn can_history(&self, redo: bool) -> bool {
        if redo {
            !self.redo.is_empty()
        } else {
            !self.undo.is_empty()
        }
    }

    pub(crate) fn history(&mut self, data: &mut CanvasData, redo: bool) -> bool {
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        if let Some(previous) = stack.pop() {
            let current = std::mem::replace(data, previous);
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.selected.retain(|id| data.element(*id).is_some());
            self.editing = None;
            self.drag = None;
            self.textures.clear();
            self.shape_geometry = shape_geometry(data);
            true
        } else {
            false
        }
    }

    fn delete_selection(&mut self, data: &mut CanvasData) -> bool {
        let ids: Vec<_> = self
            .selected
            .iter()
            .copied()
            .filter(|id| data.element(*id).is_some_and(|e| !e.locked))
            .collect();
        if ids.is_empty() {
            return false;
        }
        self.checkpoint(data);
        data.remove(&ids);
        self.selected.clear();
        self.editing = None;
        true
    }

    fn create(&mut self, data: &mut CanvasData, kind: ElementKind, at: Point) -> u64 {
        let id = data.id();
        let mut element = Element::new(id, kind, at, self.creation_style);
        element.route = self.route;
        if kind == ElementKind::Sticky {
            element.size = [170.0, 150.0];
            if element.style.fill[3] == 0 {
                element.style.fill = FILLS[1];
            }
            element.style.stroke = [160, 128, 46, 130];
            element.style.width = 0.7;
        }
        if kind == ElementKind::Text {
            element.size = [220.0, 80.0];
        }
        if kind.has_path() {
            element.points = vec![at, geo::add(at, [140.0, 0.0])];
        }
        if kind == ElementKind::Marker {
            if element.style.stroke[..3] == Style::default().stroke[..3] {
                element.style.stroke = [238, 192, 30, 75];
            }
            element.style.stroke[3] = 75;
            element.style.width = self.creation_style.width.max(12.0);
        }
        self.style = element.style;
        data.elements.push(element);
        self.selected = vec![id];
        id
    }

    pub fn show(&mut self, ui: &mut egui::Ui, data: &mut CanvasData, title: &mut String) -> bool {
        let mut changed = self.update_connections(data);
        ui.horizontal_top(|ui| {
            let height = ui.available_height();
            let expanded = ui.available_width() >= 840.0;
            let settings_width = if expanded { 264.0 } else { 52.0 };
            let size = Vec2::new(
                (ui.available_width() - 132.0 - settings_width).max(0.0),
                height,
            );
            let center = [
                (size.x * 0.5 - data.pan[0]) / data.zoom,
                (size.y * 0.5 - data.pan[1]) / data.zoom,
            ];
            ui.allocate_ui_with_layout(
                Vec2::new(116.0, height),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    changed |= self.toolbar(ui, data, center);
                },
            );
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            self.last_rect = Some(rect);
            let view = View {
                rect,
                pan: data.pan,
                zoom: data.zoom,
            };
            let painter = ui.painter_at(rect);
            self.paint_paper(&painter, view, data.paper);
            if response.clicked() || response.drag_started() {
                response.request_focus();
            }
            changed |= self.keyboard(ui, data, &response, view);
            changed |= self.interact(ui, data, &response, view);
            changed |= self.update_connections(data);
            let view = View {
                rect,
                pan: data.pan,
                zoom: data.zoom,
            };
            self.paint(&painter, data, view);
            changed |= self.text_editor(ui, data, view);
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("canvas-drawing-bounds"), rect));
            ui.allocate_ui_with_layout(
                Vec2::new(settings_width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    changed |= self.settings_panel(ui, data, title, size, expanded);
                },
            );
        });
        if self.update_connections(data) {
            changed = true;
            ui.ctx().request_repaint();
        }
        changed
    }

    fn settings_panel(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut CanvasData,
        title: &mut String,
        size: Vec2,
        expanded: bool,
    ) -> bool {
        let mut changed = false;
        let frame = egui::Frame::new()
            .fill(Color32::WHITE)
            .corner_radius(10)
            .inner_margin(if expanded { 12.0 } else { 6.0 })
            .show(ui, |ui| {
                if expanded {
                    egui::ScrollArea::vertical()
                        .id_salt("canvas-settings-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            changed |= self.settings_contents(ui, data, title, size);
                        });
                } else {
                    ui.set_min_height(ui.available_height());
                    ui.spacing_mut().button_padding = Vec2::splat(6.0);
                    let response = ui
                        .add(
                            crate::controls::button_with_text(ui, "Pengaturan canvas", "")
                                .min_size(Vec2::splat(40.0)),
                        )
                        .on_hover_text("Pengaturan canvas");
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("canvas-settings-toggle"), response.rect)
                    });
                    egui::Popup::menu(&response)
                        .align(egui::emath::RectAlign::LEFT_START)
                        .width(264.0)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .frame(
                            egui::Frame::popup(ui.style())
                                .fill(Color32::WHITE)
                                .corner_radius(10)
                                .inner_margin(12.0),
                        )
                        .show(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("canvas-settings-popup-scroll")
                                .max_height((ui.ctx().content_rect().height() - 64.0).max(100.0))
                                .show(ui, |ui| {
                                    changed |= self.settings_contents(ui, data, title, size);
                                });
                        });
                }
            });
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new("canvas-settings-bounds"), frame.response.rect)
        });
        changed
    }

    fn settings_contents(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut CanvasData,
        title: &mut String,
        size: Vec2,
    ) -> bool {
        ui.label(RichText::new("Nama canvas").size(12.0).color(crate::MUTED));
        let mut changed = ui
            .add(
                egui::TextEdit::singleline(title)
                    .id(egui::Id::new("canvas-title"))
                    .font(hand(25.0))
                    .hint_text("Tanpa judul")
                    .frame(false)
                    .desired_width(ui.available_width()),
            )
            .changed();
        ui.add_space(4.0);
        changed |= self.actions(ui, data, title, size);
        ui.add_space(8.0);
        ui.separator();
        if self.tool == Tool::Eraser {
            let mut size = self.eraser_size();
            let label = ui
                .horizontal(|ui| {
                    let label = ui.label(RichText::new("Ukuran penghapus").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::DragValue::new(&mut size)
                                .range(8.0..=120.0)
                                .speed(1.0)
                                .max_decimals(0)
                                .suffix(" px"),
                        )
                        .on_hover_text("Klik angkanya untuk mengetik ukuran");
                    });
                    label.id
                })
                .inner;
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.spacing_mut().slider_rail_height = 4.0;
                ui.spacing_mut().interact_size.y = 28.0;
                let visuals = ui.visuals_mut();
                visuals.widgets.inactive.bg_fill = Color32::from_gray(145);
                visuals.widgets.inactive.fg_stroke = Stroke::new(1.5_f32, crate::TEXT);
                visuals.widgets.hovered.bg_fill = crate::TEXT;
                visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, crate::TEXT);
                visuals.widgets.active.bg_fill = crate::TEXT;
                visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, crate::TEXT);
                visuals.selection.bg_fill = crate::TEXT;
                ui.add(
                    egui::Slider::new(&mut size, 8.0..=120.0)
                        .show_value(false)
                        .step_by(1.0)
                        .trailing_fill(true)
                        .handle_shape(egui::style::HandleShape::Circle),
                )
                .labelled_by(label)
                .on_hover_text("Geser untuk mengatur ukuran penghapus");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("8 px").size(11.0).color(crate::MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("120 px").size(11.0).color(crate::MUTED));
                });
            });
            self.eraser_size = Some(size);
            ui.label(RichText::new("Shape: hapus utuh\nPena / stabilo: hapus sebagian").size(12.0));
        } else {
            ui.label(RichText::new("Gaya").strong());
            changed |= self.properties(ui, data);
        }
        changed
    }

    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut CanvasData,
        title: &str,
        fit_size: Vec2,
    ) -> bool {
        let mut changed = false;
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.undo.is_empty(), crate::controls::button(ui, "Undo"))
                    .on_hover_text("Ctrl+Z")
                    .clicked()
                {
                    changed |= self.history(data, false);
                }
                if ui
                    .add_enabled(!self.redo.is_empty(), crate::controls::button(ui, "Redo"))
                    .on_hover_text("Ctrl+Y")
                    .clicked()
                {
                    changed |= self.history(data, true);
                }
            });
            let paper = data.paper;
            egui::ComboBox::from_id_salt("paper")
                .selected_text(match data.paper {
                    Paper::Blank => "Kertas polos",
                    Paper::Grid => "Kertas kotak",
                    Paper::Ruled => "Kertas bergaris",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut data.paper, Paper::Blank, "Polos");
                    ui.selectable_value(&mut data.paper, Paper::Grid, "Kotak");
                    ui.selectable_value(&mut data.paper, Paper::Ruled, "Bergaris");
                });
            changed |= paper != data.paper;
            if ui
                .add(crate::controls::button(ui, "Gambar..."))
                .on_hover_text("Sisipkan PNG atau JPEG; bisa juga seret file ke canvas")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Gambar", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    changed |= self.import_image(
                        data,
                        &path,
                        [
                            80.0 - data.pan[0] / data.zoom,
                            80.0 - data.pan[1] / data.zoom,
                        ],
                    );
                }
            }
            crate::controls::menu(ui, "Template", |ui| {
                for (name, template) in [("Alur proses", 0), ("Mind map", 1), ("Brainstorm", 2)] {
                    if ui.button(name).clicked() {
                        self.template(data, template);
                        changed = true;
                        ui.close();
                    }
                }
            });
            crate::controls::menu(ui, "Ekspor canvas", |ui| {
                for (label, png) in [("PNG (gambar)", true), ("SVG (vektor + font)", false)] {
                    if ui.button(label).clicked() {
                        ui.close();
                        let extension = if png { "png" } else { "svg" };
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter(extension, &[extension])
                            .set_file_name(format!("canvas.{extension}"))
                            .save_file()
                        {
                            let bytes = if png {
                                data.png(title)
                            } else {
                                Ok(data.svg(title, true).into_bytes())
                            };
                            if let Err(error) = bytes.and_then(|bytes| {
                                std::fs::write(path, bytes).map_err(|e| e.to_string())
                            }) {
                                self.error = Some(error);
                            }
                        }
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(crate::controls::button(ui, "Fit"))
                    .on_hover_text("Ctrl+Shift+F")
                    .clicked()
                {
                    self.fit(data, fit_size);
                    changed = true;
                }
                if ui.small_button("−").on_hover_text("Ctrl+-").clicked() {
                    data.zoom = (data.zoom / 1.2).max(0.15);
                    changed = true;
                }
                if ui
                    .button(format!("{:.0}%", data.zoom * 100.0))
                    .on_hover_text("Ctrl+0: reset zoom")
                    .clicked()
                {
                    data.zoom = 1.0;
                    data.pan = [0.0, 0.0];
                    changed = true;
                }
                if ui.small_button("+").on_hover_text("Ctrl++").clicked() {
                    data.zoom = (data.zoom * 1.2).min(4.0);
                    changed = true;
                }
            });
        });
        changed
    }

    fn tool_buttons(&mut self, ui: &mut egui::Ui) {
        use super::icons::{self, Icon};
        for (index, (tool, icon, hint)) in [
            (Tool::Select, Icon::Select, "V"),
            (Tool::Lasso, Icon::Lasso, "L"),
            (Tool::Pen, Icon::Pen, "P"),
            (Tool::Marker, Icon::Marker, "H"),
            (Tool::Eraser, Icon::Eraser, "E"),
            (Tool::Sticky, Icon::Sticky, "N"),
            (Tool::Text, Icon::Text, "T"),
            (Tool::Rectangle, Icon::Rectangle, "S"),
            (Tool::Ellipse, Icon::Ellipse, ""),
            (Tool::Diamond, Icon::Diamond, ""),
            (Tool::Arrow, Icon::Arrow, "A"),
            (Tool::Line, Icon::Line, ""),
            (Tool::Pan, Icon::Pan, "Space + drag"),
        ]
        .into_iter()
        .enumerate()
        {
            if icons::button(ui, icon, tool.name(), hint, self.tool == tool).clicked() {
                self.choose_tool(tool);
            }
            if index % 2 == 1 {
                ui.end_row();
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, data: &mut CanvasData, center: Point) -> bool {
        use super::icons::{self, Icon};
        let mut changed = false;
        let frame = egui::Frame::new()
            .fill(Color32::WHITE)
            .corner_radius(8)
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::ScrollArea::vertical()
                    .id_salt("canvas-tools-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("canvas-tool-grid")
                            .num_columns(2)
                            .spacing(Vec2::splat(6.0))
                            .show(ui, |ui| {
                                self.tool_buttons(ui);
                                if icons::button(ui, Icon::Ruler, "Penggaris", "Ctrl+R", self.ruler)
                                    .clicked()
                                {
                                    self.ruler = !self.ruler;
                                }
                                ui.end_row();
                                if icons::button(ui, Icon::Add, "Tambah di tengah", "", false)
                                    .clicked()
                                {
                                    self.checkpoint(data);
                                    let kind = self.tool.kind().unwrap_or(ElementKind::Rectangle);
                                    let id = self.create(data, kind, center);
                                    if matches!(kind, ElementKind::Text | ElementKind::Sticky) {
                                        self.editing = Some(id);
                                        self.focus_text = true;
                                    }
                                    changed = true;
                                }
                                ui.end_row();
                            });
                    });
            });
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new("canvas-toolbar-bounds"), frame.response.rect)
        });
        changed
    }

    fn properties(&mut self, ui: &mut egui::Ui, data: &mut CanvasData) -> bool {
        if self.selected.is_empty() {
            self.style = self.creation_style;
        }
        let arrow_context = if self.selected.is_empty() {
            self.tool == Tool::Arrow
        } else {
            self.selected.iter().all(|id| {
                data.element(*id)
                    .is_some_and(|e| e.kind == ElementKind::Arrow)
            })
        };
        if let [id] = self.selected.as_slice() {
            if let Some(arrow) = data.element(*id).filter(|e| e.kind == ElementKind::Arrow) {
                self.route = arrow.route;
            }
        }
        let before = data.clone();
        let previous = self.style;
        let old_route = self.route;
        let mut changed = false;
        ui.vertical(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            for visual in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                visual.corner_radius = egui::CornerRadius::same(4);
            }
            ui.label(RichText::new("Tinta").size(12.0));
            ui.horizontal(|ui| {
                ui.spacing_mut().button_padding = Vec2::ZERO;
                ui.spacing_mut().interact_size.y = 28.0;
                ui.spacing_mut().item_spacing.x = 6.0;
                for (i, c) in STROKES.into_iter().enumerate() {
                    let response = ui.add_sized(
                        [28.0, 28.0],
                        egui::Button::new(
                            RichText::new(if self.style.stroke[..3] == c[..3] {
                                "•"
                            } else {
                                " "
                            })
                            .color(Color32::WHITE),
                        )
                        .fill(color(c))
                        .corner_radius(4.0),
                    );
                    if response
                        .on_hover_text(format!("Warna tinta {}", i + 1))
                        .clicked()
                    {
                        self.style.stroke = c;
                    }
                }
            });
            ui.label(RichText::new("Isi").size(12.0));
            ui.horizontal(|ui| {
                ui.spacing_mut().button_padding = Vec2::ZERO;
                ui.spacing_mut().interact_size.y = 28.0;
                ui.spacing_mut().item_spacing.x = 6.0;
                for (i, c) in FILLS.into_iter().enumerate() {
                    let response = ui.add_sized(
                        [28.0, 28.0],
                        egui::Button::new(if c[3] == 0 {
                            "∅"
                        } else if self.style.fill == c {
                            "•"
                        } else {
                            " "
                        })
                        .fill(if c[3] == 0 { Color32::WHITE } else { color(c) })
                        .corner_radius(4.0),
                    );
                    if response
                        .on_hover_text(if i == 0 {
                            "Tanpa isi".into()
                        } else {
                            format!("Warna isi {i}")
                        })
                        .clicked()
                    {
                        self.style.fill = c;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Tebal");
                ui.add(
                    egui::DragValue::new(&mut self.style.width)
                        .range(0.5..=16.0)
                        .speed(0.2),
                );
                ui.label("Font");
                ui.add(
                    egui::DragValue::new(&mut self.style.font_size)
                        .range(12.0..=72.0)
                        .speed(1.0),
                );
            });
            ui.checkbox(&mut self.style.rough, "Sketsa");
            if arrow_context {
                egui::ComboBox::from_id_salt("arrow-route")
                    .selected_text(match self.route {
                        ArrowRoute::Straight => "Panah lurus",
                        ArrowRoute::Elbow => "Panah siku",
                        ArrowRoute::Curve => "Panah lengkung",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.route, ArrowRoute::Straight, "Lurus");
                        ui.selectable_value(&mut self.route, ArrowRoute::Elbow, "Siku");
                        ui.selectable_value(&mut self.route, ArrowRoute::Curve, "Lengkung");
                    });
            }
        });
        if previous != self.style || old_route != self.route {
            if self.selected.is_empty() {
                self.creation_style = self.style;
            }
            for element in data
                .elements
                .iter_mut()
                .filter(|e| self.selected.contains(&e.id) && !e.locked)
            {
                if previous.stroke != self.style.stroke {
                    element.style.stroke = self.style.stroke;
                    if element.kind == ElementKind::Marker {
                        element.style.stroke[3] = 75;
                    }
                }
                if previous.fill != self.style.fill {
                    element.style.fill = self.style.fill;
                }
                if previous.width != self.style.width {
                    element.style.width = self.style.width;
                }
                if previous.font_size != self.style.font_size {
                    element.style.font_size = self.style.font_size;
                }
                if previous.rough != self.style.rough {
                    element.style.rough = self.style.rough;
                }
                if old_route != self.route && element.kind == ElementKind::Arrow {
                    element.route = self.route;
                    element.bend = None;
                }
                changed = true;
            }
        }
        if self.selected.len() == 1 && data.element(self.selected[0]).is_some() {
            let id = self.selected[0];
            if let Some(element) = data.element_mut(id) {
                ui.horizontal_wrapped(|ui| {
                    let old = element.pos;
                    ui.label("X");
                    changed |= ui
                        .add(egui::DragValue::new(&mut element.pos[0]).speed(1.0))
                        .changed();
                    ui.label("Y");
                    changed |= ui
                        .add(egui::DragValue::new(&mut element.pos[1]).speed(1.0))
                        .changed();
                    let delta = geo::sub(element.pos, old);
                    element.pos = old;
                    element.translate(delta);
                    if !element.kind.has_path() {
                        ui.label("W");
                        let width_changed = ui
                            .add(egui::DragValue::new(&mut element.size[0]).range(20.0..=4000.0))
                            .changed();
                        if width_changed {
                            if let Some(image) = &element.image {
                                element.size = image.size_at_width(element.size[0]);
                            }
                            changed = true;
                        }
                        ui.label("H");
                        let height_changed = ui
                            .add(egui::DragValue::new(&mut element.size[1]).range(20.0..=4000.0))
                            .changed();
                        if height_changed {
                            if let Some(image) = &element.image {
                                element.size = image.size_at_height(element.size[1]);
                            }
                            changed = true;
                        }
                    }
                    changed |= ui.checkbox(&mut element.locked, "Kunci").changed();
                });
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(crate::controls::button(ui, "Edit teks"))
                    .on_hover_text("F2")
                    .clicked()
                {
                    self.checkpoint(data);
                    self.editing = Some(id);
                    self.focus_text = true;
                }
                if ui.add(crate::controls::button(ui, "Duplikat")).clicked() {
                    self.checkpoint(data);
                    self.selected = data.duplicate(&self.selected, [24.0, 24.0]);
                    changed = true;
                }
                if ui.add(crate::controls::button(ui, "Ke depan")).clicked() {
                    self.reorder(data, true);
                    changed = true;
                }
                if ui.add(crate::controls::button(ui, "Ke belakang")).clicked() {
                    self.reorder(data, false);
                    changed = true;
                }
                if ui.add(crate::controls::button(ui, "Hapus objek")).clicked() {
                    changed |= self.delete_selection(data);
                }
            });
        }
        if changed && self.undo.last() != Some(&before) {
            self.undo.push(before);
            self.redo.clear();
        }
        changed
    }

    fn import_image(&mut self, data: &mut CanvasData, path: &std::path::Path, at: Point) -> bool {
        let result = ImageData::from_path(path);
        match result {
            Ok(image) => {
                self.checkpoint(data);
                let id = self.create(data, ElementKind::Image, at);
                let scale = (500.0 / image.width.max(image.height) as f32).min(1.0);
                let element = data.element_mut(id).unwrap();
                element.size = [image.width as f32 * scale, image.height as f32 * scale];
                element.image = Some(image);
                self.tool = Tool::Select;
                true
            }
            Err(error) => {
                self.error = Some(format!("Gagal memasukkan gambar: {error}"));
                false
            }
        }
    }

    pub(crate) fn fit(&mut self, data: &mut CanvasData, size: Vec2) {
        let bounds = data.content_bounds();
        data.zoom = ((size.x.max(300.0) - 40.0) / bounds.width())
            .min((size.y.max(300.0) - 100.0) / bounds.height())
            .clamp(0.15, 2.0);
        data.pan = [
            -bounds.min.x * data.zoom + 20.0,
            -bounds.min.y * data.zoom + 20.0,
        ];
    }

    fn reorder(&mut self, data: &mut CanvasData, front: bool) {
        self.checkpoint(data);
        let mut selected = vec![];
        let mut rest = vec![];
        for element in std::mem::take(&mut data.elements) {
            if self.selected.contains(&element.id) {
                selected.push(element);
            } else {
                rest.push(element);
            }
        }
        data.elements = if front {
            rest.extend(selected);
            rest
        } else {
            selected.extend(rest);
            selected
        };
    }

    fn template(&mut self, data: &mut CanvasData, variant: usize) {
        self.checkpoint(data);
        let base = [
            -data.pan[0] / data.zoom + 70.0,
            -data.pan[1] / data.zoom + 70.0,
        ];
        let mut ids = vec![];
        let labels = if variant == 0 {
            vec!["Mulai", "Kerjakan", "Selesai"]
        } else if variant == 1 {
            vec!["Ide utama", "Cabang pertama", "Cabang kedua"]
        } else {
            vec![
                "Ide pertama",
                "Ide kedua",
                "Pertanyaan",
                "Langkah berikutnya",
            ]
        };
        for (i, label) in labels.iter().enumerate() {
            let pos = if variant == 2 {
                geo::add(base, [(i % 2) as f32 * 230.0, (i / 2) as f32 * 190.0])
            } else if variant == 1 {
                geo::add(
                    base,
                    [
                        if i == 0 { 0.0 } else { 290.0 },
                        if i == 0 {
                            100.0
                        } else {
                            (i - 1) as f32 * 210.0
                        },
                    ],
                )
            } else {
                geo::add(base, [i as f32 * 270.0, 80.0])
            };
            let id = self.create(
                data,
                if variant == 2 {
                    ElementKind::Sticky
                } else {
                    ElementKind::Rectangle
                },
                pos,
            );
            let element = data.element_mut(id).unwrap();
            element.text = (*label).into();
            element.style.fill = FILLS[i % 5 + 1];
            ids.push(id);
        }
        if variant != 2 {
            for target in 1..ids.len() {
                let source = if variant == 1 { 0 } else { target - 1 };
                let a = data.element(ids[source]).unwrap().center();
                let b = data.element(ids[target]).unwrap().center();
                let id = self.create(data, ElementKind::Arrow, a);
                let arrow = data.element_mut(id).unwrap();
                arrow.points = vec![a, b];
                arrow.start_binding = Some(Binding {
                    target: ids[source],
                    anchor: None,
                });
                arrow.end_binding = Some(Binding {
                    target: ids[target],
                    anchor: None,
                });
            }
        }
        self.selected = ids;
        self.tool = Tool::Select;
    }

    fn keyboard(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut CanvasData,
        response: &egui::Response,
        view: View,
    ) -> bool {
        if !response.has_focus() || self.editing.is_some() {
            return false;
        }
        let command = egui::Modifiers::COMMAND;
        let shifted = command | egui::Modifiers::SHIFT;
        let mut changed = false;
        let key =
            |ui: &mut egui::Ui, modifiers, key| ui.input_mut(|i| i.consume_key(modifiers, key));
        if key(ui, shifted, Key::Z) || key(ui, command, Key::Y) {
            changed |= self.history(data, true);
        } else if key(ui, command, Key::Z) {
            changed |= self.history(data, false);
        }
        if key(ui, command, Key::A) {
            self.selected = data.elements.iter().map(|e| e.id).collect();
        }
        if key(ui, egui::Modifiers::NONE, Key::Delete)
            || key(ui, egui::Modifiers::NONE, Key::Backspace)
        {
            changed |= self.delete_selection(data);
        }
        let cut = key(ui, command, Key::X);
        if cut || key(ui, command, Key::C) {
            self.clipboard = data
                .elements
                .iter()
                .filter(|e| self.selected.contains(&e.id))
                .cloned()
                .collect();
            if let Ok(json) = serde_json::to_string(&self.clipboard) {
                ui.ctx().copy_text(format!("notvas-canvas-v1:{json}"));
            }
            if cut {
                changed |= self.delete_selection(data);
            }
        }
        let paste = ui.input_mut(|i| {
            let mut result = None;
            i.events.retain(|event| {
                if let egui::Event::Paste(text) = event {
                    if let Some(json) = text.strip_prefix("notvas-canvas-v1:") {
                        result = Some(json.to_string());
                        return false;
                    }
                }
                true
            });
            result
        });
        if let Some(json) = paste {
            match serde_json::from_str::<Vec<Element>>(&json) {
                Ok(elements) => {
                    let mut test = CanvasData {
                        elements: elements.clone(),
                        ..Default::default()
                    };
                    if test.validate().is_ok() {
                        self.checkpoint(data);
                        self.selected = data.insert_copies(elements, [30.0, 30.0]);
                        changed = true;
                    }
                }
                Err(_) => self.error = Some("Data clipboard canvas tidak valid.".into()),
            }
        } else if key(ui, command, Key::V) && !self.clipboard.is_empty() {
            self.checkpoint(data);
            self.selected = data.insert_copies(self.clipboard.clone(), [30.0, 30.0]);
            changed = true;
        }
        if key(ui, command, Key::D) && !self.selected.is_empty() {
            self.checkpoint(data);
            self.selected = data.duplicate(&self.selected, [24.0, 24.0]);
            changed = true;
        }
        if key(ui, shifted, Key::CloseBracket) {
            self.reorder(data, true);
            changed = true;
        }
        if key(ui, shifted, Key::OpenBracket) {
            self.reorder(data, false);
            changed = true;
        }
        if key(ui, command, Key::R) {
            self.ruler = !self.ruler;
        }
        if key(ui, shifted, Key::F) {
            self.fit(data, view.rect.size());
            changed = true;
        }
        if key(ui, command, Key::Num0) {
            data.zoom = 1.0;
            data.pan = [0.0, 0.0];
            changed = true;
        }
        if key(ui, command, Key::Plus) || key(ui, command, Key::Equals) {
            data.zoom = (data.zoom * 1.2).min(4.0);
            changed = true;
        }
        if key(ui, command, Key::Minus) {
            data.zoom = (data.zoom / 1.2).max(0.15);
            changed = true;
        }
        if key(ui, egui::Modifiers::NONE, Key::F2) && self.selected.len() == 1 {
            self.checkpoint(data);
            self.editing = self.selected.first().copied();
            self.focus_text = true;
        }
        if key(ui, egui::Modifiers::NONE, Key::Escape) {
            self.selected.clear();
            self.drag = None;
            self.tool = Tool::Select;
        }
        for (key_code, tool) in [
            (Key::V, Tool::Select),
            (Key::L, Tool::Lasso),
            (Key::P, Tool::Pen),
            (Key::H, Tool::Marker),
            (Key::E, Tool::Eraser),
            (Key::N, Tool::Sticky),
            (Key::T, Tool::Text),
            (Key::S, Tool::Rectangle),
            (Key::A, Tool::Arrow),
        ] {
            if key(ui, egui::Modifiers::NONE, key_code) {
                self.choose_tool(tool);
            }
        }
        let mut delta = [0.0, 0.0];
        let step = if ui.input(|i| i.modifiers.shift) {
            10.0
        } else {
            1.0
        };
        for (key_code, direction) in [
            (Key::ArrowLeft, [-step, 0.0]),
            (Key::ArrowRight, [step, 0.0]),
            (Key::ArrowUp, [0.0, -step]),
            (Key::ArrowDown, [0.0, step]),
        ] {
            if ui.input(|i| i.key_pressed(key_code)) {
                delta = geo::add(delta, direction);
            }
        }
        if delta != [0.0, 0.0] && !self.selected.is_empty() {
            self.checkpoint(data);
            for element in data
                .elements
                .iter_mut()
                .filter(|e| self.selected.contains(&e.id) && !e.locked)
            {
                element.translate(delta);
            }
            changed = true;
        }
        changed
    }

    fn interact(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut CanvasData,
        response: &egui::Response,
        view: View,
    ) -> bool {
        let mut changed = false;
        let touch = ui.input(|i| i.multi_touch()).filter(|t| {
            view.rect.contains(t.start_pos)
                && ui.ctx().layer_id_at(t.start_pos) == Some(ui.layer_id())
        });
        if let Some(touch) = touch {
            let previous = (data.pan, data.zoom);
            let world = view.world(touch.center_pos - touch.translation_delta);
            data.zoom = (data.zoom * touch.zoom_delta).clamp(0.15, 4.0);
            data.pan = [
                touch.center_pos.x - view.rect.min.x - world[0] * data.zoom,
                touch.center_pos.y - view.rect.min.y - world[1] * data.zoom,
            ];
            self.drag = None;
            return previous != (data.pan, data.zoom);
        }
        let pan = crate::navigation::right_drag(ui, response.id, view.rect);
        if pan != Vec2::ZERO {
            data.pan[0] += pan.x;
            data.pan[1] += pan.y;
            changed = true;
        }
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let hovered = response.contains_pointer();
        let alt = ui.input(|i| i.modifiers.alt);
        let shift = ui.input(|i| i.modifiers.shift);
        let space = ui.input(|i| i.key_down(Key::Space));
        if hovered {
            let (scroll, control, pinch) = ui.input(|i| {
                (
                    i.smooth_scroll_delta,
                    i.modifiers.command || i.modifiers.ctrl,
                    i.zoom_delta(),
                )
            });
            if control || (pinch - 1.0).abs() > 0.001 {
                if let Some(pointer) = pointer {
                    let world = view.world(pointer);
                    let factor = pinch;
                    if (factor - 1.0).abs() > 0.0001 {
                        data.zoom = (data.zoom * factor).clamp(0.15, 4.0);
                        data.pan = [
                            pointer.x - view.rect.min.x - world[0] * data.zoom,
                            pointer.y - view.rect.min.y - world[1] * data.zoom,
                        ];
                        changed = true;
                    }
                }
            } else if scroll != Vec2::ZERO {
                data.pan[0] += scroll.x;
                data.pan[1] += scroll.y;
                changed = true;
            }
            let files = ui.input(|i| i.raw.dropped_files.clone());
            for file in files {
                if let Some(path) = file.path {
                    changed |= self.import_image(
                        data,
                        &path,
                        view.world(pointer.unwrap_or(view.rect.center())),
                    );
                }
            }
        }
        let Some(pointer) = pointer else {
            return changed;
        };
        let world = View {
            pan: data.pan,
            zoom: data.zoom,
            ..view
        }
        .world(pointer);
        if response.drag_started_by(PointerButton::Middle)
            || (response.drag_started_by(PointerButton::Primary)
                && (space || self.tool == Tool::Pan))
        {
            self.drag = Some(Drag::Pan);
        } else if response.drag_started_by(PointerButton::Primary) && self.editing.is_none() {
            let press = ui
                .input(|i| i.pointer.press_origin())
                .map(|p| view.world(p))
                .unwrap_or(world);
            match self.tool {
                Tool::Select => {
                    if let Some(drag) = self.handle_at(data, press, 10.0 / data.zoom) {
                        self.checkpoint(data);
                        self.drag = Some(drag);
                    } else if let Some(id) = geo::hit(data, press, 6.0 / data.zoom) {
                        if !self.selected.contains(&id) {
                            self.selected = vec![id];
                        }
                        if !data.element(id).unwrap().locked {
                            self.checkpoint(data);
                            self.drag = Some(Drag::Move { last: press });
                        }
                    } else {
                        if !shift {
                            self.selected.clear();
                        }
                        self.drag = Some(Drag::Marquee {
                            start: press,
                            current: world,
                        });
                    }
                }
                Tool::Lasso => {
                    if !shift {
                        self.selected.clear();
                    }
                    self.drag = Some(Drag::Lasso {
                        points: vec![press, world],
                    });
                }
                Tool::Eraser => {
                    self.drag = Some(Drag::Erase {
                        last: press,
                        saved: false,
                    });
                }
                Tool::Pan => self.drag = Some(Drag::Pan),
                tool => {
                    if let Some(kind) = tool.kind() {
                        self.checkpoint(data);
                        let id = self.create(data, kind, press);
                        let binding = if kind == ElementKind::Arrow && !alt {
                            geo::suggest_binding(data, press, Some(id), 22.0 / data.zoom)
                                .map(|b| b.0)
                        } else {
                            None
                        };
                        let element = data.element_mut(id).unwrap();
                        if kind.has_path() {
                            element.points = vec![press, world];
                            element.start_binding = binding;
                        } else {
                            element.size = [20.0, 20.0];
                        }
                        self.drag = Some(Drag::Draw { id, start: press });
                        changed = true;
                    }
                }
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            self.editing = None;
            self.focus_text = false;
            match self.tool {
                Tool::Select | Tool::Lasso => {
                    if let Some(id) = geo::hit(data, world, 6.0 / data.zoom) {
                        if shift {
                            if self.selected.contains(&id) {
                                self.selected.retain(|n| *n != id);
                            } else {
                                self.selected.push(id);
                            }
                        } else {
                            self.selected = vec![id];
                        }
                        if let Some(element) = data.element(id) {
                            self.style = element.style;
                            self.route = element.route;
                        }
                    } else {
                        self.selected.clear();
                    }
                }
                Tool::Eraser => {
                    changed |= self.erase(data, world, world, &mut false);
                }
                Tool::Pan => {}
                tool => {
                    if tool == Tool::Text {
                        if let Some(id) = geo::hit(data, world, 3.0 / data.zoom) {
                            self.checkpoint(data);
                            self.selected = vec![id];
                            self.editing = Some(id);
                            self.focus_text = true;
                            self.tool = Tool::Select;
                            return changed;
                        }
                    }
                    if let Some(kind) = tool.kind() {
                        self.checkpoint(data);
                        let id = self.create(data, kind, world);
                        if kind == ElementKind::Arrow && !alt {
                            let binding =
                                geo::suggest_binding(data, world, Some(id), 22.0 / data.zoom)
                                    .map(|b| b.0);
                            data.element_mut(id).unwrap().start_binding = binding;
                        }
                        if matches!(kind, ElementKind::Pen | ElementKind::Marker) {
                            data.element_mut(id).unwrap().points =
                                vec![world, geo::add(world, [0.1, 0.1])];
                            self.selected.clear();
                        }
                        changed = true;
                        if kind == ElementKind::Text || kind == ElementKind::Sticky {
                            self.editing = Some(id);
                            self.focus_text = true;
                            self.tool = Tool::Select;
                        } else if !matches!(kind, ElementKind::Pen | ElementKind::Marker) {
                            self.tool = Tool::Select;
                        }
                    }
                }
            }
        }
        if response.double_clicked() && matches!(self.tool, Tool::Select | Tool::Lasso) {
            if let Some(id) = geo::hit(data, world, 6.0 / data.zoom) {
                if data.element(id).is_some_and(|e| {
                    !e.locked
                        && !matches!(
                            e.kind,
                            ElementKind::Pen | ElementKind::Marker | ElementKind::Image
                        )
                }) {
                    self.checkpoint(data);
                    self.selected = vec![id];
                    self.editing = Some(id);
                    self.focus_text = true;
                }
            }
        }
        let mut drag = self.drag.take();
        if let Some(active) = &mut drag {
            let down = ui.input(|i| i.pointer.primary_down() || i.pointer.middle_down());
            let delta = ui.input(|i| i.pointer.delta());
            if down {
                match active {
                    Drag::Pan => {
                        data.pan[0] += delta.x;
                        data.pan[1] += delta.y;
                        changed |= delta != Vec2::ZERO;
                    }
                    Drag::Move { last } => {
                        let delta = geo::sub(world, *last);
                        for e in data
                            .elements
                            .iter_mut()
                            .filter(|e| self.selected.contains(&e.id) && !e.locked)
                        {
                            e.translate(delta);
                        }
                        *last = world;
                        changed |= delta != [0.0, 0.0];
                    }
                    Drag::Resize { id } => {
                        if let Some(e) = data.element_mut(*id) {
                            let old = e.size;
                            e.size = [
                                (world[0] - e.pos[0]).max(30.0),
                                (world[1] - e.pos[1]).max(30.0),
                            ];
                            if let Some(image) = &e.image {
                                e.size = image.resize_corner(e.size);
                            } else if shift {
                                let side = e.size[0].max(e.size[1]);
                                e.size = [side, side];
                            }
                            changed |= old != e.size;
                        }
                    }
                    Drag::Bend { id } => {
                        if let Some(e) = data.element_mut(*id) {
                            changed |= e.bend != Some(world);
                            e.bend = Some(world);
                        }
                    }
                    Drag::Endpoint { id, start } => {
                        let binding = if alt {
                            None
                        } else {
                            geo::suggest_binding(data, world, Some(*id), 22.0 / data.zoom)
                                .map(|b| b.0)
                        };
                        if let Some(e) = data.element_mut(*id) {
                            if *start {
                                e.points[0] = world;
                                e.start_binding = binding;
                            } else {
                                let last = e.points.len() - 1;
                                e.points[last] = world;
                                e.end_binding = binding;
                            }
                            changed |= delta != Vec2::ZERO;
                        }
                    }
                    Drag::Draw { id, start } => {
                        if ui.input(|i| i.pointer.secondary_down() || i.pointer.middle_down()) {
                            data.pan[0] += delta.x;
                            data.pan[1] += delta.y;
                            changed |= delta != Vec2::ZERO;
                        } else {
                            let kind = data.element(*id).map(|e| e.kind).unwrap_or_default();
                            let binding = if kind == ElementKind::Arrow && !alt {
                                geo::suggest_binding(data, world, Some(*id), 22.0 / data.zoom)
                                    .map(|b| b.0)
                            } else {
                                None
                            };
                            if let Some(e) = data.element_mut(*id) {
                                if matches!(kind, ElementKind::Pen | ElementKind::Marker)
                                    && !self.ruler
                                {
                                    if e.points
                                        .last()
                                        .is_none_or(|p| geo::distance(*p, world) > 1.2 / view.zoom)
                                    {
                                        e.points.push(world);
                                        changed = true;
                                    }
                                } else if kind.has_path() {
                                    let mut end = world;
                                    if shift {
                                        let d = geo::sub(world, *start);
                                        let angle = d[1].atan2(d[0]);
                                        let snap = (angle / (std::f32::consts::PI / 4.0)).round()
                                            * (std::f32::consts::PI / 4.0);
                                        let length = geo::distance(*start, world);
                                        end = [
                                            start[0] + snap.cos() * length,
                                            start[1] + snap.sin() * length,
                                        ];
                                    }
                                    changed |= e.points.last() != Some(&end);
                                    e.points = vec![*start, end];
                                    e.end_binding = binding;
                                } else {
                                    let old = (e.pos, e.size);
                                    e.pos = [start[0].min(world[0]), start[1].min(world[1])];
                                    e.size = [
                                        (world[0] - start[0]).abs().max(20.0),
                                        (world[1] - start[1]).abs().max(20.0),
                                    ];
                                    if shift {
                                        let side = e.size[0].max(e.size[1]);
                                        e.size = [side, side];
                                    }
                                    changed |= old != (e.pos, e.size);
                                }
                            }
                        }
                    }
                    Drag::Marquee { current, .. } => *current = world,
                    Drag::Lasso { points } => {
                        if points
                            .last()
                            .is_none_or(|p| geo::distance(*p, world) > 3.0 / data.zoom)
                        {
                            points.push(world);
                        }
                    }
                    Drag::Erase { last, saved } => {
                        changed |= self.erase(data, *last, world, saved);
                        *last = world;
                    }
                }
                if !matches!(active, Drag::Pan) && !view.rect.shrink(22.0).contains(pointer) {
                    let mut pan = Vec2::ZERO;
                    if pointer.x < view.rect.left() + 22.0 {
                        pan.x = 7.0;
                    } else if pointer.x > view.rect.right() - 22.0 {
                        pan.x = -7.0;
                    }
                    if pointer.y < view.rect.top() + 22.0 {
                        pan.y = 7.0;
                    } else if pointer.y > view.rect.bottom() - 22.0 {
                        pan.y = -7.0;
                    }
                    data.pan[0] += pan.x;
                    data.pan[1] += pan.y;
                    changed |= pan != Vec2::ZERO;
                    ui.ctx().request_repaint();
                }
            } else {
                match active {
                    Drag::Erase { last, saved } => {
                        changed |= self.erase(data, *last, world, saved);
                    }
                    Drag::Marquee { start, current } => {
                        let rect = Rect::from_two_pos((*start).into(), (*current).into());
                        for e in &data.elements {
                            if rect.contains(geo::bounds(data, e).center())
                                && !self.selected.contains(&e.id)
                            {
                                self.selected.push(e.id);
                            }
                        }
                    }
                    Drag::Lasso { points } => {
                        for e in &data.elements {
                            if geo::in_polygon(geo::bounds(data, e).center().into(), points)
                                && !self.selected.contains(&e.id)
                            {
                                self.selected.push(e.id);
                            }
                        }
                    }
                    Drag::Draw { id, .. } => {
                        if data.element(*id).is_some_and(|e| {
                            matches!(e.kind, ElementKind::Pen | ElementKind::Marker)
                        }) {
                            self.selected.clear();
                        }
                        if data.element(*id).is_some_and(|e| {
                            matches!(e.kind, ElementKind::Text | ElementKind::Sticky)
                        }) {
                            self.editing = Some(*id);
                            self.focus_text = true;
                            self.tool = Tool::Select;
                        }
                        if data.element(*id).is_some_and(|e| {
                            !matches!(e.kind, ElementKind::Pen | ElementKind::Marker)
                        }) {
                            self.tool = Tool::Select;
                        }
                    }
                    _ => {}
                }
                drag = None;
            }
        }
        self.drag = drag;
        changed
    }

    fn handle_at(&self, data: &CanvasData, point: Point, tolerance: f32) -> Option<Drag> {
        if self.selected.len() != 1 {
            return None;
        }
        let e = data.element(self.selected[0])?;
        if e.locked {
            return None;
        }
        if e.kind == ElementKind::Arrow {
            let (a, b) = geo::endpoints(data, e);
            if geo::distance(point, a) < tolerance {
                return Some(Drag::Endpoint {
                    id: e.id,
                    start: true,
                });
            }
            if geo::distance(point, b) < tolerance {
                return Some(Drag::Endpoint {
                    id: e.id,
                    start: false,
                });
            }
            if e.route != ArrowRoute::Straight
                && geo::distance(point, geo::bend_handle(data, e)) < tolerance
            {
                return Some(Drag::Bend { id: e.id });
            }
        } else if !e.kind.has_path() && geo::distance(point, geo::add(e.pos, e.size)) < tolerance {
            return Some(Drag::Resize { id: e.id });
        }
        None
    }

    fn paint_paper(&self, painter: &egui::Painter, view: View, paper: Paper) {
        painter.rect_filled(view.rect, 0.0, PAPER);
        if paper == Paper::Blank {
            return;
        }
        let spacing = if paper == Paper::Ruled { 32.0 } else { 24.0 } * view.zoom;
        if spacing < 6.0 {
            return;
        }
        let stroke = Stroke::new(0.65_f32, Color32::from_rgb(231, 232, 229));
        let y0 = view.rect.top() + view.pan[1].rem_euclid(spacing);
        let mut y = y0;
        while y < view.rect.bottom() {
            painter.line_segment(
                [
                    Pos2::new(view.rect.left(), y),
                    Pos2::new(view.rect.right(), y),
                ],
                stroke,
            );
            y += spacing;
        }
        if paper == Paper::Grid {
            let mut x = view.rect.left() + view.pan[0].rem_euclid(spacing);
            while x < view.rect.right() {
                painter.line_segment(
                    [
                        Pos2::new(x, view.rect.top()),
                        Pos2::new(x, view.rect.bottom()),
                    ],
                    stroke,
                );
                x += spacing;
            }
        }
    }

    fn paint(&mut self, painter: &egui::Painter, data: &CanvasData, view: View) {
        for e in &data.elements {
            let bounds = view.screen_rect(geo::bounds(data, e));
            if !bounds.expand(100.0).intersects(view.rect) {
                continue;
            }
            let stroke = Stroke::new(e.style.width * view.zoom, color(e.style.stroke));
            let rect = Rect::from_min_size(
                view.screen(e.pos),
                Vec2::new(e.size[0], e.size[1]) * view.zoom,
            );
            if e.kind == ElementKind::Image {
                if !self.textures.contains_key(&e.id) {
                    if let Some(image) = &e.image {
                        match image::load_from_memory(image.bytes.as_slice()) {
                            Ok(decoded) => {
                                let rgba = decoded.to_rgba8();
                                let image = egui::ColorImage::from_rgba_unmultiplied(
                                    [rgba.width() as usize, rgba.height() as usize],
                                    rgba.as_raw(),
                                );
                                self.textures.insert(
                                    e.id,
                                    painter.ctx().load_texture(
                                        format!("canvas-image-{}", e.id),
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ),
                                );
                            }
                            Err(_) => {
                                self.error =
                                    Some("Salah satu gambar tersimpan tidak dapat dibaca.".into());
                            }
                        }
                    }
                }
                if let Some(texture) = self.textures.get(&e.id) {
                    painter.image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            } else if e.kind.has_path() {
                let points = if e.kind == ElementKind::Arrow {
                    geo::arrow_path(data, e)
                } else {
                    e.points.clone()
                };
                painter.add(egui::Shape::line(
                    points.iter().map(|p| view.screen(*p)).collect(),
                    stroke,
                ));
                if e.kind == ElementKind::Arrow {
                    painter.add(egui::Shape::line(
                        geo::arrowhead(&points, e.style.width)
                            .into_iter()
                            .map(|p| view.screen(p))
                            .collect(),
                        stroke,
                    ));
                }
            } else if e.kind != ElementKind::Text {
                if e.kind == ElementKind::Sticky {
                    painter.rect_filled(
                        rect.translate(Vec2::new(3.0, 4.0) * view.zoom),
                        2.0,
                        Color32::from_black_alpha(17),
                    );
                }
                match e.kind {
                    ElementKind::Ellipse => {
                        painter.add(egui::Shape::ellipse_filled(
                            rect.center(),
                            rect.size() * 0.5,
                            color(e.style.fill),
                        ));
                    }
                    ElementKind::Diamond => {
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                rect.center_top(),
                                rect.right_center(),
                                rect.center_bottom(),
                                rect.left_center(),
                            ],
                            color(e.style.fill),
                            Stroke::NONE,
                        ));
                    }
                    _ => {
                        painter.rect_filled(rect, 2.0, color(e.style.fill));
                    }
                }
                painter.add(egui::Shape::line(
                    geo::outline(e)
                        .into_iter()
                        .map(|p| view.screen(p))
                        .collect(),
                    stroke,
                ));
            }
            if e.kind == ElementKind::Text && e.text.trim().is_empty() && self.editing != Some(e.id)
            {
                let outline = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ];
                painter.extend(egui::Shape::dashed_line(
                    &outline,
                    Stroke::new(1.0_f32, Color32::from_gray(145)),
                    5.0,
                    4.0,
                ));
                painter.with_clip_rect(rect.intersect(view.rect)).text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Teks kosong",
                    FontId::proportional((14.0 * view.zoom).clamp(10.0, 18.0)),
                    Color32::from_gray(100),
                );
            }
            if !e.text.is_empty() && self.editing != Some(e.id) {
                let content = text_content_rect(data, e, view);
                let galley = painter.layout(
                    e.text.clone(),
                    hand(e.style.font_size * view.zoom),
                    INK,
                    content.width(),
                );
                let text_rect = Rect::from_center_size(content.center(), galley.size());
                if e.kind.has_path() {
                    painter.rect_filled(text_rect.expand(4.0), 3.0, PAPER);
                }
                painter
                    .with_clip_rect(if e.kind.has_path() {
                        view.rect
                    } else {
                        rect.intersect(view.rect)
                    })
                    .galley(text_rect.min, galley, INK);
            }
            if self.selected.contains(&e.id) && matches!(self.tool, Tool::Select | Tool::Lasso) {
                painter.rect_stroke(
                    bounds.expand(5.0),
                    2.0,
                    Stroke::new(1.0_f32, FOCUS),
                    egui::StrokeKind::Outside,
                );
                if !e.locked && self.selected.len() == 1 {
                    let handles = if e.kind == ElementKind::Arrow {
                        let (a, b) = geo::endpoints(data, e);
                        let mut list = vec![a, b];
                        if e.route != ArrowRoute::Straight {
                            list.push(geo::bend_handle(data, e));
                        }
                        list
                    } else if !e.kind.has_path() {
                        vec![geo::add(e.pos, e.size)]
                    } else {
                        vec![]
                    };
                    for handle in handles {
                        painter.circle_filled(view.screen(handle), 5.0, Color32::WHITE);
                        painter.circle_stroke(
                            view.screen(handle),
                            5.0,
                            Stroke::new(1.5_f32, FOCUS),
                        );
                    }
                }
            }
        }
        if let Some(drag) = &self.drag {
            match drag {
                Drag::Marquee { start, current } => {
                    painter.rect_filled(
                        Rect::from_two_pos(view.screen(*start), view.screen(*current)),
                        0.0,
                        Color32::from_rgba_unmultiplied(133, 101, 189, 30),
                    );
                }
                Drag::Lasso { points } => {
                    let mut points: Vec<_> = points.iter().map(|p| view.screen(*p)).collect();
                    if let Some(first) = points.first().copied() {
                        points.push(first);
                    }
                    painter.add(egui::Shape::line(points, Stroke::new(1.2_f32, FOCUS)));
                }
                _ => {}
            }
        }
        if self.tool == Tool::Eraser {
            if let Some(pointer) = painter.ctx().input(|i| i.pointer.hover_pos()).filter(|p| {
                view.rect.contains(*p) && painter.ctx().layer_id_at(*p) == Some(painter.layer_id())
            }) {
                painter.ctx().set_cursor_icon(egui::CursorIcon::None);
                let radius = self.eraser_size() * 0.5;
                painter.circle_stroke(pointer, radius, Stroke::new(3.0_f32, Color32::WHITE));
                painter.circle_stroke(pointer, radius, Stroke::new(1.0_f32, Color32::BLACK));
                painter.circle_filled(pointer, 1.5, Color32::BLACK);
            }
        }
        if self.tool == Tool::Arrow
            || matches!(self.drag, Some(Drag::Endpoint { .. }))
            || matches!(self.drag, Some(Drag::Draw { id, .. }) if data.element(id).is_some_and(|e| e.kind == ElementKind::Arrow))
        {
            if let Some(pointer) = painter
                .ctx()
                .input(|i| i.pointer.hover_pos())
                .filter(|p| view.rect.contains(*p))
            {
                let point = view.world(pointer);
                let exclude = self.selected.first().copied();
                if !painter.ctx().input(|i| i.modifiers.alt) {
                    if let Some((binding, anchor)) =
                        geo::suggest_binding(data, point, exclude, 22.0 / data.zoom)
                    {
                        if let Some(target) = data.element(binding.target) {
                            painter.rect_stroke(
                                view.screen_rect(geo::bounds(data, target)).expand(7.0),
                                4.0,
                                Stroke::new(1.5_f32, FOCUS),
                                egui::StrokeKind::Outside,
                            );
                        }
                        painter.circle_filled(view.screen(anchor), 5.0, FOCUS);
                        painter.text(
                            pointer + Vec2::new(14.0, -16.0),
                            egui::Align2::LEFT_BOTTOM,
                            "Sambungkan otomatis",
                            FontId::proportional(12.0),
                            FOCUS,
                        );
                    }
                }
            }
        }
    }

    fn text_editor(&mut self, ui: &mut egui::Ui, data: &mut CanvasData, view: View) -> bool {
        let Some(id) = self.editing else {
            return false;
        };
        let Some(element) = data.element(id) else {
            self.editing = None;
            return false;
        };
        if element.locked
            || matches!(
                element.kind,
                ElementKind::Image | ElementKind::Pen | ElementKind::Marker
            )
        {
            self.editing = None;
            return false;
        }
        let rect = text_content_rect(data, element, view);
        let element = data.element_mut(id).unwrap();
        // Floating text must not advance the row that positions the settings panel.
        let mut editor_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("canvas-text-overlay", id))
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        editor_ui.set_clip_rect(view.rect.intersect(ui.clip_rect()));
        let response = editor_ui.add(
            egui::TextEdit::multiline(&mut element.text)
                .id(egui::Id::new(("canvas-text", id)))
                .font(hand(element.style.font_size * view.zoom))
                .text_color(INK)
                .frame(false)
                .margin(egui::Margin::ZERO)
                .desired_width(rect.width())
                .desired_rows(1)
                .min_size(rect.size())
                .horizontal_align(egui::Align::Center)
                .vertical_align(egui::Align::Center)
                .hint_text("Tulis di sini..."),
        );
        if self.focus_text {
            response.request_focus();
            self.focus_text = false;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
            || (response.lost_focus() && ui.input(|i| i.pointer.any_pressed()))
        {
            self.editing = None;
        }
        response.changed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        editor: &mut CanvasEditor,
        data: &mut CanvasData,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1180.0, 850.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    editor.show(ui, data, &mut "Pengujian".to_owned());
                });
            },
        )
    }
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn arrow_preview_and_connection_stay_on_the_cursor_side_at_every_zoom() {
        let has_route = |output: &egui::FullOutput, label: &str| {
            output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label)
        })
        };
        for zoom in [0.5, 1.0, 2.0] {
            let ctx = egui::Context::default();
            crate::configure(&ctx);
            let mut editor = CanvasEditor::default();
            let mut data = CanvasData {
                zoom,
                ..Default::default()
            };
            editor.create(&mut data, ElementKind::Rectangle, [60.0, 80.0]);
            let shape_panel = frame(&mut editor, &mut data, &ctx, vec![]);
            assert!(!has_route(&shape_panel, "Panah lurus"));
            editor.choose_tool(Tool::Arrow);
            let arrow_panel = frame(&mut editor, &mut data, &ctx, vec![]);
            assert!(has_route(&arrow_panel, "Panah lurus"));
            let origin = editor.last_rect.unwrap().min;
            let cursor = origin + Vec2::new(220.0 * zoom + 10.0, 105.0 * zoom);
            let anchor = origin + Vec2::new(220.0, 105.0) * zoom;
            let preview = frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(cursor)],
            );
            assert!(preview.shapes.iter().any(|s| {
                matches!(&s.shape, egui::Shape::Circle(c) if c.fill == FOCUS && c.center.distance(anchor) < 0.001)
            }));
            let to = origin + Vec2::new(310.0, 220.0) * zoom;
            frame(&mut editor, &mut data, &ctx, vec![button(cursor, true)]);
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(to)],
            );
            frame(&mut editor, &mut data, &ctx, vec![button(to, false)]);
            let arrow = data.elements.last().unwrap();
            assert_eq!(arrow.kind, ElementKind::Arrow);
            assert!(geo::distance(geo::endpoints(&data, arrow).0, [220.0, 105.0]) < 0.001);
            let arrow_id = arrow.id;
            editor.selected = vec![arrow_id];
            let top = origin + Vec2::new(100.0, 80.0) * zoom - Vec2::new(0.0, 10.0);
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(to), button(to, true)],
            );
            let preview = frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(top)],
            );
            let expected = origin + Vec2::new(100.0, 80.0) * zoom;
            assert!(preview.shapes.iter().any(|s| {
                matches!(&s.shape, egui::Shape::Circle(c) if c.fill == FOCUS && c.center.distance(expected) < 0.001)
            }));
            frame(&mut editor, &mut data, &ctx, vec![button(top, false)]);
            assert!(
                geo::distance(
                    geo::endpoints(&data, data.element(arrow_id).unwrap()).1,
                    [100.0, 80.0]
                ) < 0.001
            );
            data.element_mut(arrow_id).unwrap().route = ArrowRoute::Elbow;
            editor.route = ArrowRoute::Straight;
            let before = data.clone();
            let selected_arrow = frame(&mut editor, &mut data, &ctx, vec![]);
            assert!(has_route(&selected_arrow, "Panah siku"));
            assert_eq!(data, before);
            editor.selected = vec![data.elements[0].id, arrow_id];
            let mixed = frame(&mut editor, &mut data, &ctx, vec![]);
            assert!(!has_route(&mixed, "Panah siku"));
            editor.choose_tool(Tool::Line);
            let line_panel = frame(&mut editor, &mut data, &ctx, vec![]);
            assert!(!has_route(&line_panel, "Panah siku"));
        }
    }

    #[test]
    fn dragging_a_shape_across_its_target_reroutes_the_arrow_and_undo_restores_anchors() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let source = editor.create(&mut data, ElementKind::Rectangle, [30.0, 70.0]);
        let target = editor.create(&mut data, ElementKind::Rectangle, [290.0, 70.0]);
        let id = editor.create(&mut data, ElementKind::Arrow, [190.0, 120.0]);
        let arrow = data.element_mut(id).unwrap();
        arrow.points = vec![[190.0, 120.0], [290.0, 120.0]];
        arrow.start_binding = Some(Binding {
            target: source,
            anchor: Some([1.0, 0.5]),
        });
        arrow.end_binding = Some(Binding {
            target,
            anchor: Some([0.0, 0.5]),
        });
        editor.choose_tool(Tool::Select);
        frame(&mut editor, &mut data, &ctx, vec![]);
        let before = data.clone();
        let origin = editor.last_rect.unwrap().min;
        let from = origin + Vec2::new(100.0, 90.0);
        let to = from + Vec2::new(470.0, 170.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&mut editor, &mut data, &ctx, vec![button(to, false)]);
        let moved = data.clone();
        let (a, b) = geo::endpoints(&data, data.element(id).unwrap());
        assert!(a[0] < data.element(source).unwrap().center()[0]);
        assert!(b[0] > data.element(target).unwrap().center()[0]);
        assert_eq!(editor.undo.len(), 1);
        assert!(editor.history(&mut data, false));
        frame(&mut editor, &mut data, &ctx, vec![]);
        assert_eq!(data, before);
        assert!(editor.history(&mut data, true));
        frame(&mut editor, &mut data, &ctx, vec![]);
        assert_eq!(data, moved);
        editor.checkpoint(&data);
        data.element_mut(target).unwrap().size = [180.0, 120.0];
        frame(&mut editor, &mut data, &ctx, vec![]);
        assert_eq!(editor.undo.len(), 2);
        assert!(
            data.element(id)
                .unwrap()
                .end_binding
                .as_ref()
                .unwrap()
                .anchor
                .is_none()
        );
    }

    #[test]
    fn new_shapes_do_not_inherit_the_selected_sticky_or_marker_style() {
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        editor.create(&mut data, ElementKind::Sticky, [0.0, 0.0]);
        assert_ne!(editor.style, Style::default());
        for tool in [
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Diamond,
            Tool::Arrow,
            Tool::Line,
        ] {
            editor.choose_tool(tool);
            let id = editor.create(&mut data, tool.kind().unwrap(), [0.0, 0.0]);
            let style = data.element(id).unwrap().style;
            assert_eq!(style.stroke, [0, 0, 0, 255]);
            assert_eq!(style.width, 2.0);
            assert_eq!(style.fill[3], 0);
            editor.create(&mut data, ElementKind::Marker, [0.0, 0.0]);
            editor.style.fill = [170, 170, 170, 255];
        }
    }

    #[test]
    fn empty_text_marker_disappears_when_filled_and_does_not_export() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let id = editor.create(&mut data, ElementKind::Text, [100.0, 100.0]);
        editor.choose_tool(Tool::Select);
        let marker = |output: &egui::FullOutput| {
            output.shapes.iter().any(
                |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Teks kosong"),
            )
        };
        frame(&mut editor, &mut data, &ctx, vec![]);
        assert!(marker(&frame(&mut editor, &mut data, &ctx, vec![])));
        assert!(!data.svg("Tes", false).contains("Teks kosong"));
        data.element_mut(id).unwrap().text = "Isi teks".into();
        assert!(!marker(&frame(&mut editor, &mut data, &ctx, vec![])));
        data.element_mut(id).unwrap().text = "  ".into();
        assert!(marker(&frame(&mut editor, &mut data, &ctx, vec![])));
        let at = editor.last_rect.unwrap().min + Vec2::new(150.0, 140.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(at), button(at, true)],
        );
        frame(&mut editor, &mut data, &ctx, vec![button(at, false)]);
        assert_eq!(editor.selected, vec![id]);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(data.elements.is_empty());
    }

    #[test]
    fn eraser_drag_matches_preview_size_and_undo_restores_the_whole_gesture() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData {
            zoom: 2.0,
            ..Default::default()
        };
        let id = editor.create(&mut data, ElementKind::Pen, [20.0, 100.0]);
        data.element_mut(id).unwrap().points = vec![[20.0, 100.0], [200.0, 100.0]];
        let before = data.clone();
        editor.choose_tool(Tool::Eraser);
        editor.eraser_size = Some(40.0);
        frame(&mut editor, &mut data, &ctx, vec![]);
        let origin = editor.last_rect.unwrap().min;
        let from = origin + Vec2::new(200.0, 140.0);
        let middle = origin + Vec2::new(200.0, 200.0);
        let to = origin + Vec2::new(200.0, 260.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        let preview = frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(middle)],
        );
        assert!(preview.shapes.iter().any(|s| {
            matches!(&s.shape, egui::Shape::Circle(c) if c.center == middle && c.radius == 20.0 && c.stroke.color == Color32::BLACK)
        }));
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&mut editor, &mut data, &ctx, vec![button(to, false)]);
        assert_eq!(data.elements.len(), 2);
        assert!((data.elements[0].points.last().unwrap()[0] - 89.0).abs() < 0.001);
        assert!((data.elements[1].points[0][0] - 111.0).abs() < 0.001);
        assert_eq!(editor.undo.len(), 1);
        assert!(editor.history(&mut data, false));
        assert_eq!(data, before);
        assert!(editor.history(&mut data, true));
        assert_eq!(data.elements.len(), 2);
        let mut saved = false;
        assert!(!editor.erase(&mut data, [300.0, 300.0], [400.0, 300.0], &mut saved));
        assert!(!saved);
        assert_eq!(editor.undo.len(), 1);
        let settings = frame(&mut editor, &mut data, &ctx, vec![]);
        let rail = settings
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == Color32::from_gray(145)
                        && rect.rect.width() > 180.0
                        && rect.rect.height() == 4.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("The eraser size control needs a visible, full-width rail");
        let at = egui::pos2(rail.right() - 12.0, rail.center().y);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(at), button(at, true)],
        );
        frame(&mut editor, &mut data, &ctx, vec![button(at, false)]);
        assert!(editor.eraser_size() > 100.0);
        assert_eq!(editor.undo.len(), 1);
    }
    #[test]
    fn clicking_blank_canvas_ends_text_editing_and_clears_selection() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let id = editor.create(&mut data, ElementKind::Sticky, [100.0, 70.0]);
        data.element_mut(id).unwrap().text = "Catatan tetap".into();
        let before = data.clone();
        editor.editing = Some(id);
        editor.focus_text = true;
        frame(&mut editor, &mut data, &ctx, vec![]);
        frame(&mut editor, &mut data, &ctx, vec![]);
        let input_id = egui::Id::new(("canvas-text", id));
        let inside = ctx.read_response(input_id).unwrap().rect.center();
        for _ in 0..2 {
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(inside), button(inside, true)],
            );
            frame(&mut editor, &mut data, &ctx, vec![button(inside, false)]);
        }
        assert_eq!(editor.editing, Some(id));
        assert_eq!(editor.selected, vec![id]);
        let blank = editor.last_rect.unwrap().min + Vec2::new(500.0, 430.0);
        for _ in 0..2 {
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(blank), button(blank, true)],
            );
            frame(&mut editor, &mut data, &ctx, vec![button(blank, false)]);
            assert!(editor.selected.is_empty());
            assert_eq!(editor.editing, None);
            assert!(!ctx.memory(|memory| memory.has_focus(input_id)));
        }
        assert_eq!(data, before);
        assert!(editor.undo.is_empty());
    }

    #[test]
    fn editing_canvas_text_keeps_panels_fixed_and_input_inside_the_canvas() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let id = editor.create(&mut data, ElementKind::Text, [190.0, 160.0]);
        data.element_mut(id).unwrap().text = "Pilih teks ini".into();
        frame(&mut editor, &mut data, &ctx, vec![]);
        let panel_id = egui::Id::new("canvas-settings-bounds");
        let before_panel = ctx.data(|d| d.get_temp::<Rect>(panel_id)).unwrap();
        let before_canvas = editor.last_rect.unwrap();
        editor.checkpoint(&data);
        editor.editing = Some(id);
        editor.focus_text = true;
        for _ in 0..2 {
            frame(&mut editor, &mut data, &ctx, vec![]);
            assert_eq!(
                ctx.data(|d| d.get_temp::<Rect>(panel_id)).unwrap(),
                before_panel
            );
            assert_eq!(editor.last_rect.unwrap(), before_canvas);
        }
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::Text(" mengetik di canvas".into())],
        );
        assert!(
            data.element(id)
                .unwrap()
                .text
                .contains("mengetik di canvas")
        );
        assert_eq!(
            ctx.data(|d| d.get_temp::<Rect>(panel_id)).unwrap(),
            before_panel
        );
        assert_eq!(editor.last_rect.unwrap(), before_canvas);
        data.element_mut(id).unwrap().pos[0] = before_canvas.width() - 35.0;
        for _ in 0..2 {
            frame(&mut editor, &mut data, &ctx, vec![]);
        }
        let input = ctx
            .read_response(egui::Id::new(("canvas-text", id)))
            .unwrap();
        assert!(input.rect.right() > before_canvas.right());
        assert!(before_canvas.contains_rect(input.interact_rect));
        assert!(!input.interact_rect.intersects(before_panel));
        assert_eq!(
            ctx.data(|d| d.get_temp::<Rect>(panel_id)).unwrap(),
            before_panel
        );
    }

    #[test]
    fn pointer_drawing_and_keyboard_shape_text_are_editable() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        frame(&mut editor, &mut data, &ctx, vec![]);
        let rect = editor.last_rect.unwrap();
        let from = rect.min + Vec2::new(90.0, 80.0);
        let to = from + Vec2::new(180.0, 110.0);
        editor.tool = Tool::Rectangle;
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&mut editor, &mut data, &ctx, vec![button(to, false)]);
        assert_eq!(data.elements.len(), 1);
        assert_eq!(data.elements[0].kind, ElementKind::Rectangle);
        assert!(data.elements[0].size[0] > 170.0 && data.elements[0].size[1] > 100.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::Key {
                key: Key::F2,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::Text("Teks di dalam shape".into())],
        );
        assert_eq!(data.elements[0].text, "Teks di dalam shape");
    }
    #[test]
    fn pen_and_marker_finish_without_selection_boxes() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        for tool in [Tool::Pen, Tool::Marker] {
            editor.tool = tool;
            frame(&mut editor, &mut data, &ctx, vec![]);
            let from = editor.last_rect.unwrap().min + Vec2::new(80.0, 90.0);
            let to = from + Vec2::new(150.0, 120.0);
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(from), button(from, true)],
            );
            frame(
                &mut editor,
                &mut data,
                &ctx,
                vec![egui::Event::PointerMoved(to)],
            );
            frame(&mut editor, &mut data, &ctx, vec![button(to, false)]);
            assert_eq!(editor.tool, tool);
            assert!(editor.selected.is_empty());
            assert!(editor.drag.is_none());
        }
        assert_eq!(data.elements.len(), 2);
        assert_eq!(data.elements[0].kind, ElementKind::Pen);
        assert_eq!(data.elements[1].kind, ElementKind::Marker);
    }

    #[test]
    fn icon_grid_selects_tools_and_scrolls_without_panning_canvas() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let mut run = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(380.0, 520.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        editor.show(ui, &mut data, &mut "Pengujian".to_owned());
                    });
                },
            )
        };
        run(vec![]);
        let pen = ctx
            .read_response(egui::Id::new(("canvas-tool", "Pena")))
            .unwrap()
            .rect
            .center();
        run(vec![egui::Event::PointerMoved(pen), button(pen, true)]);
        run(vec![button(pen, false)]);
        let rail = ctx
            .data(|d| d.get_temp::<Rect>(egui::Id::new("canvas-toolbar-bounds")))
            .unwrap();
        run(vec![
            egui::Event::PointerMoved(rail.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -1200.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        for _ in 0..30 {
            run(vec![]);
        }
        let add = ctx
            .read_response(egui::Id::new(("canvas-tool", "Tambah di tengah")))
            .unwrap()
            .rect;
        assert!(
            rail.contains_rect(add),
            "Last tool should be reachable by scrolling: {add:?}, rail {rail:?}"
        );
        let settings = ctx
            .data(|d| d.get_temp::<Rect>(egui::Id::new("canvas-settings-toggle")))
            .unwrap()
            .center();
        run(vec![
            egui::Event::PointerMoved(settings),
            button(settings, true),
        ]);
        run(vec![button(settings, false)]);
        let output = run(vec![]);
        fn has_settings(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Text(text) => text.galley.job.text == "Gaya",
                egui::Shape::Vec(shapes) => shapes.iter().any(has_settings),
                _ => false,
            }
        }
        assert!(
            output.shapes.iter().any(|s| has_settings(&s.shape)),
            "Compact settings should open from the right panel"
        );
        assert_eq!(editor.tool, Tool::Pen);
        assert_eq!(data.pan, [0.0, 0.0]);
        assert!(data.elements.is_empty());
    }

    #[test]
    fn right_drag_pans_with_pen_selected_without_creating_a_stroke() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        editor.tool = Tool::Pen;
        frame(&mut editor, &mut data, &ctx, vec![]);
        let at = editor.last_rect.unwrap().center();
        let right = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(at), right(at, true)],
        );
        let to = at + Vec2::new(80.0, -50.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&mut editor, &mut data, &ctx, vec![right(to, false)]);
        assert_eq!(data.pan, [80.0, -50.0]);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![egui::Event::PointerMoved(at)],
        );
        let touch = |id, phase, pos| egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(id),
            phase,
            pos,
            force: None,
        };
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![
                touch(1, egui::TouchPhase::Start, at - Vec2::new(20.0, 0.0)),
                touch(2, egui::TouchPhase::Start, at + Vec2::new(20.0, 0.0)),
            ],
        );
        let to = at + Vec2::new(40.0, 30.0);
        frame(
            &mut editor,
            &mut data,
            &ctx,
            vec![
                touch(1, egui::TouchPhase::Move, to - Vec2::new(20.0, 0.0)),
                touch(2, egui::TouchPhase::Move, to + Vec2::new(20.0, 0.0)),
            ],
        );
        assert_eq!(data.pan, [120.0, -20.0]);
        assert!(data.elements.is_empty() && editor.undo.is_empty());
        assert_eq!(editor.tool, Tool::Pen);
    }

    #[test]
    fn image_import_and_undo_preserve_embedded_bytes() {
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/notvas-icon-256.png");
        assert!(editor.import_image(&mut data, &path, [10.0, 10.0]));
        let bytes = data.elements[0].image.as_ref().unwrap().bytes.clone();
        let json = serde_json::to_string(&data).unwrap();
        let mut restored: CanvasData = serde_json::from_str(&json).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.elements[0].image.as_ref().unwrap().bytes, bytes);
        assert!(editor.history(&mut data, false));
        assert!(data.elements.is_empty());
        assert!(editor.history(&mut data, true));
        assert_eq!(data.elements[0].image.as_ref().unwrap().bytes, bytes);
    }

    #[test]
    fn template_exports_keep_labels_and_connected_arrows() {
        let mut editor = CanvasEditor::default();
        let mut data = CanvasData::default();
        editor.template(&mut data, 0);
        assert_eq!(
            data.elements
                .iter()
                .filter(|e| e.kind == ElementKind::Arrow
                    && e.start_binding.is_some()
                    && e.end_binding.is_some())
                .count(),
            2
        );
        let title = editor.create(&mut data, ElementKind::Text, [80.0, -20.0]);
        let e = data.element_mut(title).unwrap();
        e.size = [700.0, 90.0];
        e.text = "Tuangkan ide, lalu hubungkan.".into();
        e.style.font_size = 38.0;
        for (i, label) in [
            "Catatan kecil untuk ide besar",
            "Geser shape.\nPanah ikut tersambung.",
        ]
        .iter()
        .enumerate()
        {
            let id = editor.create(
                &mut data,
                ElementKind::Sticky,
                [160.0 + i as f32 * 260.0, 310.0],
            );
            let e = data.element_mut(id).unwrap();
            e.text = (*label).into();
            e.size = [210.0, 160.0];
            e.style.fill = FILLS[if i == 0 { 1 } else { 3 }];
        }
        data.validate().unwrap();
        let svg = data.svg("Contoh canvas", true);
        assert!(svg.contains("Kerjakan") && svg.contains("Patrick Hand"));
        let png = data.png("Contoh canvas").unwrap();
        assert!(png.len() > 1000);
        if let Some(folder) = std::env::var_os("NOTVAS_QA_DIR") {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("whiteboard-preview.png"), png).unwrap();
            std::fs::write(folder.join("whiteboard-preview.svg"), svg).unwrap();
        }
    }
}
