#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autosave;
mod model;
mod storage;

use autosave::Autosave;
use egui::{Color32, FontFamily, FontId, RichText, Stroke, TextEdit};
use model::{Kind, RichDocument, Task, TextStyle, Workspace};
use std::{path::PathBuf, time::Instant};

const BG: Color32 = Color32::from_gray(20);
const PANEL: Color32 = Color32::from_gray(26);
const CARD: Color32 = Color32::from_gray(35);
const TEXT: Color32 = Color32::from_gray(229);
const MUTED: Color32 = Color32::from_gray(165);
const ACCENT: Color32 = Color32::from_gray(211);

struct Notez {
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

impl Notez {
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
        let selected = workspace.documents.first().map(|d| d.id);
        let kind = workspace.documents.first().map_or(Kind::Note, |d| d.kind);
        let load_failed = error.is_some();
        Self {
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
            .add_filter("Workspace Notez", &["notez", "bak"])
            .pick_file()
        {
            match storage::load(&path) {
                Ok(ws) => {
                    self.workspace = ws;
                    self.path = path;
                    self.load_failed = false;
                    self.kind = self
                        .workspace
                        .documents
                        .first()
                        .map_or(Kind::Note, |d| d.kind);
                    self.select(self.workspace.documents.first().map(|d| d.id));
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
                format!("{}\n\n{}", doc.title, doc.body.text)
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

    fn sidebar(&mut self, ctx: &egui::Context) {
        let logo = self.logo.get_or_insert_with(|| {
            let icon = app_icon();
            ctx.load_texture(
                "notez-logo",
                egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                ),
                egui::TextureOptions::LINEAR,
            )
        });
        let logo_id = logo.id();
        egui::SidePanel::left("sidebar")
            .exact_width(222.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(18.0))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((logo_id, egui::vec2(40.0, 40.0))).corner_radius(6.0));
                    ui.label(RichText::new("notez.").size(31.0).strong().color(TEXT));
                });
                ui.label(
                    RichText::new("Ruang untuk pikiranmu")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_space(30.0);
                for kind in [Kind::Note, Kind::Checklist, Kind::Kanban] {
                    let count = self
                        .workspace
                        .documents
                        .iter()
                        .filter(|d| d.kind == kind)
                        .count();
                    let response = ui.add_sized(
                        [ui.available_width(), 40.0],
                        egui::Button::new(format!("{}    {}", kind.label(), count))
                            .selected(self.kind == kind),
                    );
                    if response.clicked() && self.kind != kind {
                        self.flush();
                        self.kind = kind;
                        self.query.clear();
                        self.select(
                            self.workspace
                                .documents
                                .iter()
                                .find(|d| d.kind == kind)
                                .map(|d| d.id),
                        );
                    }
                    ui.add_space(4.0);
                }
                ui.add_space(22.0);
                ui.separator();
                ui.add_space(18.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new("+ Buat baru"),
                    )
                    .on_hover_text("Ctrl+N")
                    .clicked()
                {
                    self.create();
                }
                ui.add_space(10.0);
                if ui
                    .button("Buka workspace...")
                    .on_hover_text("Ctrl+O")
                    .clicked()
                {
                    self.open();
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("Konsep awal  /  0.1.0")
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.label(
                        RichText::new("Lokal di perangkatmu")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
    }

    fn document_list(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("documents")
            .default_width(238.0)
            .width_range(190.0..=300.0)
            .frame(egui::Frame::new().fill(BG).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.label(RichText::new(self.kind.label()).size(22.0).strong());
                ui.add_space(18.0);
                ui.add(
                    TextEdit::singleline(&mut self.query)
                        .hint_text("Cari judul atau isi...")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(16.0);
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
                    .map(|d| {
                        (
                            d.id,
                            d.title.clone(),
                            if d.kind == Kind::Note {
                                d.body
                                    .text
                                    .lines()
                                    .next()
                                    .unwrap_or("Belum ada tulisan")
                                    .chars()
                                    .take(45)
                                    .collect::<String>()
                            } else {
                                format!("{} tugas", d.tasks.len())
                            },
                        )
                    })
                    .collect();
                egui::ScrollArea::vertical()
                    .id_salt("document-scroll")
                    .show(ui, |ui| {
                        if docs.is_empty() {
                            ui.label(
                                RichText::new(if query.is_empty() {
                                    "Belum ada dokumen.\nBuat yang pertama."
                                } else {
                                    "Tidak ada hasil pencarian."
                                })
                                .color(MUTED),
                            );
                        }
                        for (id, title, preview) in docs {
                            let selected = self.selected == Some(id);
                            let label = format!(
                                "{}\n{}",
                                if title.trim().is_empty() {
                                    "Tanpa judul"
                                } else {
                                    &title
                                },
                                preview
                            );
                            if ui
                                .add_sized(
                                    [ui.available_width(), 67.0],
                                    egui::Button::new(label).selected(selected),
                                )
                                .clicked()
                            {
                                self.flush();
                                self.select(Some(id));
                            }
                            ui.add_space(6.0);
                        }
                    });
            });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self
            .workspace
            .documents
            .iter()
            .position(|d| Some(d.id) == self.selected)
        else {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.27);
                ui.label(
                    RichText::new(match self.kind {
                        Kind::Note => "Mulai dari satu pikiran.",
                        Kind::Checklist => "Satu tugas, satu langkah.",
                        Kind::Kanban => "Beri ruang untuk rencanamu.",
                    })
                    .size(29.0)
                    .strong(),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Buat dokumen baru. Perubahan tersimpan otomatis.").color(MUTED),
                );
                ui.add_space(24.0);
                if ui
                    .add_sized(
                        [170.0, 40.0],
                        egui::Button::new(format!("+ Buat {}", self.kind.label())),
                    )
                    .clicked()
                {
                    self.create();
                }
            });
            return;
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Ruang pribadi / {}", self.kind.label()))
                    .size(12.0)
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Hapus").clicked() {
                    self.delete_document = true;
                }
                if self.kind == Kind::Note {
                    ui.menu_button("Ekspor", |ui| {
                        if ui.button("HTML (dengan format)").clicked() {
                            ui.close();
                            self.export(true);
                        }
                        if ui.button("Teks (.txt)").clicked() {
                            ui.close();
                            self.export(false);
                        }
                    });
                }
            });
        });
        ui.add_space(22.0);
        if ui
            .add(
                TextEdit::singleline(&mut self.workspace.documents[index].title)
                    .font(FontId::proportional(30.0))
                    .frame(false)
                    .hint_text("Tanpa judul")
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.change();
        }
        ui.add_space(10.0);
        match self.kind {
            Kind::Note => self.note(ui, index),
            Kind::Checklist => self.checklist(ui, index),
            Kind::Kanban => self.kanban(ui, index),
        }
    }

    fn note(&mut self, ui: &mut egui::Ui, index: usize) {
        let previous = self.typing;
        let mut format_changed = false;
        ui.horizontal_wrapped(|ui| {
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
            ui.label(RichText::new("Ukuran").color(MUTED));
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
            ui.label(RichText::new("Jarak baris").color(MUTED));
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
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                self.history(false);
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                .clicked()
            {
                self.history(true);
            }
        });
        if format_changed {
            self.apply_format(previous);
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(RichText::new("Pilih teks untuk memformat. Tanpa seleksi, format berlaku untuk tulisan berikutnya.").size(11.0).color(MUTED));
        ui.add_space(14.0);
        let doc = &self.workspace.documents[index];
        let before = doc.body.clone();
        let mut text = before.text.clone();
        let typing = self.typing;
        let spacing = doc.spacing;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, width: f32| {
            let mut styled = before.clone();
            styled.reconcile(buffer.as_str().into(), typing);
            ui.fonts_mut(|fonts| fonts.layout_job(layout_job(&styled, width, spacing)))
        };
        let mut output = None;
        egui::ScrollArea::vertical()
            .id_salt(("note-scroll", doc.id))
            .show(ui, |ui| {
                output = Some(
                    TextEdit::multiline(&mut text)
                        .id(egui::Id::new("note-body"))
                        .font(FontId::proportional(18.0))
                        .desired_width(f32::INFINITY)
                        .desired_rows(22)
                        .frame(false)
                        .hint_text("Tulis sesuatu di sini...")
                        .layouter(&mut layouter)
                        .show(ui),
                );
            });
        if let Some(output) = output {
            if output.response.changed() {
                self.workspace.documents[index].body.reconcile(text, typing);
                self.remember(before);
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
    }

    fn task_input(&mut self, ui: &mut egui::Ui, index: usize) {
        let mut add = false;
        ui.horizontal(|ui| {
            let response = ui.add_sized(
                [ui.available_width() - 100.0, 34.0],
                TextEdit::singleline(&mut self.new_task).hint_text("Tulis tugas baru..."),
            );
            add = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            add |= ui
                .add_enabled(
                    !self.new_task.trim().is_empty(),
                    egui::Button::new("Tambah").min_size(egui::vec2(88.0, 34.0)),
                )
                .clicked();
            if add {
                response.request_focus();
            }
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
            .show(ui, |ui| {
                let tasks = &mut self.workspace.documents[index].tasks;
                if tasks.is_empty() {
                    ui.label(
                        RichText::new("Daftar masih kosong. Tambahkan tugas pertamamu di atas.")
                            .color(MUTED),
                    );
                }
                for task in tasks {
                    ui.push_id(task.id, |ui| {
                        ui.horizontal(|ui| {
                            immediate |= ui
                                .checkbox(&mut task.done, "")
                                .on_hover_text("Tandai selesai")
                                .changed();
                            ui.scope(|ui| {
                                if task.done {
                                    ui.visuals_mut().override_text_color = Some(MUTED);
                                }
                                changed |= ui
                                    .add_sized(
                                        [ui.available_width() - 60.0, 32.0],
                                        TextEdit::singleline(&mut task.text).frame(false),
                                    )
                                    .changed();
                            });
                            if ui.button("Hapus").clicked() {
                                delete = Some(task.id);
                            }
                        });
                        ui.separator();
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
        ui.label(
            RichText::new("Pindahkan kartu dengan menyeretnya atau pilih status pada kartu.")
                .color(MUTED),
        );
        ui.add_space(14.0);
        self.task_input(ui, index);
        ui.add_space(20.0);
        let titles = ["Rencana", "Dikerjakan", "Selesai"];
        let mut changed = false;
        let mut delete = None;
        let mut moved = None;
        egui::ScrollArea::horizontal()
            .id_salt("board-horizontal")
            .show(ui, |ui| {
                ui.set_min_width(660.0_f32.max(ui.available_width()));
                ui.columns(3, |columns| {
                    for (column, ui) in columns.iter_mut().enumerate() {
                        let frame = egui::Frame::new()
                            .fill(PANEL)
                            .corner_radius(6)
                            .inner_margin(12.0);
                        let (_, payload) = ui.dnd_drop_zone::<u64, _>(frame, |ui| {
                            ui.set_min_height(340.0);
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
                                .show(ui, |ui| {
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
                                                    ui.dnd_drag_source(
                                                        egui::Id::new(("drag", task.id)),
                                                        task.id,
                                                        |ui| {
                                                            ui.label(
                                                                RichText::new("Seret kartu")
                                                                    .size(11.0)
                                                                    .color(MUTED),
                                                            );
                                                        },
                                                    );
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
                                                    if ui.small_button("Hapus kartu").clicked() {
                                                        delete = Some(task.id);
                                                    }
                                                });
                                            ui.add_space(10.0);
                                        });
                                    }
                                    if count == 0 {
                                        ui.label(
                                            RichText::new("Belum ada kartu")
                                                .size(12.0)
                                                .color(MUTED),
                                        );
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

impl Notez {
    fn draw(&mut self, ctx: &egui::Context) {
        if !self.delete_document && !self.pending_close {
            self.shortcuts(ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.flush() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_close = true;
        }
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18, 9)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.load_failed {
                            "Autosave memerlukan folder penyimpanan lain"
                        } else if self.dirty && self.autosave.retrying() {
                            "Belum tersimpan. Mencoba lagi otomatis..."
                        } else if self.dirty {
                            "Menyimpan otomatis..."
                        } else {
                            &self.status
                        })
                        .size(11.0)
                        .color(MUTED),
                    );
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
                });
            });
        self.sidebar(ctx);
        self.document_list(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(30, 24)),
            )
            .show(ctx, |ui| {
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
                                if ui.button("Pilih folder penyimpanan lain...").clicked() {
                                    self.choose_storage_folder();
                                }
                            } else if ui.button("Tutup pesan").clicked() {
                                self.error = None;
                            }
                        });
                    ui.add_space(12.0);
                }
                self.editor(ui);
            });
        if self.delete_document {
            let modal = egui::Modal::new(egui::Id::new("delete-document")).show(ctx, |ui| {
                ui.heading("Hapus dokumen?");
                ui.label("Dokumen beserta isinya akan dihapus dari workspace.");
                ui.horizontal(|ui| {
                    if ui.button("Batal").clicked() {
                        self.delete_document = false;
                    }
                    if ui.button("Hapus dokumen").clicked() {
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
                if ui.button("Pilih folder penyimpanan lain...").clicked() {
                    self.choose_storage_folder();
                    if !self.dirty {
                        self.pending_close = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                if ui.button("Kembali menulis").clicked() {
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

fn configure(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, path, family) in [
        (
            "notez-regular",
            "C:/Windows/Fonts/segoeui.ttf",
            FontFamily::Proportional,
        ),
        (
            "notez-bold",
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
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = PANEL;
    style.visuals.faint_bg_color = CARD;
    style.visuals.selection.bg_fill = Color32::from_gray(66);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.bg_fill = CARD;
    style.visuals.widgets.inactive.weak_bg_fill = CARD;
    style.visuals.widgets.hovered.bg_fill = Color32::from_gray(50);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(50);
    style.visuals.widgets.active.bg_fill = Color32::from_gray(60);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_gray(60);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.warn_fg_color = TEXT;
    style.visuals.error_fg_color = TEXT;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    ctx.set_style(style);
}

impl eframe::App for Notez {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw(ctx);
    }
}

fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/notez-icon-256.png"))
        .expect("Bundled Notez icon must be a valid PNG")
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Notez",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(app_icon())
                .with_inner_size([1280.0, 820.0])
                .with_min_inner_size([940.0, 620.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Notez::new(cc)))),
    )
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    fn app() -> Notez {
        Notez {
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

    fn frame(app: &mut Notez, ctx: &egui::Context, size: [f32; 2]) {
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
        for size in [[940.0, 620.0], [1280.0, 820.0]] {
            for kind in [Kind::Note, Kind::Checklist, Kind::Kanban] {
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
                assert!((luminance(foreground) + 0.05) / (luminance(background) + 0.05) >= 4.5);
            }
        }
        assert!((luminance(ACCENT) + 0.05) / (luminance(BG) + 0.05) >= 4.5);
    }

    #[test]
    fn smart_autosave_persists_edits_deletions_and_retries() {
        use std::{
            fs,
            time::{Duration, SystemTime, UNIX_EPOCH},
        };
        let folder = std::env::temp_dir().join(format!(
            "notez-autosave-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut app = app();
        app.load_failed = false;
        app.path = folder.join("workspace.notez");
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
        app.path = blocked.join("workspace.notez");
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
    fn theme_surfaces_and_interaction_states_are_monochrome() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let style = ctx.style();
        for color in [
            BG,
            PANEL,
            CARD,
            TEXT,
            MUTED,
            ACCENT,
            style.visuals.selection.bg_fill,
            style.visuals.widgets.hovered.bg_fill,
            style.visuals.widgets.active.bg_fill,
        ] {
            assert_eq!(color.r(), color.g());
            assert_eq!(color.g(), color.b());
        }
    }

    #[test]
    fn focus_loss_flushes_pending_text_and_failed_close_keeps_changes() {
        use std::{
            fs,
            time::{SystemTime, UNIX_EPOCH},
        };
        let folder = std::env::temp_dir().join(format!(
            "notez-focus-{}-{}",
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
        app.path = folder.join("workspace.notez");
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
        app.path = blocker.join("workspace.notez");
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
