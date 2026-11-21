#![windows_subsystem = "windows"]

mod autosave;
mod canvas;
mod controls;
#[cfg(test)]
mod docs_render;
mod header;
mod model;
mod navigation;
mod note_images;
mod notes_board;
mod storage;
mod window_chrome;
mod zoom;

use autosave::Autosave;
use egui::{Color32, FontFamily, FontId, RichText, Stroke, TextEdit};
use model::{Kind, RichDocument, Task, TextStyle, Workspace};
use navigation::PanScrollArea;
use std::{path::PathBuf, time::Instant};

const BG: Color32 = Color32::from_gray(250);
const PANEL: Color32 = Color32::from_gray(247);
const CARD: Color32 = Color32::from_gray(239);
const TEXT: Color32 = Color32::from_gray(40);
const MUTED: Color32 = Color32::from_gray(99);
const ACCENT: Color32 = Color32::from_gray(70);
const BUTTON: Color32 = Color32::TRANSPARENT;
const BUTTON_HOVER: Color32 = Color32::from_gray(244);
const BUTTON_ACTIVE: Color32 = Color32::from_gray(233);
const BUTTON_BORDER: Color32 = Color32::from_gray(220);
const SHELL_INSET: i8 = 6;
const SHELL_BORDER: Color32 = Color32::from_gray(218);
const OUTER_BORDER: Color32 = Color32::from_gray(226);
const SHELL_RADIUS: u8 = 6;

fn note_header_color(color: [u8; 3]) -> Color32 {
    let index = model::NOTE_COLORS
        .iter()
        .position(|candidate| *candidate == color)
        .unwrap_or(0);
    let [r, g, b] = [
        [105, 78, 21],
        [112, 65, 37],
        [120, 54, 77],
        [88, 64, 122],
        [42, 78, 112],
        [59, 86, 46],
    ][index];
    Color32::from_rgb(r, g, b)
}

fn note_header_style(ui: &mut egui::Ui, background: Color32) {
    let hover = background.lerp_to_gamma(Color32::WHITE, 0.08);
    let active = background.lerp_to_gamma(Color32::WHITE, 0.14);
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(Color32::WHITE);
    visuals.weak_text_color = Some(Color32::WHITE);
    visuals.panel_fill = background;
    visuals.window_fill = background;
    visuals.extreme_bg_color = background;
    visuals.faint_bg_color = hover;
    visuals.selection.bg_fill = active;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.text_cursor.stroke.color = Color32::WHITE;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke.color = Color32::WHITE;
        widget.bg_stroke = Stroke::new(0.7_f32, Color32::from_white_alpha(48));
    }
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.hovered.weak_bg_fill = hover;
    visuals.widgets.active.bg_fill = active;
    visuals.widgets.active.weak_bg_fill = active;
    visuals.widgets.open.bg_fill = active;
    visuals.widgets.open.weak_bg_fill = active;
}

struct Notvas {
    content_zoom: [f32; 3],
    note_images: note_images::NoteImages,
    canvas_editor: canvas::CanvasEditor,
    notes_board: notes_board::NotesBoard,
    checklist_board: notes_board::NotesBoard,
    restart_path: Option<PathBuf>,
    logo: Option<egui::TextureHandle>,
    workspace: Workspace,
    path: PathBuf,
    kind: Kind,
    selected: Option<u64>,
    query: String,
    typing: TextStyle,
    selection: std::ops::Range<usize>,
    new_task: String,
    dirty: bool,
    autosave: Autosave,
    status: String,
    error: Option<String>,
    load_failed: bool,
    delete_document: bool,
    pending_close: bool,
    was_focused: bool,
    undo: Vec<RichDocument>,
    redo: Vec<RichDocument>,
}

impl Notvas {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure(&cc.egui_ctx);
        let path = storage::default_path();
        let (workspace, error) = if path.exists() {
            match storage::load(&path) {
                Ok(ws) => (ws, None),
                Err(e) => (Workspace::default(), Some(e)),
            }
        } else {
            (Workspace::default(), None)
        };
        let first = workspace
            .documents
            .iter()
            .find(|d| Kind::NAVIGATION.contains(&d.kind));
        let selected = first.map(|d| d.id);
        let kind = first.map_or(Kind::Note, |d| d.kind);
        let load_failed = error.is_some();
        Self {
            content_zoom: [1.0; 3],
            note_images: note_images::NoteImages::default(),
            canvas_editor: canvas::CanvasEditor::default(),
            notes_board: notes_board::NotesBoard::default(),
            checklist_board: notes_board::NotesBoard::default(),
            restart_path: if cfg!(debug_assertions) {
                std::env::args()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .find(|a| a[0] == "--restart-file")
                    .map(|a| PathBuf::from(&a[1]))
            } else {
                None
            },
            logo: None,
            workspace,
            path,
            kind,
            selected,
            query: String::new(),
            typing: TextStyle::default(),
            selection: 0..0,
            new_task: String::new(),
            dirty: false,
            autosave: Autosave::default(),
            status: "Tersimpan otomatis di perangkat".into(),
            error,
            load_failed,
            delete_document: false,
            pending_close: false,
            was_focused: true,
            undo: vec![],
            redo: vec![],
        }
    }

    fn change(&mut self) {
        self.dirty = true;
        self.autosave.edited(Instant::now());
    }

    fn change_immediately(&mut self) {
        self.dirty = true;
        self.autosave.immediately(Instant::now());
    }

    fn flush(&mut self) -> bool {
        !self.dirty || self.save()
    }

    fn autosave_if_due(&mut self, now: Instant) {
        if self.dirty && !self.load_failed && self.autosave.due(now) {
            self.save();
        }
    }

    fn select(&mut self, id: Option<u64>) {
        self.note_images = note_images::NoteImages::default();
        self.canvas_editor.reset();
        self.selected = id;
        self.selection = 0..0;
        self.typing = TextStyle::default();
        self.undo.clear();
        self.redo.clear();
        self.new_task.clear();
    }

    fn create(&mut self) {
        let id = self.workspace.add(self.kind);
        self.query.clear();
        self.select(Some(id));
        if self.kind == Kind::Note {
            self.notes_board.query.clear();
            self.notes_board.overview = false;
        } else if self.kind == Kind::Checklist {
            self.checklist_board.query.clear();
            self.checklist_board.overview = false;
        }
        self.change_immediately();
    }

    fn save(&mut self) -> bool {
        if self.load_failed {
            self.error = Some("File asal tidak bisa dibaca. Pilih folder penyimpanan lain untuk melanjutkan autosave. File asal tetap aman.".into());
            return false;
        }
        match storage::save(&self.path, &self.workspace) {
            Ok(()) => {
                self.dirty = false;
                self.autosave.saved();
                self.error = None;
                self.status = "Semua perubahan tersimpan otomatis".into();
                true
            }
            Err(e) => {
                self.error = Some(e);
                self.autosave.failed(Instant::now());
                false
            }
        }
    }

    fn choose_storage_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let path = storage::unused_path(&folder);
            match storage::save(&path, &self.workspace) {
                Ok(()) => {
                    self.path = path;
                    self.load_failed = false;
                    self.dirty = false;
                    self.autosave.saved();
                    self.error = None;
                    self.status = "Autosave aktif di folder baru".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn open(&mut self) {
        if !self.flush() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Workspace Notvas", &["notvas", "bak"])
            .pick_file()
        {
            match storage::load(&path) {
                Ok(ws) => {
                    self.workspace = ws;
                    self.path = path;
                    self.load_failed = false;
                    let first = self
                        .workspace
                        .documents
                        .iter()
                        .find(|d| Kind::NAVIGATION.contains(&d.kind))
                        .map(|d| (d.id, d.kind));
                    self.kind = first.map_or(Kind::Note, |(_, kind)| kind);
                    self.select(first.map(|(id, _)| id));
                    self.error = None;
                    self.status = "Workspace dibuka".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn export(&mut self, html: bool) {
        let Some(doc) = self
            .workspace
            .documents
            .iter()
            .find(|d| Some(d.id) == self.selected)
        else {
            return;
        };
        let ext = if html { "html" } else { "txt" };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(ext, &[ext])
            .set_file_name(format!("catatan.{ext}"))
            .save_file()
        {
            let content = if html {
                doc.body.html(&doc.title, doc.spacing)
            } else {
                format!("{}\n\n{}", doc.title, doc.body.plain_text())
            };
            match std::fs::write(&path, content) {
                Ok(()) => self.status = format!("Diekspor ke {}", path.display()),
                Err(e) => self.error = Some(format!("Gagal ekspor: {e}")),
            }
        }
    }

    fn remember(&mut self, before: RichDocument) {
        if self.undo.last() != Some(&before) {
            self.undo.push(before);
        }
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn history(&mut self, redo: bool) {
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        let Some(previous) = stack.pop() else {
            return;
        };
        if let Some(doc) = self
            .workspace
            .documents
            .iter_mut()
            .find(|d| Some(d.id) == self.selected)
        {
            let current = std::mem::replace(&mut doc.body, previous);
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.selection = 0..0;
            self.change();
        }
    }

    fn apply_format(&mut self, previous: TextStyle) {
        if self.selection.is_empty() {
            return;
        }
        if let Some(doc) = self
            .workspace
            .documents
            .iter_mut()
            .find(|d| Some(d.id) == self.selected)
        {
            let before = doc.body.clone();
            let end = self.selection.end.min(doc.body.styles.len());
            let start = self.selection.start.min(end);
            for style in &mut doc.body.styles[start..end] {
                if previous.bold != self.typing.bold {
                    style.bold = self.typing.bold;
                }
                if previous.italic != self.typing.italic {
                    style.italic = self.typing.italic;
                }
                if previous.underline != self.typing.underline {
                    style.underline = self.typing.underline;
                }
                if previous.size != self.typing.size {
                    style.size = self.typing.size;
                }
            }
            self.remember(before);
            self.change_immediately();
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let command = egui::Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(command, egui::Key::N)) {
            self.create();
        }
        if ctx.input_mut(|i| i.consume_key(command, egui::Key::O)) {
            self.open();
        }
        if self.kind == Kind::Canvas && ctx.input_mut(|i| i.consume_key(command, egui::Key::S)) {
            if let Some(doc) = self
                .workspace
                .documents
                .iter()
                .find(|d| Some(d.id) == self.selected)
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Salinan canvas Notvas", &["notvas"])
                    .set_file_name("canvas-copy.notvas")
                    .save_file()
                {
                    let copy = Workspace {
                        version: 1,
                        next_id: self.workspace.next_id,
                        documents: vec![doc.clone()],
                    };
                    if let Err(error) = storage::save(&path, &copy) {
                        self.error = Some(error);
                    }
                }
            }
        }
        let editor_focused = ctx.memory(|m| m.has_focus(egui::Id::new("note-body")));
        if editor_focused {
            if ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
            }) {
                self.history(true);
            } else if ctx.input_mut(|i| i.consume_key(command, egui::Key::Z)) {
                self.history(false);
            }
            if ctx.input_mut(|i| i.consume_key(command, egui::Key::Y)) {
                self.history(true);
            }
            for (key, field) in [(egui::Key::B, 0), (egui::Key::I, 1), (egui::Key::U, 2)] {
                if ctx.input_mut(|i| i.consume_key(command, key)) {
                    let previous = self.typing;
                    match field {
                        0 => self.typing.bold = !self.typing.bold,
                        1 => self.typing.italic = !self.typing.italic,
                        _ => self.typing.underline = !self.typing.underline,
                    }
                    self.apply_format(previous);
                }
            }
        }
    }

    fn switch_kind(&mut self, kind: Kind) {
        if self.kind == kind {
            if matches!(kind, Kind::Note | Kind::Checklist) {
                self.flush();
                if kind == Kind::Note {
                    self.notes_board.overview = true;
                } else {
                    self.checklist_board.overview = true;
                }
            }
            return;
        }
        self.flush();
        self.kind = kind;
        self.notes_board.overview = kind == Kind::Note;
        self.checklist_board.overview = kind == Kind::Checklist;
        self.query.clear();
        self.select(
            self.workspace
                .documents
                .iter()
                .find(|d| d.kind == kind)
                .map(|d| d.id),
        );
    }

    fn mode_tabs(&mut self, ui: &mut egui::Ui) {
        for kind in Kind::NAVIGATION {
            let label = RichText::new(kind.label());
            if ui
                .add_sized(
                    [76.0, 28.0],
                    controls::button_with_text(
                        ui,
                        kind.label(),
                        if self.kind == kind {
                            label.family(FontFamily::Name("bold".into())).strong()
                        } else {
                            label
                        },
                    )
                    .frame(false),
                )
                .clicked()
            {
                self.switch_kind(kind);
            }
        }
    }

    fn document_picker(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("header-document")
            .width(112.0)
            .truncate()
            .selected_text("Dokumen")
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show_ui(ui, |ui| {
                ui.add(
                    TextEdit::singleline(&mut self.query)
                        .id(egui::Id::new("document-search"))
                        .hint_text("Cari dokumen…")
                        .desired_width(220.0),
                );
                let query = self.query.to_lowercase();
                let docs: Vec<_> = self
                    .workspace
                    .documents
                    .iter()
                    .filter(|d| d.kind == self.kind)
                    .filter(|d| {
                        query.is_empty()
                            || d.title.to_lowercase().contains(&query)
                            || d.body.text.to_lowercase().contains(&query)
                            || d.tasks
                                .iter()
                                .any(|t| t.text.to_lowercase().contains(&query))
                    })
                    .map(|d| (d.id, d.title.clone()))
                    .collect();
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for (id, title) in docs {
                            if ui
                                .selectable_label(
                                    self.selected == Some(id),
                                    if title.trim().is_empty() {
                                        "Tanpa judul"
                                    } else {
                                        &title
                                    },
                                )
                                .clicked()
                            {
                                self.flush();
                                self.select(Some(id));
                                if self.kind == Kind::Note {
                                    self.notes_board.overview = false;
                                } else if self.kind == Kind::Checklist {
                                    self.checklist_board.overview = false;
                                }
                                ui.close();
                            }
                        }
                    });
            });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let overview = match self.kind {
            Kind::Note => self.notes_board.overview,
            Kind::Checklist => self.checklist_board.overview,
            _ => false,
        };
        if overview {
            let action = if self.kind == Kind::Note {
                self.notes_board.show(ui, &mut self.workspace.documents)
            } else {
                self.checklist_board
                    .show_kind(ui, &mut self.workspace.documents, Kind::Checklist)
            };
            if action.changed {
                self.change();
            }
            if let Some(id) = action.open {
                self.flush();
                self.select(Some(id));
                self.notes_board.overview = false;
                self.checklist_board.overview = false;
            }
            if action.create {
                self.create();
            }
            if let Some(id) = action.delete {
                self.flush();
                self.select(Some(id));
                self.delete_document = true;
            }
            return;
        }
        let Some(index) = self
            .workspace
            .documents
            .iter()
            .position(|d| Some(d.id) == self.selected)
        else {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.27);
                if ui
                    .add_sized(
                        [170.0, 40.0],
                        crate::controls::button(ui, &format!("+ Buat {}", self.kind.label())),
                    )
                    .clicked()
                {
                    self.create();
                }
            });
            return;
        };
        if self.kind == Kind::Note {
            self.note(ui, index);
            return;
        }
        self.document_heading(ui, index);
        match self.kind {
            Kind::Canvas => {
                let doc = &mut self.workspace.documents[index];
                let changed = self.canvas_editor.show(ui, &mut doc.canvas, &mut doc.title);
                if changed {
                    self.change();
                }
                if let Some(error) = self.canvas_editor.error.take() {
                    self.error = Some(error);
                }
            }
            Kind::Note => unreachable!(),
            Kind::Checklist => self.checklist(ui, index),
            Kind::Kanban => self.kanban(ui, index),
        }
    }

    fn document_heading(&mut self, ui: &mut egui::Ui, index: usize) {
        if matches!(self.kind, Kind::Note | Kind::Checklist) {
            let compact = ui.available_width() < 520.0;
            egui::Grid::new(if self.kind == Kind::Note {
                "note-actions"
            } else {
                "checklist-actions"
            })
            .num_columns(if compact { 2 } else { 4 })
            .show(ui, |ui| {
                if ui
                    .add(crate::controls::button(ui, "Kembali ke papan"))
                    .clicked()
                {
                    self.flush();
                    if self.kind == Kind::Note {
                        self.notes_board.overview = true;
                    } else {
                        self.checklist_board.overview = true;
                    }
                }
                if ui.add(crate::controls::button(ui, "Hapus")).clicked() {
                    self.delete_document = true;
                }
                if compact {
                    ui.end_row();
                }
                {
                    let doc = &mut self.workspace.documents[index];
                    let mut changed = false;
                    crate::controls::menu(ui, "Warna", |ui| {
                        changed = notes_board::note_palette(ui, &mut doc.note_color);
                    });
                    if changed {
                        self.change_immediately();
                    }
                }
                if self.kind == Kind::Note {
                    crate::controls::menu(ui, "Ekspor", |ui| {
                        if ui
                            .add(crate::controls::button(ui, "HTML (dengan format)"))
                            .clicked()
                        {
                            ui.close();
                            self.export(true);
                        }
                        if ui.add(crate::controls::button(ui, "Teks (.txt)")).clicked() {
                            ui.close();
                            self.export(false);
                        }
                    });
                }
                ui.end_row();
            });
        }
        if self.kind != Kind::Canvas {
            ui.add_space(8.0);
            if ui
                .add(
                    TextEdit::singleline(&mut self.workspace.documents[index].title)
                        .font(FontId::new(
                            if ui.available_width() < 640.0 {
                                24.0
                            } else {
                                30.0
                            },
                            FontFamily::Name("hand".into()),
                        ))
                        .frame(false)
                        .hint_text("Tanpa judul")
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                self.change();
            }
            ui.add_space(10.0);
        }
    }

    fn note(&mut self, ui: &mut egui::Ui, index: usize) {
        let [r, g, b] = self.workspace.documents[index].note_color;
        let paper = Color32::from_rgb(r, g, b);
        let header_color = note_header_color([r, g, b]);
        let wrap = egui::Frame::new()
            .fill(paper)
            .corner_radius(SHELL_RADIUS)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(ui.available_height());
                let mut format_changed = false;
                let header = egui::Frame::new()
                    .fill(header_color)
                    .corner_radius(egui::CornerRadius {
                        nw: SHELL_RADIUS,
                        ne: SHELL_RADIUS,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        note_header_style(ui, header_color);
                        self.document_heading(ui, index);
                        format_changed = self.note_toolbar(ui, index);
                    });
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new("note-header-bounds"), header.response.rect)
                });
                egui::Frame::new()
                    .inner_margin(16.0)
                    .show(ui, |ui| self.note_body(ui, index, format_changed));
            });
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("note-paper-bounds"), wrap.response.rect)
        });
    }

    fn note_toolbar(&mut self, ui: &mut egui::Ui, index: usize) -> bool {
        let previous = self.typing;
        let mut format_changed = false;
        let mut insert_image = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().button_padding.x = 6.0;
            ui.spacing_mut().item_spacing.x = 6.0;
            format_changed |= ui
                .selectable_label(self.typing.bold, RichText::new("B").strong())
                .on_hover_text("Tebal · Ctrl+B")
                .clicked()
                .then(|| self.typing.bold = !self.typing.bold)
                .is_some();
            format_changed |= ui
                .selectable_label(self.typing.italic, RichText::new("I").italics())
                .on_hover_text("Miring · Ctrl+I")
                .clicked()
                .then(|| self.typing.italic = !self.typing.italic)
                .is_some();
            format_changed |= ui
                .selectable_label(self.typing.underline, RichText::new("U").underline())
                .on_hover_text("Garis bawah · Ctrl+U")
                .clicked()
                .then(|| self.typing.underline = !self.typing.underline)
                .is_some();
            ui.separator();
            ui.label("Ukuran");
            egui::ComboBox::from_id_salt("font-size")
                .width(55.0)
                .selected_text(format!("{}", self.typing.size))
                .show_ui(ui, |ui| {
                    for size in [12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 40.0, 48.0] {
                        format_changed |= ui
                            .selectable_value(&mut self.typing.size, size, format!("{size}"))
                            .changed();
                    }
                });
            ui.label("Jarak baris");
            let spacing = &mut self.workspace.documents[index].spacing;
            let old = *spacing;
            egui::ComboBox::from_id_salt("line-spacing")
                .width(50.0)
                .selected_text(format!("{spacing:.1}"))
                .show_ui(ui, |ui| {
                    for value in [1.0, 1.3, 1.5, 1.8, 2.0, 2.5] {
                        ui.selectable_value(spacing, value, format!("{value:.1}"));
                    }
                });
            if *spacing != old {
                self.change_immediately();
            }
            ui.separator();
            if ui
                .add_enabled(!self.undo.is_empty(), crate::controls::button(ui, "Undo"))
                .clicked()
            {
                self.history(false);
            }
            if ui
                .add_enabled(!self.redo.is_empty(), crate::controls::button(ui, "Redo"))
                .clicked()
            {
                self.history(true);
            }
            insert_image = ui.add(crate::controls::button(ui, "Gambar...")).clicked();
        });
        if format_changed {
            self.apply_format(previous);
        }
        if insert_image
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Gambar", &["png", "jpg", "jpeg"])
                .pick_file()
        {
            self.insert_note_image(index, &path, ui.ctx());
        }
        let before_images = self.workspace.documents[index].body.clone();
        if self
            .note_images
            .toolbar(ui, &mut self.workspace.documents[index].body)
        {
            self.remember(before_images);
            self.change_immediately();
        }
        format_changed
    }

    fn note_body(&mut self, ui: &mut egui::Ui, index: usize, format_changed: bool) {
        let doc = &self.workspace.documents[index];
        let before = doc.body.clone();
        let mut text = before.text.clone();
        let typing = self.typing;
        let spacing = doc.spacing;
        let doc_id = doc.id;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, width: f32| {
            let mut styled = before.clone();
            styled.reconcile(buffer.as_str().into(), typing);
            ui.fonts_mut(|fonts| fonts.layout_job(layout_job(&styled, width, spacing)))
        };
        let mut output = None;
        let mut images_changed = false;
        let mut drop_at = None;
        let body = &mut self.workspace.documents[index].body;
        egui::ScrollArea::vertical()
            .id_salt(("note-scroll", doc_id))
            .show_pannable(ui, [false, true], |ui| {
                let available = (ui
                    .available_width()
                    .min(ui.clip_rect().right() - ui.cursor().left())
                    - 8.0)
                    .max(1.0);
                let edit = TextEdit::multiline(&mut text)
                    .id(egui::Id::new("note-body"))
                    .font(FontId::proportional(18.0))
                    .desired_width(available)
                    .desired_rows(22)
                    .frame(false)
                    .hint_text("Tulis sesuatu di sini...")
                    .layouter(&mut layouter)
                    .show(ui);
                if edit.response.changed() {
                    body.reconcile(text.clone(), typing);
                }
                if ui.input(|i| !i.raw.dropped_files.is_empty())
                    && ui
                        .input(|i| i.pointer.hover_pos())
                        .is_some_and(|p| ui.clip_rect().contains(navigation::local_pointer(ui, p)))
                {
                    drop_at = ui.input(|i| i.pointer.hover_pos()).map(|p| {
                        edit.galley
                            .cursor_from_pos(navigation::local_pointer(ui, p) - edit.galley_pos)
                            .index
                    });
                }
                images_changed = self.note_images.show(ui, body, &edit, available);
                output = Some(edit);
            });
        if let Some(output) = output {
            if output.response.changed() || images_changed {
                if !self.note_images.dragging() {
                    self.remember(before);
                }
                self.change();
            }
            if let Some(range) = output.cursor_range {
                let sorted = range.as_sorted_char_range();
                if sorted != self.selection && !format_changed {
                    let styles = &self.workspace.documents[index].body.styles;
                    let position = if sorted.is_empty() {
                        sorted.start.saturating_sub(1)
                    } else {
                        sorted.start
                    };
                    if let Some(style) = styles.get(position) {
                        self.typing = *style;
                    }
                }
                self.selection = sorted;
            }
        }
        if let Some(before) = self
            .note_images
            .finish_gesture(ui, &self.workspace.documents[index].body)
        {
            self.remember(before);
        }
        if let Some(at) = drop_at {
            self.selection = at..at;
            let files = ui.input(|i| i.raw.dropped_files.clone());
            for file in files {
                if let Some(path) = file.path {
                    self.insert_note_image(index, &path, ui.ctx());
                }
            }
        }
    }

    fn insert_note_image(&mut self, index: usize, path: &std::path::Path, ctx: &egui::Context) {
        match canvas::model::ImageData::from_path(path) {
            Ok(data) => {
                let body = &mut self.workspace.documents[index].body;
                let before = body.clone();
                let id = body.insert_image(
                    self.selection.start,
                    data,
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    self.typing,
                );
                let cursor = body
                    .images
                    .iter()
                    .find(|image| image.id == id)
                    .unwrap()
                    .anchor
                    + 2;
                self.selection = cursor..cursor;
                self.note_images.selected = Some(id);
                let text_id = egui::Id::new("note-body");
                if let Some(mut state) = egui::TextEdit::load_state(ctx, text_id) {
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(cursor),
                        )));
                    state.store(ctx, text_id);
                }
                self.remember(before);
                self.change_immediately();
            }
            Err(error) => self.error = Some(format!("Gagal memasukkan gambar: {error}")),
        }
    }

    fn task_input(&mut self, ui: &mut egui::Ui, index: usize) {
        let mut add = false;
        egui::Frame::new()
            .fill(Color32::WHITE)
            .corner_radius(10)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [(ui.available_width() - 96.0).max(24.0), 44.0],
                        TextEdit::singleline(&mut self.new_task)
                            .frame(false)
                            .vertical_align(egui::Align::Center)
                            .hint_text("Tulis tugas baru..."),
                    );
                    add = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    add |= ui
                        .add_enabled(
                            !self.new_task.trim().is_empty(),
                            crate::controls::button(ui, "Tambah").min_size(egui::vec2(88.0, 44.0)),
                        )
                        .clicked();
                    if add {
                        response.request_focus();
                    }
                });
            });
        if add && !self.new_task.trim().is_empty() {
            let id = self.workspace.id();
            self.workspace.documents[index].tasks.push(Task {
                id,
                text: self.new_task.trim().into(),
                detail: String::new(),
                done: false,
                column: 0,
            });
            self.new_task.clear();
            self.change_immediately();
        }
    }

    fn checklist(&mut self, ui: &mut egui::Ui, index: usize) {
        let tasks = &self.workspace.documents[index].tasks;
        let done = tasks.iter().filter(|t| t.done).count();
        ui.label(RichText::new(format!("{done} dari {} tugas selesai", tasks.len())).color(MUTED));
        ui.add_space(14.0);
        self.task_input(ui, index);
        ui.add_space(18.0);
        let mut changed = false;
        let mut immediate = false;
        let mut delete = None;
        egui::ScrollArea::vertical()
            .id_salt("checklist-scroll")
            .auto_shrink([false, false])
            .show_pannable(ui, [false, true], |ui| {
                let tasks = &mut self.workspace.documents[index].tasks;
                for task in tasks {
                    ui.push_id(task.id, |ui| {
                        let card = egui::Frame::new()
                            .fill(Color32::WHITE)
                            .corner_radius(10)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                let width = ui.available_width();
                                let compact = width < 420.0;
                                let delete_width = if compact { 44.0 } else { 90.0 };
                                let text_width = (width
                                    - 44.0
                                    - delete_width
                                    - ui.spacing().item_spacing.x * 2.0)
                                    .max(24.0);
                                ui.horizontal(|ui| {
                                    let checkbox =
                                        controls::task_checkbox(ui, &mut task.done, &task.text);
                                    immediate |= checkbox.changed();
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(
                                            egui::Id::new(("checklist-check", task.id)),
                                            checkbox.id,
                                        )
                                    });
                                    let done = task.done;
                                    let ink = if done { MUTED } else { TEXT };
                                    let mut layouter =
                                        |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _: f32| {
                                            let mut job = egui::text::LayoutJob::simple_singleline(
                                                buffer.as_str().into(),
                                                FontId::proportional(16.0),
                                                ink,
                                            );
                                            if done {
                                                for section in &mut job.sections {
                                                    section.format.strikethrough =
                                                        Stroke::new(1.2_f32, ink);
                                                }
                                            }
                                            ui.fonts_mut(|fonts| fonts.layout_job(job))
                                        };
                                    changed |= ui
                                        .add_sized(
                                            [text_width, 44.0],
                                            TextEdit::singleline(&mut task.text)
                                                .id(egui::Id::new(("checklist-text", task.id)))
                                                .font(FontId::proportional(16.0))
                                                .text_color(ink)
                                                .vertical_align(egui::Align::Center)
                                                .frame(false)
                                                .layouter(&mut layouter),
                                        )
                                        .changed();
                                    let delete_button = if compact {
                                        controls::button_with_text(ui, "Hapus", "")
                                    } else {
                                        controls::button(ui, "Hapus")
                                    };
                                    let response = ui
                                        .add_sized([delete_width, 44.0], delete_button)
                                        .on_hover_text("Hapus tugas");
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            ui.is_enabled(),
                                            "Hapus tugas",
                                        )
                                    });
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(
                                            egui::Id::new(("checklist-delete", task.id)),
                                            response.id,
                                        )
                                    });
                                    if response.clicked() {
                                        delete = Some(task.id);
                                    }
                                });
                            });
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(
                                egui::Id::new(("checklist-row", task.id)),
                                card.response.rect,
                            )
                        });
                        ui.add_space(4.0);
                    });
                }
            });
        if let Some(id) = delete {
            self.workspace.documents[index].tasks.retain(|t| t.id != id);
            immediate = true;
        }
        if immediate {
            self.change_immediately();
        } else if changed {
            self.change();
        }
    }

    fn kanban(&mut self, ui: &mut egui::Ui, index: usize) {
        self.task_input(ui, index);
        ui.add_space(20.0);
        let titles = ["Rencana", "Dikerjakan", "Selesai"];
        let mut changed = false;
        let mut delete = None;
        let mut moved = None;
        egui::ScrollArea::horizontal()
            .id_salt("board-horizontal")
            .show_pannable(ui, [true, false], |ui| {
                ui.set_min_width(660.0_f32.max(ui.available_width()));
                ui.columns(3, |columns| {
                    for (column, ui) in columns.iter_mut().enumerate() {
                        let frame = egui::Frame::new()
                            .fill(PANEL)
                            .corner_radius(6)
                            .inner_margin(12.0);
                        let (_, payload) = ui.dnd_drop_zone::<u64, _>(frame, |ui| {
                            ui.set_min_height(ui.available_height().min(340.0));
                            let count = self.workspace.documents[index]
                                .tasks
                                .iter()
                                .filter(|t| t.column == column)
                                .count();
                            ui.label(
                                RichText::new(format!("{}   {count}", titles[column]))
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(14.0);
                            egui::ScrollArea::vertical()
                                .id_salt(("column", column))
                                .max_height(ui.available_height())
                                .show_pannable(ui, [false, true], |ui| {
                                    for task in self.workspace.documents[index]
                                        .tasks
                                        .iter_mut()
                                        .filter(|t| t.column == column)
                                    {
                                        ui.push_id(task.id, |ui| {
                                            egui::Frame::new()
                                                .fill(CARD)
                                                .corner_radius(5)
                                                .inner_margin(10.0)
                                                .show(ui, |ui| {
                                                    let navigating = ui.input(|i| i.multi_touch().is_some()
                                                        || (!i.pointer.primary_down() && (i.pointer.secondary_down() || i.pointer.button_released(egui::PointerButton::Secondary))));
                                                    if navigating {
                                                        crate::controls::drag_handle(ui);
                                                    } else { ui.dnd_drag_source(
                                                        egui::Id::new(("drag", task.id)),
                                                        task.id,
                                                        |ui| {
                                                            crate::controls::drag_handle(ui);
                                                        },
                                                    ); }
                                                    changed |= ui
                                                        .add(
                                                            TextEdit::multiline(&mut task.text)
                                                                .desired_width(f32::INFINITY)
                                                                .desired_rows(2)
                                                                .frame(false),
                                                        )
                                                        .changed();
                                                    changed |= ui
                                                        .add(
                                                            TextEdit::multiline(&mut task.detail)
                                                                .hint_text("Tambahkan rincian...")
                                                                .desired_width(f32::INFINITY)
                                                                .desired_rows(2)
                                                                .frame(false)
                                                                .font(FontId::proportional(12.0)),
                                                        )
                                                        .changed();
                                                    ui.add_space(8.0);
                                                    let mut target_column = task.column;
                                                    egui::ComboBox::from_id_salt("status")
                                                        .selected_text(titles[task.column])
                                                        .width(100.0)
                                                        .show_ui(ui, |ui| {
                                                            for (i, title) in
                                                                titles.iter().enumerate()
                                                            {
                                                                ui.selectable_value(
                                                                    &mut target_column,
                                                                    i,
                                                                    *title,
                                                                );
                                                            }
                                                        });
                                                    if target_column != task.column {
                                                        moved = Some((task.id, target_column));
                                                    }
                                                    if ui
                                                        .add(
                                                            crate::controls::button(
                                                                ui,
                                                                "Hapus kartu",
                                                            )
                                                            .small(),
                                                        )
                                                        .clicked()
                                                    {
                                                        delete = Some(task.id);
                                                    }
                                                });
                                            ui.add_space(10.0);
                                        });
                                    }
                                });
                        });
                        if let Some(id) = payload {
                            moved = Some((*id, column));
                        }
                    }
                });
            });
        let immediate = moved.is_some() || delete.is_some();
        if let Some((id, column)) = moved {
            if let Some(task) = self.workspace.documents[index]
                .tasks
                .iter_mut()
                .find(|t| t.id == id)
            {
                task.column = column;
                changed = true;
            }
        }
        if let Some(id) = delete {
            self.workspace.documents[index].tasks.retain(|t| t.id != id);
            changed = true;
        }
        if immediate {
            self.change_immediately();
        } else if changed {
            self.change();
        }
    }
}

impl Notvas {
    fn draw(&mut self, ctx: &egui::Context) {
        ctx.layer_painter(egui::LayerId::background())
            .rect_filled(ctx.content_rect(), 0.0, PANEL);
        window_chrome::resize_edges(ctx);
        if let Some(path) = self.restart_path.clone() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            if path.exists() && (!self.autosave.retrying() || self.autosave.due(Instant::now())) {
                if self.flush() {
                    let _ = std::fs::remove_file(path);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.pending_close = true;
                }
            }
        }
        if !self.delete_document && !self.pending_close {
            self.shortcuts(ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.flush() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_close = true;
        }
        egui::TopBottomPanel::bottom("status")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .outer_margin(egui::Margin { left: SHELL_INSET, right: SHELL_INSET, top: 0, bottom: SHELL_INSET })
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().interact_size.y = 18.0;
                ui.spacing_mut().button_padding = egui::vec2(6.0, 1.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.horizontal(|ui| {
                    if self.kind != Kind::Canvas {
                        let slot = match self.kind { Kind::Note => 0, Kind::Checklist => 1, Kind::Kanban => 2, Kind::Canvas => unreachable!() };
                        if ui.small_button(format!("{:.0}%", self.content_zoom[slot] * 100.0))
                            .on_hover_text("Reset zoom · Ctrl+0\nCtrl+scroll untuk memperbesar atau memperkecil")
                            .clicked() {
                            self.content_zoom[slot] = 1.0;
                        }
                    }
                    let status = if self.load_failed {
                            "Autosave memerlukan folder penyimpanan lain"
                        } else if self.dirty && self.autosave.retrying() {
                            "Belum tersimpan. Mencoba lagi otomatis..."
                        } else if self.dirty {
                            "Menyimpan otomatis..."
                        } else {
                            &self.status
                        };
                    ui.add(egui::Label::new(RichText::new(status)
                        .size(11.0)
                        .color(MUTED)).truncate()).on_hover_text(status);
                    if ui.available_width() > 220.0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(doc) = self
                                .workspace
                                .documents
                                .iter()
                                .find(|d| Some(d.id) == self.selected)
                            {
                                if doc.kind == Kind::Note {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} kata  ·  {} karakter",
                                            doc.body.text.split_whitespace().count(),
                                            doc.body.text.chars().count()
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    );
                                }
                            }
                        });
                    }
                });
            });
        self.header(ctx);
        let shell = ctx.available_rect().shrink(f32::from(SHELL_INSET));
        let note_paper = if self.kind == Kind::Note && !self.notes_board.overview {
            self.workspace
                .documents
                .iter()
                .find(|doc| Some(doc.id) == self.selected)
                .map(|doc| {
                    Color32::from_rgb(doc.note_color[0], doc.note_color[1], doc.note_color[2])
                })
        } else {
            None
        };
        let main = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(note_paper.unwrap_or(Color32::WHITE))
                    .outer_margin(SHELL_INSET)
                    .corner_radius(SHELL_RADIUS)
                    .inner_margin(if note_paper.is_some() { 0 } else { 8 }),
            )
            .show(ctx, |ui| {
                if self.kind != Kind::Canvas && note_paper.is_none() {
                    paint_grid(ui.painter(), ui.max_rect());
                }
                if let Some(error) = self.error.clone() {
                    egui::Frame::new()
                        .fill(CARD)
                        .stroke(Stroke::new(1.0_f32, MUTED))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(error).color(TEXT));
                            if self.dirty || self.load_failed {
                                if !self.load_failed {
                                    ui.label(
                                        RichText::new("Autosave akan mencoba lagi otomatis.")
                                            .size(12.0)
                                            .color(MUTED),
                                    );
                                }
                                if ui
                                    .add(crate::controls::button(
                                        ui,
                                        "Pilih folder penyimpanan lain...",
                                    ))
                                    .clicked()
                                {
                                    self.choose_storage_folder();
                                }
                            } else if ui.add(crate::controls::button(ui, "Tutup pesan")).clicked() {
                                self.error = None;
                            }
                        });
                    ui.add_space(12.0);
                }
                if self.kind == Kind::Canvas {
                    self.editor(ui);
                } else {
                    let slot = match self.kind {
                        Kind::Note => 0,
                        Kind::Checklist => 1,
                        Kind::Kanban => 2,
                        Kind::Canvas => unreachable!(),
                    };
                    let mut factor = self.content_zoom[slot];
                    zoom::show(ui, &mut factor, |ui| self.editor(ui));
                    self.content_zoom[slot] = factor;
                }
            });
        let painter = ctx.layer_painter(main.response.layer_id);
        let border = Stroke::new(0.8_f32, SHELL_BORDER);
        painter.rect_stroke(shell, SHELL_RADIUS, border, egui::StrokeKind::Inside);
        painter.rect_stroke(
            ctx.content_rect().shrink(0.5),
            SHELL_RADIUS,
            Stroke::new(0.8_f32, OUTER_BORDER),
            egui::StrokeKind::Inside,
        );
        if self.delete_document {
            let modal = egui::Modal::new(egui::Id::new("delete-document")).show(ctx, |ui| {
                ui.heading("Hapus dokumen?");
                ui.label("Dokumen beserta isinya akan dihapus dari workspace.");
                ui.horizontal(|ui| {
                    if ui.add(crate::controls::button(ui, "Batal")).clicked() {
                        self.delete_document = false;
                    }
                    if ui
                        .add(crate::controls::button(ui, "Hapus dokumen"))
                        .clicked()
                    {
                        self.workspace
                            .documents
                            .retain(|d| Some(d.id) != self.selected);
                        self.select(
                            self.workspace
                                .documents
                                .iter()
                                .find(|d| d.kind == self.kind)
                                .map(|d| d.id),
                        );
                        self.delete_document = false;
                        self.change_immediately();
                    }
                });
            });
            if modal.should_close() {
                self.delete_document = false;
            }
        }
        if self.pending_close {
            let modal = egui::Modal::new(egui::Id::new("close-failed")).show(ctx, |ui| {
                ui.heading("Perubahan belum tersimpan");
                ui.label("Autosave belum berhasil. Aplikasi akan mencoba lagi otomatis.");
                if ui
                    .add(crate::controls::button(
                        ui,
                        "Pilih folder penyimpanan lain...",
                    ))
                    .clicked()
                {
                    self.choose_storage_folder();
                    if !self.dirty {
                        self.pending_close = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                if ui
                    .add(crate::controls::button(ui, "Kembali menulis"))
                    .clicked()
                {
                    self.pending_close = false;
                }
            });
            if modal.should_close() {
                self.pending_close = false;
            }
        }
        let focused = ctx.input(|i| i.focused);
        if self.was_focused && !focused {
            self.flush();
        }
        self.was_focused = focused;
        let now = Instant::now();
        let save_due = self.dirty && !self.load_failed && self.autosave.due(now);
        self.autosave_if_due(now);
        if save_due {
            ctx.request_repaint();
        }
        if self.pending_close && !self.dirty {
            self.pending_close = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.dirty && !self.load_failed {
            if let Some(deadline) = self.autosave.deadline() {
                ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
        }
    }
}

fn layout_job(doc: &RichDocument, width: f32, spacing: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let mut run = String::new();
    let mut current = doc.styles.first().copied().unwrap_or_default();
    for (i, ch) in doc.text.chars().enumerate() {
        let style = doc.styles.get(i).copied().unwrap_or_default();
        if ch == model::IMAGE_MARKER {
            if !run.is_empty() {
                append_run(&mut job, &run, current, spacing);
                run.clear();
            }
            let image = doc.images.iter().find(|image| image.anchor == i);
            let height = image
                .filter(|image| image.layout == model::ImageLayout::Text)
                .map_or(style.size * spacing, |image| {
                    image.display_size(width).y + 12.0
                });
            job.append(
                &ch.to_string(),
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(1.0),
                    color: Color32::TRANSPARENT,
                    line_height: Some(height),
                    extra_letter_spacing: if image
                        .is_some_and(|image| image.layout == model::ImageLayout::Text)
                    {
                        width.max(1.0) - 2.0
                    } else {
                        0.0
                    },
                    ..Default::default()
                },
            );
            continue;
        }
        if style != current && !run.is_empty() {
            append_run(&mut job, &run, current, spacing);
            run.clear();
        }
        current = style;
        run.push(ch);
    }
    if !run.is_empty() {
        append_run(&mut job, &run, current, spacing);
    }
    job
}

fn append_run(job: &mut egui::text::LayoutJob, text: &str, style: TextStyle, spacing: f32) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: FontId::new(
                style.size,
                if style.bold {
                    FontFamily::Name("bold".into())
                } else {
                    FontFamily::Proportional
                },
            ),
            color: TEXT,
            italics: style.italic,
            underline: if style.underline {
                Stroke::new(1.0_f32, TEXT)
            } else {
                Stroke::NONE
            },
            line_height: Some(style.size * spacing),
            ..Default::default()
        },
    );
}

fn paint_grid(painter: &egui::Painter, rect: egui::Rect) {
    let rect = rect.intersect(painter.clip_rect());
    if !rect.is_positive() {
        return;
    }
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    let stroke = Stroke::new(0.65_f32, Color32::from_gray(228));
    let mut x = (rect.left() / 24.0).ceil() * 24.0;
    while x < rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
        x += 24.0;
    }
    let mut y = (rect.top() / 24.0).ceil() * 24.0;
    while y < rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            stroke,
        );
        y += 24.0;
    }
}

fn configure(ctx: &egui::Context) {
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "patrick-hand".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/PatrickHand-Regular.ttf"))
            .into(),
    );
    fonts
        .families
        .insert(FontFamily::Name("hand".into()), vec!["patrick-hand".into()]);
    for (name, path, family) in [
        (
            "notvas-regular",
            "C:/Windows/Fonts/segoeui.ttf",
            FontFamily::Proportional,
        ),
        (
            "notvas-bold",
            "C:/Windows/Fonts/segoeuib.ttf",
            FontFamily::Name("bold".into()),
        ),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.into());
        }
    }
    if !fonts
        .families
        .contains_key(&FontFamily::Name("bold".into()))
    {
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            fonts.families[&FontFamily::Proportional].clone(),
        );
    }
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = PANEL;
    style.visuals.faint_bg_color = CARD;
    style.visuals.selection.bg_fill = BUTTON_ACTIVE;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, TEXT);
    style.visuals.widgets.inactive.bg_fill = BUTTON;
    style.visuals.widgets.inactive.weak_bg_fill = BUTTON;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(0.7_f32, BUTTON_BORDER);
    style.visuals.widgets.hovered.bg_fill = BUTTON_HOVER;
    style.visuals.widgets.hovered.weak_bg_fill = BUTTON_HOVER;
    style.visuals.widgets.active.bg_fill = BUTTON_ACTIVE;
    style.visuals.widgets.active.weak_bg_fill = BUTTON_ACTIVE;
    style.visuals.widgets.active.bg_stroke = Stroke::new(0.8_f32, Color32::from_gray(180));
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(0.7_f32, Color32::from_gray(200));
    style.visuals.widgets.open.bg_fill = BUTTON_ACTIVE;
    style.visuals.widgets.open.weak_bg_fill = BUTTON_ACTIVE;
    style.visuals.widgets.open.bg_stroke = Stroke::new(0.8_f32, Color32::from_gray(180));
    for visual in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        visual.corner_radius = egui::CornerRadius::same(8);
    }
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.warn_fg_color = TEXT;
    style.visuals.error_fg_color = TEXT;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    ctx.set_style(style);
}

impl eframe::App for Notvas {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(PANEL).to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw(ctx);
    }
}

fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/notvas-icon-256.png"))
        .expect("Bundled Notvas icon must be a valid PNG")
}

fn main() -> eframe::Result {
    eframe::run_native(
        if cfg!(debug_assertions) {
            "Notvas (Development)"
        } else {
            "Notvas"
        },
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_icon(app_icon())
                .with_inner_size([1280.0, 820.0])
                .with_min_inner_size([380.0, 520.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Notvas::new(cc)))),
    )
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    pub(super) fn app() -> Notvas {
        Notvas {
            content_zoom: [1.0; 3],
            note_images: note_images::NoteImages::default(),
            canvas_editor: canvas::CanvasEditor::default(),
            notes_board: notes_board::NotesBoard::default(),
            checklist_board: notes_board::NotesBoard::default(),
            restart_path: None,
            logo: None,
            workspace: Workspace::default(),
            path: PathBuf::new(),
            kind: Kind::Note,
            selected: None,
            query: String::new(),
            typing: TextStyle::default(),
            selection: 0..0,
            new_task: String::new(),
            dirty: false,
            autosave: Autosave::default(),
            status: String::new(),
            error: None,
            load_failed: true,
            delete_document: false,
            pending_close: false,
            was_focused: true,
            undo: vec![],
            redo: vec![],
        }
    }

    fn frame(app: &mut Notvas, ctx: &egui::Context, size: [f32; 2]) {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        );
        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
        assert!(!meshes.is_empty());
        for primitive in meshes {
            assert!(primitive.clip_rect.is_finite());
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                assert!(mesh.vertices.iter().all(|vertex| vertex.pos.is_finite()));
            }
        }
    }

    #[test]
    fn all_modes_empty_populated_and_dialogs_generate_valid_frames() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        for size in [
            [380.0, 520.0],
            [640.0, 620.0],
            [800.0, 620.0],
            [940.0, 620.0],
            [1280.0, 820.0],
        ] {
            for kind in [Kind::Canvas, Kind::Note, Kind::Checklist, Kind::Kanban] {
                app.kind = kind;
                app.select(None);
                frame(&mut app, &ctx, size);
                app.create();
                let doc = app.workspace.documents.last_mut().unwrap();
                doc.body.reconcile(
                    "Catatan dengan format dan Unicode 猫".into(),
                    TextStyle {
                        size: 32.0,
                        bold: true,
                        underline: true,
                        ..Default::default()
                    },
                );
                doc.body.apply(0..7, TextStyle::default());
                for column in 0..3 {
                    let id = app.workspace.id();
                    app.workspace
                        .documents
                        .last_mut()
                        .unwrap()
                        .tasks
                        .push(Task {
                            id,
                            text: "Tugas contoh pengujian".into(),
                            detail: "Rincian pengujian".into(),
                            done: column == 2,
                            column,
                        });
                }
                frame(&mut app, &ctx, size);
                frame(&mut app, &ctx, size);
            }
        }
        app.delete_document = true;
        frame(&mut app, &ctx, [940.0, 620.0]);
        app.delete_document = false;
        app.pending_close = true;
        app.error = Some("Kesalahan penyimpanan untuk pengujian".into());
        frame(&mut app, &ctx, [940.0, 620.0]);
    }

    #[test]
    fn navigation_and_canvas_controls_remain_visible_when_resizing() {
        fn inspect(
            shape: &egui::Shape,
            clip: egui::Rect,
            screen: egui::Rect,
            found: &mut std::collections::HashSet<String>,
        ) {
            match shape {
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        inspect(shape, clip, screen, found);
                    }
                }
                egui::Shape::Text(text) => {
                    if text
                        .galley
                        .job
                        .sections
                        .first()
                        .is_some_and(|s| s.format.font_id.family == FontFamily::Name("hand".into()))
                    {
                        return;
                    }
                    let label = text.galley.job.text.as_str();
                    if [
                        "Canvas",
                        "Notes",
                        "Checklist",
                        "Kanban",
                        "Workspace",
                        "Pilih",
                        "Pena",
                        "Alat lain",
                        "Catatan baru",
                        "Rapikan posisi",
                    ]
                    .contains(&label)
                    {
                        let rect = text.visual_bounding_rect();
                        assert!(
                            clip.expand(1.0).contains_rect(rect),
                            "{label} clipped: {rect:?}, clip {clip:?}"
                        );
                        assert!(
                            screen.contains_rect(rect),
                            "{label} outside {screen:?}: {rect:?}"
                        );
                        found.insert(label.to_owned());
                    }
                }
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        for size in [
            [1280.0, 820.0],
            [1920.0, 1080.0],
            [380.0, 520.0],
            [640.0, 620.0],
            [800.0, 620.0],
            [940.0, 620.0],
        ] {
            for kind in [Kind::Canvas, Kind::Note] {
                app.kind = kind;
                app.create();
                app.notes_board.overview = true;
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size.into());
                frame(&mut app, &ctx, size);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                );
                let mut found = std::collections::HashSet::new();
                let wrapper = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.stroke.color == SHELL_BORDER
                                && rect.fill == Color32::TRANSPARENT =>
                        {
                            Some(rect)
                        }
                        _ => None,
                    })
                    .expect("Main content must have its own enclosing border");
                let wrapped_content = wrapper.rect;
                let header =
                    egui::containers::panel::PanelState::load(&ctx, egui::Id::new("navigation"))
                        .unwrap()
                        .rect;
                let close = ctx
                    .read_response(egui::Id::new(("window-control", "Tutup")))
                    .unwrap()
                    .rect;
                let minimize = ctx
                    .read_response(egui::Id::new(("window-control", "Minimalkan")))
                    .unwrap()
                    .rect;
                assert!(
                    screen.contains_rect(header),
                    "Header overflow at {size:?}: {header:?}"
                );
                assert!(
                    (close.right() - (header.right() - 10.0)).abs() < 1.0,
                    "Window controls must stay at the right edge at {size:?}: {close:?}, header {header:?}"
                );
                assert!(header.contains_rect(close) && header.contains_rect(minimize));
                for label in ["File", "Edit", "Tampilan", "Workspace", "Bantuan"] {
                    let menu = ctx
                        .data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("header-menu", label)))
                        })
                        .unwrap();
                    assert!(
                        header.contains_rect(menu),
                        "Menu {label} outside header at {size:?}: {menu:?}"
                    );
                    let font = egui::TextStyle::Button.resolve(&ctx.style());
                    let text_width = ctx
                        .fonts_mut(|fonts| fonts.layout_no_wrap(label.into(), font, TEXT).size().x);
                    assert!(
                        menu.width() >= text_width + 11.5,
                        "Menu {label} needs horizontal padding at {size:?}: {menu:?}"
                    );
                }
                for label in [
                    "+ Baru", "Hapus…", "Undo", "Redo", "Zoom out", "Zoom in", "Fit",
                ] {
                    let button = ctx
                        .data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("header-action", label)))
                        })
                        .unwrap();
                    assert!(
                        header.contains_rect(button),
                        "Action {label} outside header at {size:?}: {button:?}"
                    );
                }
                if kind == Kind::Note {
                    let title = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == "Papan catatan" => {
                                Some(text.visual_bounding_rect())
                            }
                            _ => None,
                        })
                        .unwrap();
                    let file = ctx
                        .data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("header-menu", "File")))
                        })
                        .unwrap();
                    assert!(
                        (title.left() - file.left()).abs() < 8.0,
                        "Overview title must align with the menu at {size:?}: {title:?}, {file:?}"
                    );
                }
                if kind == Kind::Canvas {
                    let title = ctx
                        .read_response(egui::Id::new("header-title"))
                        .unwrap()
                        .rect;
                    let file = ctx
                        .data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("header-menu", "File")))
                        })
                        .unwrap();
                    assert!(title.right() < minimize.left());
                    let toolbar = ctx
                        .data(|data| {
                            data.get_temp::<egui::Rect>(egui::Id::new(("header-action", "+ Baru")))
                        })
                        .unwrap();
                    assert!(title.bottom() <= file.top() && file.bottom() < toolbar.top());
                }
                assert!(
                    wrapped_content.top() > header.bottom(),
                    "Main border must start below the header: {wrapped_content:?}, {header:?}"
                );
                assert!(wrapper.corner_radius.nw > 0 && wrapper.corner_radius.nw <= 8);
                assert!(wrapper.corner_radius.ne > 0 && wrapper.corner_radius.ne <= 8);
                assert!(wrapper.corner_radius.sw > 0 && wrapper.corner_radius.sw <= 8);
                assert!(wrapper.corner_radius.se > 0 && wrapper.corner_radius.se <= 8);
                assert!(screen.contains_rect(wrapped_content));
                let footer =
                    egui::containers::panel::PanelState::load(&ctx, egui::Id::new("status"))
                        .unwrap()
                        .rect;
                assert!(wrapped_content.bottom() < footer.top());
                assert!(output.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == PANEL && rect.rect == screen)
                }), "Frame margins need an opaque light background");
                let outer = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect) if rect.stroke.color == OUTER_BORDER => {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .expect("The window and main content need separate frames");
                assert!(outer.contains_rect(wrapped_content));
                assert!(outer.contains(header.center()) && outer.contains(footer.center()));
                assert!(
                    footer.height() <= 36.0,
                    "Footer should remain compact: {footer:?}"
                );
                for shape in output.shapes {
                    inspect(&shape.shape, shape.clip_rect, screen, &mut found);
                }
                for label in ["Canvas", "Notes", "Checklist", "Workspace"] {
                    assert!(found.contains(label), "{label} missing at {size:?}");
                }
                assert!(!found.contains("Kanban"));
                if kind == Kind::Canvas {
                    let toolbar = ctx
                        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-toolbar-bounds")))
                        .unwrap();
                    assert!(
                        wrapped_content.contains_rect(toolbar),
                        "Toolbar outside window: {toolbar:?} at {size:?}"
                    );
                    let canvas = ctx
                        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-drawing-bounds")))
                        .unwrap();
                    assert!(
                        toolbar.right() <= canvas.left(),
                        "Toolbar should be left of canvas: {toolbar:?}, {canvas:?}"
                    );
                    let settings = ctx
                        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-settings-bounds")))
                        .unwrap();
                    assert!(
                        wrapped_content.contains_rect(settings),
                        "Settings outside window at {size:?}: {settings:?}"
                    );
                    assert!(
                        canvas.right() <= settings.left(),
                        "Settings should be right of canvas: {settings:?}, {canvas:?}"
                    );
                    if settings.width() > 200.0 {
                        let title = ctx
                            .read_response(egui::Id::new("canvas-title"))
                            .unwrap()
                            .rect;
                        assert!(
                            settings.contains_rect(title),
                            "Canvas name must be inside right settings panel: {title:?}, {settings:?}"
                        );
                    }
                    let select = ctx
                        .read_response(egui::Id::new(("canvas-tool", "Pilih")))
                        .unwrap()
                        .rect;
                    let lasso = ctx
                        .read_response(egui::Id::new(("canvas-tool", "Lasso")))
                        .unwrap()
                        .rect;
                    let pen = ctx
                        .read_response(egui::Id::new(("canvas-tool", "Pena")))
                        .unwrap()
                        .rect;
                    assert!(
                        toolbar.contains_rect(select)
                            && toolbar.contains_rect(lasso)
                            && toolbar.contains_rect(pen),
                        "size {size:?}; rail {toolbar:?}; select {select:?}; lasso {lasso:?}; pen {pen:?}"
                    );
                    assert_eq!(select.top(), lasso.top());
                    assert!(select.right() < lasso.left() && pen.top() > select.bottom());
                } else {
                    assert!(found.contains("Catatan baru") && found.contains("Rapikan posisi"));
                }
            }
        }
    }

    #[test]
    fn document_search_stays_open_while_typing_and_closes_after_selection_in_each_module() {
        fn run(
            app: &mut Notvas,
            ctx: &egui::Context,
            size: egui::Vec2,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            )
        }
        fn click(app: &mut Notvas, ctx: &egui::Context, size: egui::Vec2, at: egui::Pos2) {
            for pressed in [true, false] {
                run(
                    app,
                    ctx,
                    size,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        fn label(output: &egui::FullOutput, value: &str) -> egui::Pos2 {
            output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == value => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing {value}"))
        }
        for kind in Kind::NAVIGATION {
            for size in [egui::vec2(380.0, 520.0), egui::vec2(1280.0, 820.0)] {
                let ctx = egui::Context::default();
                configure(&ctx);
                let mut app = app();
                app.kind = kind;
                let target = app.workspace.add(kind);
                app.workspace.documents[0].title = "猫 TARGET".into();
                let other = app.workspace.add(kind);
                app.workspace.documents[1].title = "Dokumen lain".into();
                app.select(Some(other));
                run(&mut app, &ctx, size, vec![]);
                let output = run(&mut app, &ctx, size, vec![]);
                click(&mut app, &ctx, size, label(&output, "Dokumen"));
                run(&mut app, &ctx, size, vec![]);
                let search = egui::Id::new("document-search");
                let at = ctx
                    .read_response(search)
                    .expect("Search must be visible in the document picker")
                    .rect
                    .center();
                click(&mut app, &ctx, size, at);
                run(&mut app, &ctx, size, vec![]);
                assert!(
                    ctx.read_response(search).is_some(),
                    "Clicking search closed {kind:?} at {size:?}"
                );
                run(
                    &mut app,
                    &ctx,
                    size,
                    vec![egui::Event::Text("猫 target".into())],
                );
                let output = run(&mut app, &ctx, size, vec![]);
                assert_eq!(app.query, "猫 target");
                assert!(ctx.read_response(search).is_some());
                assert_eq!(app.selected, Some(other));
                click(&mut app, &ctx, size, label(&output, "猫 TARGET"));
                run(&mut app, &ctx, size, vec![]);
                run(&mut app, &ctx, size, vec![]);
                assert_eq!(app.selected, Some(target));
                assert!(
                    ctx.read_response(search).is_none(),
                    "Choosing a document must close the picker"
                );
                if kind == Kind::Note {
                    assert!(!app.notes_board.overview);
                }
                if kind == Kind::Checklist {
                    assert!(!app.checklist_board.overview);
                }
                let output = run(&mut app, &ctx, size, vec![]);
                click(&mut app, &ctx, size, label(&output, "Dokumen"));
                run(&mut app, &ctx, size, vec![]);
                assert!(ctx.read_response(search).is_some());
                run(
                    &mut app,
                    &ctx,
                    size,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                run(&mut app, &ctx, size, vec![]);
                run(&mut app, &ctx, size, vec![]);
                assert!(ctx.read_response(search).is_none());
                let output = run(&mut app, &ctx, size, vec![]);
                click(&mut app, &ctx, size, label(&output, "Dokumen"));
                run(&mut app, &ctx, size, vec![]);
                assert!(ctx.read_response(search).is_some());
                click(
                    &mut app,
                    &ctx,
                    size,
                    egui::pos2(size.x - 20.0, size.y - 60.0),
                );
                run(&mut app, &ctx, size, vec![]);
                run(&mut app, &ctx, size, vec![]);
                assert!(ctx.read_response(search).is_none());
                assert_eq!(app.selected, Some(target));
            }
        }
    }

    #[test]
    fn drawing_app_header_menus_title_history_and_zoom_work_with_pointer_input() {
        fn run(
            app: &mut Notvas,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let modifiers = events
                .iter()
                .find_map(|event| match event {
                    egui::Event::Key { modifiers, .. } => Some(*modifiers),
                    _ => None,
                })
                .unwrap_or_default();
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 820.0),
                    )),
                    events,
                    modifiers,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            )
        }
        fn click(app: &mut Notvas, ctx: &egui::Context, at: egui::Pos2) -> egui::FullOutput {
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            run(app, ctx, vec![egui::Event::PointerMoved(at), button(true)]);
            run(app, ctx, vec![button(false)])
        }
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.kind = Kind::Canvas;
        app.create();
        run(&mut app, &ctx, vec![]);
        let file = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("header-menu", "File"))))
            .unwrap()
            .center();
        click(&mut app, &ctx, file);
        let output = run(&mut app, &ctx, vec![]);
        let new = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.contains("Dokumen baru") => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("File menu must expose the New action");
        click(&mut app, &ctx, new);
        assert_eq!(app.workspace.documents.len(), 2);
        let title = ctx
            .read_response(egui::Id::new("header-title"))
            .unwrap()
            .rect
            .center();
        click(&mut app, &ctx, title);
        run(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
        );
        run(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Diagram rencana".into())],
        );
        assert_eq!(
            app.workspace.documents.last().unwrap().title,
            "Diagram rencana"
        );
        assert!(app.dirty);
        let tool = ctx
            .read_response(egui::Id::new(("canvas-tool", "Kotak")))
            .unwrap()
            .rect
            .center();
        click(&mut app, &ctx, tool);
        let canvas = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("canvas-drawing-bounds")))
            .unwrap();
        let from = canvas.min + egui::vec2(30.0, 40.0);
        let to = from + egui::vec2(160.0, 100.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        run(&mut app, &ctx, vec![egui::Event::PointerMoved(to)]);
        run(&mut app, &ctx, vec![button(to, false)]);
        assert_eq!(
            app.workspace
                .documents
                .last()
                .unwrap()
                .canvas
                .elements
                .len(),
            1
        );
        for (label, expected) in [("Undo", 0), ("Redo", 1)] {
            let at = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("header-action", label))))
                .unwrap()
                .center();
            click(&mut app, &ctx, at);
            assert_eq!(
                app.workspace
                    .documents
                    .last()
                    .unwrap()
                    .canvas
                    .elements
                    .len(),
                expected
            );
        }
        for (label, expected) in [("Zoom in", 1.2), ("Zoom out", 1.0)] {
            let at = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("header-action", label))))
                .unwrap()
                .center();
            click(&mut app, &ctx, at);
            assert!((app.workspace.documents.last().unwrap().canvas.zoom - expected).abs() < 0.001);
        }
        let at = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("header-action", "Hapus…"))))
            .unwrap()
            .center();
        click(&mut app, &ctx, at);
        assert!(app.delete_document);
        assert_eq!(app.workspace.documents.len(), 2);
    }

    #[test]
    fn board_context_menu_deletes_the_clicked_document_after_confirmation() {
        fn run(
            app: &mut Notvas,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1100.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            )
        }
        fn locate(shapes: &[egui::epaint::ClippedShape], label: &str) -> egui::Pos2 {
            fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.visual_bounding_rect().center())
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            shapes
                .iter()
                .find_map(|s| find(&s.shape, label))
                .unwrap_or_else(|| panic!("Missing {label}"))
        }
        fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }
        for kind in [Kind::Note, Kind::Checklist] {
            let ctx = egui::Context::default();
            configure(&ctx);
            let mut app = app();
            app.kind = kind;
            app.create();
            let target = app.selected.unwrap();
            app.create();
            assert_ne!(app.selected, Some(target));
            app.notes_board.overview = true;
            app.checklist_board.overview = true;
            run(&mut app, &ctx, vec![]);
            let at = ctx
                .read_response(egui::Id::new((
                    if kind == Kind::Note {
                        "note-card"
                    } else {
                        "checklist-card"
                    },
                    target,
                )))
                .unwrap()
                .rect
                .center();
            run(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    pointer(at, egui::PointerButton::Secondary, true),
                ],
            );
            run(
                &mut app,
                &ctx,
                vec![pointer(at, egui::PointerButton::Secondary, false)],
            );
            let output = run(&mut app, &ctx, vec![]);
            let at = locate(
                &output.shapes,
                if kind == Kind::Note {
                    "Hapus catatan…"
                } else {
                    "Hapus daftar…"
                },
            );
            run(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    pointer(at, egui::PointerButton::Primary, true),
                ],
            );
            run(
                &mut app,
                &ctx,
                vec![pointer(at, egui::PointerButton::Primary, false)],
            );
            assert!(app.delete_document);
            assert_eq!(app.selected, Some(target));
            assert_eq!(app.workspace.documents.len(), 2);
            let output = run(&mut app, &ctx, vec![]);
            let at = locate(&output.shapes, "Hapus dokumen");
            run(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    pointer(at, egui::PointerButton::Primary, true),
                ],
            );
            run(
                &mut app,
                &ctx,
                vec![pointer(at, egui::PointerButton::Primary, false)],
            );
            assert!(!app.delete_document);
            assert_eq!(app.workspace.documents.len(), 1);
            assert!(!app.workspace.documents.iter().any(|d| d.id == target));
            assert!(app.notes_board.overview);
            assert!(app.checklist_board.overview);
        }
    }

    #[test]
    fn checklist_board_opens_returns_searches_and_saves_color_and_position() {
        fn run(
            app: &mut Notvas,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1100.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            )
        }
        fn click(
            app: &mut Notvas,
            ctx: &egui::Context,
            at: egui::Pos2,
            button: egui::PointerButton,
        ) {
            for pressed in [true, false] {
                run(
                    app,
                    ctx,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        fn label(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.job.text.trim().ends_with(text) => {
                        Some(t.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing {text}"))
        }
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.kind = Kind::Checklist;
        app.create();
        let first = app.selected.unwrap();
        app.workspace.documents[0].title = "Belanja mingguan".into();
        for done in [true, false] {
            let id = app.workspace.id();
            app.workspace.documents[0].tasks.push(Task {
                id,
                text: if done { "Beras" } else { "Telur" }.into(),
                detail: String::new(),
                done,
                column: 0,
            });
        }
        app.create();
        let second = app.selected.unwrap();
        app.workspace.documents[1].title = "Liburan".into();
        let note = app.workspace.add(Kind::Note);
        app.workspace.documents[2].title = "Belanja mingguan".into();
        app.workspace.documents[2].note_position = Some([500.0, 600.0]);
        assert_ne!(
            app.workspace.documents[0].note_color,
            app.workspace.documents[1].note_color
        );
        app.path = std::env::temp_dir().join(format!(
            "notvas-checklist-board-{}-{}.notvas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.load_failed = false;
        app.switch_kind(Kind::Checklist);
        assert!(app.checklist_board.overview);
        for size in [[380.0, 520.0], [640.0, 620.0], [1280.0, 820.0]] {
            frame(&mut app, &ctx, size);
            frame(&mut app, &ctx, size);
            let rect = ctx
                .read_response(egui::Id::new(("checklist-card", first)))
                .unwrap()
                .rect;
            assert_eq!(rect.size(), egui::vec2(208.0, 160.0));
            assert!(rect.left() >= 0.0 && rect.right() <= size[0]);
        }
        let output = run(&mut app, &ctx, vec![]);
        label(&output, "1/2 selesai");
        label(&output, "Beras");
        assert!(
            ctx.read_response(egui::Id::new(("checklist-card", note)))
                .is_none()
        );
        let at = ctx
            .read_response(egui::Id::new(("checklist-card", first)))
            .unwrap()
            .rect
            .center();
        click(&mut app, &ctx, at, egui::PointerButton::Primary);
        run(&mut app, &ctx, vec![]);
        assert!(!app.checklist_board.overview);
        assert_eq!(app.selected, Some(first));
        let task = app.workspace.documents[0].tasks[1].id;
        let check_id = ctx
            .data(|data| data.get_temp::<egui::Id>(egui::Id::new(("checklist-check", task))))
            .unwrap();
        let at = ctx.read_response(check_id).unwrap().rect.center();
        click(&mut app, &ctx, at, egui::PointerButton::Primary);
        assert!(app.workspace.documents[0].tasks[1].done);
        let output = run(&mut app, &ctx, vec![]);
        click(
            &mut app,
            &ctx,
            label(&output, "Kembali ke papan"),
            egui::PointerButton::Primary,
        );
        let output = run(&mut app, &ctx, vec![]);
        assert!(app.checklist_board.overview);
        label(&output, "2/2 selesai");
        let at = ctx
            .read_response(egui::Id::new(("checklist-card", first)))
            .unwrap()
            .rect
            .center();
        click(&mut app, &ctx, at, egui::PointerButton::Secondary);
        let output = run(&mut app, &ctx, vec![]);
        click(
            &mut app,
            &ctx,
            label(&output, "Sage"),
            egui::PointerButton::Primary,
        );
        assert_eq!(app.workspace.documents[0].note_color, model::NOTE_COLORS[5]);
        app.workspace.documents[0].note_position = Some([4000.0, 4000.0]);
        let before = app.workspace.clone();
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("checklist-search")));
        run(&mut app, &ctx, vec![egui::Event::Text("BELANJA".into())]);
        run(&mut app, &ctx, vec![]);
        assert!(
            ctx.read_response(egui::Id::new(("checklist-card", second)))
                .is_none()
        );
        let matched = ctx
            .read_response(egui::Id::new(("checklist-card", first)))
            .unwrap()
            .rect;
        assert!(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 800.0))
                .contains_rect(matched)
        );
        assert_eq!(app.workspace, before);
        app.checklist_board.query.clear();
        run(&mut app, &ctx, vec![]);
        let output = run(&mut app, &ctx, vec![]);
        click(
            &mut app,
            &ctx,
            label(&output, "Rapikan posisi"),
            egui::PointerButton::Primary,
        );
        run(&mut app, &ctx, vec![]);
        assert!(app.workspace.documents[0].note_position.is_none());
        assert_eq!(
            app.workspace.documents[2].note_position,
            Some([500.0, 600.0])
        );
        let at = ctx
            .read_response(egui::Id::new(("checklist-card", first)))
            .unwrap()
            .rect
            .center();
        run(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let to = at + egui::vec2(40.0, 30.0);
        run(&mut app, &ctx, vec![egui::Event::PointerMoved(to)]);
        run(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.workspace.documents[0].note_position.is_some());
        assert!(app.flush());
        assert_eq!(storage::load(&app.path).unwrap(), app.workspace);
        let output = run(&mut app, &ctx, vec![]);
        click(
            &mut app,
            &ctx,
            label(&output, "Daftar baru"),
            egui::PointerButton::Primary,
        );
        assert!(!app.checklist_board.overview);
        let created = app.workspace.documents.last().unwrap();
        assert_eq!(created.kind, Kind::Checklist);
        assert_eq!(app.selected, Some(created.id));
        assert!(created.tasks.is_empty());
        let _ = std::fs::remove_file(app.path.with_extension("notvas.bak"));
        let _ = std::fs::remove_file(&app.path);
    }

    #[test]
    fn right_drag_on_kanban_handle_does_not_move_a_card() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.kind = Kind::Kanban;
        app.create();
        let id = app.workspace.id();
        app.workspace.documents[0].tasks.push(Task {
            id,
            text: "Tugas".into(),
            detail: String::new(),
            done: false,
            column: 0,
        });
        let before = app.workspace.clone();
        let mut run = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(940.0, 620.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            );
        };
        run(vec![]);
        let at = ctx
            .read_response(egui::Id::new(("drag", id)))
            .unwrap()
            .rect
            .center();
        let right = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(vec![egui::Event::PointerMoved(at), right(at, true)]);
        let to = at + egui::vec2(330.0, -20.0);
        run(vec![egui::Event::PointerMoved(to)]);
        assert!(!egui::DragAndDrop::has_any_payload(&ctx));
        run(vec![right(to, false)]);
        assert_eq!(app.workspace, before);
    }

    #[test]
    fn note_image_insertion_history_and_small_window_layout() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.create();
        app.workspace.documents[0]
            .body
            .reconcile("猫\nParagraf setelah gambar".into(), TextStyle::default());
        app.selection = 2..2;
        let original = app.workspace.documents[0].body.clone();
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/notvas-icon-256.png");
        app.insert_note_image(0, &path, &ctx);
        let inserted = app.workspace.documents[0].body.clone();
        assert_eq!(inserted.images[0].layout, model::ImageLayout::Text);
        app.history(false);
        assert_eq!(app.workspace.documents[0].body, original);
        app.history(true);
        assert_eq!(app.workspace.documents[0].body, inserted);
        app.error = None;
        app.load_failed = false;
        app.dirty = false;
        for size in [[380.0, 520.0], [640.0, 600.0], [1280.0, 820.0]] {
            frame(&mut app, &ctx, size);
            frame(&mut app, &ctx, size);
            let body = ctx.read_response(egui::Id::new("note-body")).unwrap();
            assert!(
                body.rect.left() >= 0.0 && body.rect.right() <= size[0],
                "Note body {:?} outside {:?}",
                body.rect,
                size
            );
        }
        app.workspace.documents[0].body.images[0].layout = model::ImageLayout::Overlay;
        for size in [[380.0, 520.0], [1280.0, 820.0]] {
            frame(&mut app, &ctx, size);
        }
    }

    #[test]
    fn ctrl_wheel_zooms_every_mode_without_changing_note_data_or_header_scale() {
        for (kind, overview) in [
            (Kind::Canvas, false),
            (Kind::Note, true),
            (Kind::Note, false),
            (Kind::Checklist, true),
            (Kind::Checklist, false),
            (Kind::Kanban, false),
        ] {
            let ctx = egui::Context::default();
            configure(&ctx);
            let mut app = app();
            app.kind = kind;
            app.create();
            app.error = None;
            app.dirty = false;
            app.load_failed = false;
            app.notes_board.overview = overview;
            app.checklist_board.overview = overview;
            app.path = std::env::temp_dir().join(format!(
                "notvas-zoom-{}-{:?}-{}.notvas",
                std::process::id(),
                kind,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            frame(&mut app, &ctx, [1280.0, 820.0]);
            let before = app.workspace.clone();
            let position = egui::pos2(420.0, 360.0);
            let modifiers = egui::Modifiers {
                ctrl: true,
                command: true,
                ..Default::default()
            };
            let slot = match kind {
                Kind::Note => 0,
                Kind::Checklist => 1,
                Kind::Kanban => 2,
                Kind::Canvas => 0,
            };
            let level = |app: &Notvas| {
                if kind == Kind::Canvas {
                    app.workspace.documents[0].canvas.zoom
                } else {
                    app.content_zoom[slot]
                }
            };
            for direction in [6.0, -6.0] {
                let previous = level(&app);
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280.0, 820.0),
                        )),
                        modifiers,
                        events: vec![
                            egui::Event::PointerMoved(position),
                            egui::Event::MouseWheel {
                                unit: egui::MouseWheelUnit::Point,
                                delta: egui::vec2(0.0, direction),
                                modifiers,
                            },
                        ],
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                );
                assert!(
                    (level(&app) - previous) * direction > 0.0,
                    "{kind:?}, overview={overview}, direction={direction}, zoom {} -> {}, bounds {:?}, error {:?}",
                    previous,
                    level(&app),
                    ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-drawing-bounds"))),
                    app.error
                );
                assert_eq!(ctx.zoom_factor(), 1.0);
                if kind != Kind::Canvas {
                    assert_eq!(app.workspace, before);
                }
            }
            if kind != Kind::Canvas {
                app.content_zoom[slot] = 1.6;
                frame(&mut app, &ctx, [380.0, 520.0]);
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(380.0, 520.0),
                        )),
                        modifiers,
                        events: vec![egui::Event::Key {
                            key: egui::Key::Num0,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers,
                        }],
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                );
                assert_eq!(level(&app), 1.0);
                assert_eq!(app.workspace, before);
            }
            let _ = std::fs::remove_file(&app.path);
            let _ = std::fs::remove_file(app.path.with_extension("notvas.bak"));
        }
    }

    #[test]
    fn formatting_history_restores_content_and_style() {
        let mut app = app();
        app.create();
        app.workspace.documents[0]
            .body
            .reconcile("Halo dunia".into(), TextStyle::default());
        let original = app.workspace.documents[0].body.clone();
        app.selection = 0..4;
        app.typing.bold = true;
        app.typing.size = 28.0;
        app.apply_format(TextStyle::default());
        let formatted = app.workspace.documents[0].body.clone();
        assert!(
            formatted.styles[..4]
                .iter()
                .all(|s| s.bold && s.size == 28.0)
        );
        assert!(!formatted.styles[4].bold);
        app.history(false);
        assert_eq!(app.workspace.documents[0].body, original);
        app.history(true);
        assert_eq!(app.workspace.documents[0].body, formatted);
        app.workspace.documents[0].body.apply(
            5..10,
            TextStyle {
                size: 40.0,
                ..Default::default()
            },
        );
        app.selection = 0..10;
        app.typing = TextStyle {
            bold: true,
            ..Default::default()
        };
        app.apply_format(TextStyle::default());
        assert_eq!(app.workspace.documents[0].body.styles[5].size, 40.0);
        assert!(
            app.workspace.documents[0]
                .body
                .styles
                .iter()
                .all(|style| style.bold)
        );
    }

    #[test]
    fn editor_keyboard_input_selection_formatting_and_undo() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.create();
        frame(&mut app, &ctx, [1280.0, 820.0]);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("note-body")));
        let mut send = |events: Vec<egui::Event>, modifiers: egui::Modifiers| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 820.0),
                    )),
                    events,
                    modifiers,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            );
        };
        send(
            vec![egui::Event::Text("Halo dunia".into())],
            egui::Modifiers::NONE,
        );
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
        send(vec![key(egui::Key::A)], egui::Modifiers::COMMAND);
        send(vec![key(egui::Key::B)], egui::Modifiers::COMMAND);
        assert_eq!(app.workspace.documents[0].body.text, "Halo dunia");
        assert!(
            app.workspace.documents[0]
                .body
                .styles
                .iter()
                .all(|style| style.bold)
        );
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(egui::Key::Z)],
                modifiers: egui::Modifiers::COMMAND,
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        );
        assert_eq!(app.workspace.documents[0].body.text, "Halo dunia");
        assert!(
            app.workspace.documents[0]
                .body
                .styles
                .iter()
                .all(|style| !style.bold)
        );
    }

    #[test]
    fn checklist_rows_keep_actions_inside_and_support_click_keyboard_strike_and_delete() {
        fn struck(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().any(struck),
                egui::Shape::Text(shape) => {
                    shape.galley.job.text == "Tugas pertama"
                        && shape
                            .galley
                            .job
                            .sections
                            .iter()
                            .any(|section| section.format.strikethrough.width > 0.0)
                }
                _ => false,
            }
        }
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.kind = Kind::Checklist;
        app.create();
        app.load_failed = false;
        app.error = None;
        app.dirty = false;
        app.path = std::env::temp_dir().join(format!(
            "notvas-checklist-{}-{}.notvas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut ids = vec![];
        for label in [
            "Tugas pertama",
            "Tugas kedua",
            "Tugas ketiga dengan teks yang panjang sekali untuk memastikan tombol tetap berada di dalam kartu",
        ] {
            let id = app.workspace.id();
            ids.push(id);
            app.workspace.documents[0].tasks.push(Task {
                id,
                text: label.into(),
                detail: String::new(),
                done: false,
                column: 0,
            });
        }
        let run = |app: &mut Notvas, size: [f32; 2], events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                    events,
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            )
        };
        for size in [[1280.0, 820.0], [640.0, 620.0], [380.0, 520.0]] {
            run(&mut app, size, vec![]);
            run(&mut app, size, vec![]);
            let mut previous_right: Option<f32> = None;
            for id in &ids {
                let row = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("checklist-row", *id))))
                    .unwrap();
                let delete = ctx
                    .data(|data| {
                        data.get_temp::<egui::Id>(egui::Id::new(("checklist-delete", *id)))
                    })
                    .unwrap();
                let delete = ctx.read_response(delete).unwrap().rect;
                let text = ctx
                    .read_response(egui::Id::new(("checklist-text", *id)))
                    .unwrap()
                    .rect;
                assert!(
                    row.left() >= 0.0 && row.right() <= size[0],
                    "{row:?} at {size:?}"
                );
                assert!(
                    row.contains_rect(delete),
                    "delete {delete:?} outside {row:?}"
                );
                assert!(text.right() <= delete.left());
                if let Some(right) = previous_right {
                    assert!((row.right() - right).abs() < 0.1);
                }
                previous_right = Some(row.right());
            }
        }
        let size = [1280.0, 820.0];
        run(&mut app, size, vec![]);
        run(&mut app, size, vec![]);
        let checkbox_id = ctx
            .data(|data| data.get_temp::<egui::Id>(egui::Id::new(("checklist-check", ids[0]))))
            .unwrap();
        let checkbox = ctx.read_response(checkbox_id).unwrap().rect;
        assert!(checkbox.width() >= 44.0 && checkbox.height() >= 44.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(
            &mut app,
            size,
            vec![
                egui::Event::PointerMoved(checkbox.center()),
                button(checkbox.center(), true),
            ],
        );
        run(&mut app, size, vec![button(checkbox.center(), false)]);
        let output = run(&mut app, size, vec![]);
        assert!(app.workspace.documents[0].tasks[0].done);
        assert!(output.shapes.iter().any(|shape| struck(&shape.shape)));
        assert!(storage::load(&app.path).unwrap().documents[0].tasks[0].done);
        ctx.memory_mut(|memory| memory.request_focus(checkbox_id));
        run(
            &mut app,
            size,
            vec![egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!app.workspace.documents[0].tasks[0].done);
        let delete_id = ctx
            .data(|data| data.get_temp::<egui::Id>(egui::Id::new(("checklist-delete", ids[0]))))
            .unwrap();
        let delete = ctx.read_response(delete_id).unwrap().rect.center();
        run(
            &mut app,
            size,
            vec![egui::Event::PointerMoved(delete), button(delete, true)],
        );
        run(&mut app, size, vec![button(delete, false)]);
        assert_eq!(app.workspace.documents[0].tasks.len(), 2);
        assert!(
            !app.workspace.documents[0]
                .tasks
                .iter()
                .any(|task| task.id == ids[0])
        );
        let _ = std::fs::remove_file(&app.path);
        let _ = std::fs::remove_file(app.path.with_extension("notvas.bak"));
    }

    #[test]
    fn custom_titlebar_close_saves_changes_and_blocks_closing_when_save_fails() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.create();
        app.workspace.documents[0]
            .body
            .reconcile("Simpan sebelum tutup".into(), TextStyle::default());
        let run = |app: &mut Notvas, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0, 820.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.header(ctx),
            )
        };
        let has_close = |output: &egui::FullOutput| {
            output
                .viewport_output
                .values()
                .any(|viewport| viewport.commands.contains(&egui::ViewportCommand::Close))
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&mut app, vec![]);
        let at = ctx
            .read_response(egui::Id::new(("window-control", "Tutup")))
            .unwrap()
            .rect
            .center();
        run(
            &mut app,
            vec![egui::Event::PointerMoved(at), button(at, true)],
        );
        let output = run(&mut app, vec![button(at, false)]);
        assert!(!has_close(&output));
        assert!(app.pending_close && app.dirty);
        app.load_failed = false;
        app.pending_close = false;
        app.path = std::env::temp_dir().join(format!(
            "notvas-close-{}-{}.notvas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        run(&mut app, vec![button(at, true)]);
        let output = run(&mut app, vec![button(at, false)]);
        assert!(has_close(&output));
        assert!(!app.dirty);
        assert_eq!(
            storage::load(&app.path).unwrap().documents[0].body.text,
            "Simpan sebelum tutup"
        );
        let _ = std::fs::remove_file(&app.path);
    }

    #[test]
    fn primary_text_colors_meet_aa_contrast() {
        fn luminance(c: Color32) -> f32 {
            let values = [c.r(), c.g(), c.b()].map(|v| {
                let s = v as f32 / 255.0;
                if s <= 0.04045 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
            });
            values[0] * 0.2126 + values[1] * 0.7152 + values[2] * 0.0722
        }
        for background in [BG, PANEL, CARD] {
            for foreground in [TEXT, MUTED] {
                let a = luminance(foreground);
                let b = luminance(background);
                assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
            }
        }
        assert!((luminance(BG) + 0.05) / (luminance(ACCENT) + 0.05) >= 4.5);
        for background in [PANEL, BUTTON_HOVER, BUTTON_ACTIVE] {
            assert!((luminance(background) + 0.05) / (luminance(TEXT) + 0.05) >= 4.5);
        }
        for [r, g, b] in model::NOTE_COLORS {
            assert!((luminance(Color32::from_rgb(r, g, b)) + 0.05) / 0.05 >= 7.0);
            let header = note_header_color([r, g, b]);
            for background in [
                header,
                header.lerp_to_gamma(Color32::WHITE, 0.08),
                header.lerp_to_gamma(Color32::WHITE, 0.14),
            ] {
                assert!(1.05 / (luminance(background) + 0.05) >= 4.5);
            }
        }
    }

    #[test]
    fn note_editor_wrap_matches_note_color_and_keeps_the_header_white() {
        fn find(shape: &egui::Shape, text: &str, color: Color32) -> bool {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().any(|shape| find(shape, text, color)),
                egui::Shape::Text(shape) => {
                    shape.galley.job.text == text
                        && shape
                            .galley
                            .job
                            .sections
                            .iter()
                            .all(|section| section.format.color == color)
                }
                _ => false,
            }
        }
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.create();
        app.error = None;
        app.dirty = false;
        app.workspace.documents[0].title = "Judul berwarna".into();
        app.workspace.documents[0]
            .body
            .reconcile("Isi catatan".into(), TextStyle::default());
        for color in model::NOTE_COLORS {
            app.workspace.documents[0].note_color = color;
            for size in [egui::vec2(380.0, 520.0), egui::vec2(1280.0, 820.0)] {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| app.draw(ctx),
                );
                let header = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("note-header-bounds")))
                    .unwrap();
                let paper = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("note-paper-bounds")))
                    .unwrap();
                let main = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect) if rect.stroke.color == SHELL_BORDER => {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(
                    (paper.left() - main.left()).abs() < 0.5
                        && (paper.right() - main.right()).abs() < 0.5
                        && paper.bottom() >= main.bottom() - 0.5,
                    "Note color must reach the main edges: {paper:?}, main {main:?}"
                );
                assert!(paper.contains_rect(header));
                assert!(
                    paper.left() >= 0.0 && paper.right() <= size.x,
                    "{paper:?} at {size:?}, header {header:?}"
                );
                assert!(output.shapes.iter().any(|shape| find(
                    &shape.shape,
                    "Judul berwarna",
                    Color32::WHITE
                )));
                assert!(
                    output
                        .shapes
                        .iter()
                        .any(|shape| find(&shape.shape, "Isi catatan", TEXT))
                );
                assert_eq!(ctx.style().visuals.override_text_color, Some(TEXT));
            }
        }
    }

    #[test]
    fn smart_autosave_persists_edits_deletions_and_retries() {
        use std::{
            fs,
            time::{Duration, SystemTime, UNIX_EPOCH},
        };
        let folder = std::env::temp_dir().join(format!(
            "notvas-autosave-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut app = app();
        app.load_failed = false;
        app.path = folder.join("workspace.notvas");
        app.create();
        app.autosave_if_due(Instant::now());
        assert!(!app.dirty);

        app.workspace.documents[0]
            .body
            .reconcile("Catatan penting".into(), TextStyle::default());
        app.change();
        let deadline = app.autosave.deadline().unwrap();
        app.autosave_if_due(deadline - Duration::from_millis(1));
        assert!(
            storage::load(&app.path).unwrap().documents[0]
                .body
                .text
                .is_empty()
        );
        app.autosave_if_due(deadline);
        assert_eq!(
            storage::load(&app.path).unwrap().documents[0].body.text,
            "Catatan penting"
        );

        app.workspace.documents[0]
            .body
            .reconcile(String::new(), TextStyle::default());
        app.change();
        app.autosave_if_due(app.autosave.deadline().unwrap());
        assert!(
            storage::load(&app.path).unwrap().documents[0]
                .body
                .text
                .is_empty()
        );
        assert!(!app.dirty);

        let blocked = folder.join("blocked");
        fs::write(&blocked, "file penghalang pengujian").unwrap();
        app.path = blocked.join("workspace.notvas");
        app.workspace.documents[0].title = "Setelah retry".into();
        app.change();
        app.autosave_if_due(app.autosave.deadline().unwrap());
        assert!(app.dirty && app.autosave.retrying() && app.error.is_some());
        fs::remove_file(&blocked).unwrap();
        fs::create_dir(&blocked).unwrap();
        let retry = app.autosave.deadline().unwrap();
        app.autosave_if_due(retry - Duration::from_millis(1));
        assert!(!app.path.exists());
        app.autosave_if_due(retry);
        assert_eq!(
            storage::load(&app.path).unwrap().documents[0].title,
            "Setelah retry"
        );
        assert!(!app.dirty && app.error.is_none());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn paper_theme_keeps_readable_neutral_surfaces() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let style = ctx.style();
        for color in [BG, PANEL, CARD, TEXT, MUTED, ACCENT] {
            assert_eq!(color.r(), color.g());
            assert_eq!(color.g(), color.b());
        }
        assert_eq!(style.visuals.widgets.inactive.bg_fill, BUTTON);
        assert_ne!(BUTTON, BUTTON_HOVER);
        for visual in [
            style.visuals.widgets.inactive,
            style.visuals.widgets.hovered,
            style.visuals.widgets.active,
            style.visuals.widgets.open,
        ] {
            assert!(visual.bg_stroke.width > 0.0 && visual.bg_stroke.width <= 0.8);
        }
    }

    #[test]
    fn focus_loss_flushes_pending_text_and_failed_close_keeps_changes() {
        use std::{
            fs,
            time::{SystemTime, UNIX_EPOCH},
        };
        let folder = std::env::temp_dir().join(format!(
            "notvas-focus-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = app();
        app.load_failed = false;
        app.path = folder.join("workspace.notvas");
        app.create();
        frame(&mut app, &ctx, [1280.0, 820.0]);
        app.workspace.documents[0]
            .body
            .reconcile("Sebelum pindah jendela".into(), TextStyle::default());
        app.change();
        let _ = ctx.run(
            egui::RawInput {
                focused: false,
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        );
        assert_eq!(
            storage::load(&app.path).unwrap().documents[0].body.text,
            "Sebelum pindah jendela"
        );
        assert!(!app.dirty);

        let blocker = folder.join("blocked");
        fs::write(&blocker, "file penghalang pengujian").unwrap();
        app.path = blocker.join("workspace.notvas");
        app.workspace.documents[0]
            .body
            .reconcile("Tetap ada saat gagal".into(), TextStyle::default());
        app.change();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| app.draw(ctx));
        assert!(app.dirty && app.pending_close);
        assert_eq!(app.workspace.documents[0].body.text, "Tetap ada saat gagal");
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::CancelClose)
        );
        fs::remove_dir_all(folder).unwrap();
    }
}
