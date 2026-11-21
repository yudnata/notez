use super::*;

const HEADER: Color32 = Color32::from_rgb(242, 243, 244);
const LOGO_SIZE: f32 = 44.0;

fn text_menu<R>(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui) -> R) {
    let (response, _) = egui::containers::menu::MenuButton::from_button(
        egui::Button::new(label)
            .frame(true)
            .frame_when_inactive(false),
    )
    .ui(ui, contents);
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("header-menu", label)), response.rect));
}

fn action(ui: &mut egui::Ui, label: &str, hint: &str, enabled: bool) -> egui::Response {
    let response = ui
        .add_enabled(
            enabled,
            controls::button_with_text(ui, label, "")
                .frame(false)
                .min_size(egui::vec2(28.0, 28.0)),
        )
        .on_hover_text(hint);
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("header-action", label)), response.rect));
    response
}

impl Notvas {
    pub(super) fn header(&mut self, ctx: &egui::Context) {
        let logo = self
            .logo
            .get_or_insert_with(|| {
                let icon = app_icon();
                ctx.load_texture(
                    "notvas-logo",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [icon.width as usize, icon.height as usize],
                        &icon.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        egui::TopBottomPanel::top("navigation")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(HEADER)
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                ui.spacing_mut().button_padding = egui::vec2(6.0, 1.0);
                ui.spacing_mut().interact_size.y = 20.0;
                ui.horizontal(|ui| {
                    let (slot, _) =
                        ui.allocate_exact_size(egui::vec2(LOGO_SIZE, 28.0), egui::Sense::hover());
                    ui.place(
                        egui::Rect::from_min_size(
                            slot.min + egui::vec2(0.0, 2.0),
                            egui::Vec2::splat(LOGO_SIZE),
                        ),
                        egui::Image::new((logo, egui::Vec2::splat(LOGO_SIZE))).corner_radius(7.0),
                    );
                    ui.add_space(6.0);
                    let width = (ui.available_width() - 118.0).max(40.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(width);
                            let board_title = match self.kind {
                                Kind::Note if self.notes_board.overview => Some("Papan catatan"),
                                Kind::Checklist if self.checklist_board.overview => {
                                    Some("Papan checklist")
                                }
                                _ => None,
                            };
                            let title_width = (ui.available_width() - 24.0).clamp(40.0, 420.0);
                            let changed = if let Some(title) = board_title {
                                ui.add(
                                    egui::Label::new(RichText::new(title).size(18.0))
                                        .halign(egui::Align::Min)
                                        .truncate(),
                                );
                                false
                            } else if let Some(doc) = self
                                .workspace
                                .documents
                                .iter_mut()
                                .find(|d| Some(d.id) == self.selected)
                            {
                                ui.add(
                                    TextEdit::singleline(&mut doc.title)
                                        .id(egui::Id::new("header-title"))
                                        .font(FontId::proportional(18.0))
                                        .horizontal_align(egui::Align::Min)
                                        .frame(false)
                                        .desired_width(title_width)
                                        .hint_text("Tanpa judul"),
                                )
                                .changed()
                            } else {
                                ui.add(
                                    egui::Label::new(RichText::new(self.kind.label()).size(18.0))
                                        .halign(egui::Align::Min)
                                        .truncate(),
                                );
                                false
                            };
                            if changed {
                                self.change();
                            }
                            window_chrome::drag_area(ui, ui.available_rect_before_wrap());
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(114.0, 28.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            if let Some(command) = window_chrome::controls(ui) {
                                if command != egui::ViewportCommand::Close || self.flush() {
                                    ctx.send_viewport_cmd(command);
                                } else {
                                    self.pending_close = true;
                                }
                            }
                        },
                    );
                });
                let show_status = ui.available_width() >= 640.0;
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(LOGO_SIZE + 10.0);
                    self.header_menus(ui);
                    if show_status && ui.available_width() > 210.0 {
                        ui.add_space(10.0);
                        let status = if self.load_failed || self.autosave.retrying() {
                            "Penyimpanan bermasalah"
                        } else if self.dirty {
                            "Menyimpan otomatis..."
                        } else {
                            "Tersimpan otomatis"
                        };
                        ui.add(
                            egui::Label::new(RichText::new(status).size(11.0).color(MUTED))
                                .truncate(),
                        );
                    }
                });
                ui.add_space(2.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().interact_size.y = 28.0;
                    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                    self.mode_tabs(ui);
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        egui::vec2(112.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.document_picker(ui),
                    );
                    if action(ui, "+ Baru", "Dokumen baru · Ctrl+N", true).clicked() {
                        self.create();
                    }
                    if action(ui, "Hapus…", "Hapus dokumen", self.selected.is_some()).clicked() {
                        self.delete_document = true;
                    }
                    ui.separator();
                    for (label, redo, hint) in [
                        ("Undo", false, "Undo · Ctrl+Z"),
                        ("Redo", true, "Redo · Ctrl+Y"),
                    ] {
                        if action(ui, label, hint, self.header_can_history(redo)).clicked() {
                            self.header_history(redo);
                        }
                    }
                    ui.separator();
                    if action(ui, "Zoom out", "Perkecil tampilan · Ctrl+-", true).clicked() {
                        self.header_set_zoom(ctx, self.header_zoom() / 1.2);
                    }
                    ui.allocate_ui_with_layout(
                        egui::vec2(70.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            egui::ComboBox::from_id_salt("header-zoom")
                                .width(70.0)
                                .selected_text(format!("{:.0}%", self.header_zoom() * 100.0))
                                .show_ui(ui, |ui| self.zoom_choices(ui));
                        },
                    );
                    if action(ui, "Zoom in", "Perbesar tampilan · Ctrl++", true).clicked() {
                        self.header_set_zoom(ctx, self.header_zoom() * 1.2);
                    }
                    if action(
                        ui,
                        "Fit",
                        if self.kind == Kind::Canvas {
                            "Fit canvas · Ctrl+Shift+F"
                        } else {
                            "Reset zoom · Ctrl+0"
                        },
                        true,
                    )
                    .clicked()
                    {
                        self.header_fit(ctx);
                    }
                });
            });
    }

    fn header_menus(&mut self, ui: &mut egui::Ui) {
        text_menu(ui, "File", |ui| {
            if ui
                .add(egui::Button::new("Dokumen baru").shortcut_text("Ctrl+N"))
                .clicked()
            {
                self.create();
                ui.close();
            }
            if ui
                .add(egui::Button::new("Buka workspace…").shortcut_text("Ctrl+O"))
                .clicked()
            {
                ui.close();
                self.open();
            }
            if self.selected.is_some() && matches!(self.kind, Kind::Canvas | Kind::Note) {
                ui.menu_button("Ekspor", |ui| {
                    let formats = if self.kind == Kind::Canvas {
                        ["PNG", "SVG"]
                    } else {
                        ["HTML", "Teks"]
                    };
                    for (index, label) in formats.into_iter().enumerate() {
                        if ui.button(label).clicked() {
                            ui.close();
                            if self.kind == Kind::Canvas {
                                self.header_export_canvas(index == 0);
                            } else {
                                self.export(index == 0);
                            }
                        }
                    }
                });
            }
            ui.separator();
            if ui
                .add_enabled(self.selected.is_some(), egui::Button::new("Hapus dokumen…"))
                .clicked()
            {
                self.delete_document = true;
                ui.close();
            }
        });
        text_menu(ui, "Edit", |ui| {
            for (label, redo, shortcut) in [("Undo", false, "Ctrl+Z"), ("Redo", true, "Ctrl+Y")] {
                if ui
                    .add_enabled(
                        self.header_can_history(redo),
                        egui::Button::new(label).shortcut_text(shortcut),
                    )
                    .clicked()
                {
                    self.header_history(redo);
                    ui.close();
                }
            }
        });
        text_menu(ui, "Tampilan", |ui| {
            ui.menu_button("Zoom", |ui| self.zoom_choices(ui));
            if ui
                .button(if self.kind == Kind::Canvas {
                    "Fit canvas"
                } else {
                    "Reset zoom"
                })
                .clicked()
            {
                self.header_fit(ui.ctx());
                ui.close();
            }
            ui.separator();
            for kind in Kind::NAVIGATION {
                if ui
                    .selectable_label(self.kind == kind, kind.label())
                    .clicked()
                {
                    self.switch_kind(kind);
                    ui.close();
                }
            }
        });
        text_menu(ui, "Workspace", |ui| {
            ui.label("Penyimpanan lokal");
            ui.add(egui::Label::new(self.path.display().to_string()).wrap());
            if ui.add(controls::button(ui, "Salin lokasi")).clicked() {
                ui.ctx().copy_text(self.path.display().to_string());
            }
            if ui.button("Buka workspace…").clicked() {
                ui.close();
                self.open();
            }
        });
        text_menu(ui, "Bantuan", |ui| {
            ui.label(RichText::new("Shortcut").strong());
            for text in [
                "Ctrl+N · dokumen baru",
                "Ctrl+O · buka workspace",
                "Ctrl+scroll · zoom",
                "Ctrl+0 · reset zoom",
                "Ctrl+Z / Ctrl+Y · undo / redo",
                "Canvas: V pilih · P pena · E penghapus",
                "Canvas: T teks · S kotak · A panah",
                "Space+drag / klik kanan+drag · geser",
            ] {
                ui.label(text);
            }
        });
    }

    fn header_can_history(&self, redo: bool) -> bool {
        match self.kind {
            Kind::Canvas => self.selected.is_some() && self.canvas_editor.can_history(redo),
            Kind::Note if !self.notes_board.overview => {
                if redo {
                    !self.redo.is_empty()
                } else {
                    !self.undo.is_empty()
                }
            }
            _ => false,
        }
    }

    fn header_history(&mut self, redo: bool) {
        if self.kind == Kind::Canvas {
            if let Some(doc) = self
                .workspace
                .documents
                .iter_mut()
                .find(|d| Some(d.id) == self.selected)
            {
                if self.canvas_editor.history(&mut doc.canvas, redo) {
                    self.change_immediately();
                }
            }
        } else if self.kind == Kind::Note {
            self.history(redo);
        }
    }

    fn header_zoom(&self) -> f32 {
        match self.kind {
            Kind::Canvas => self
                .workspace
                .documents
                .iter()
                .find(|d| Some(d.id) == self.selected)
                .map_or(1.0, |d| d.canvas.zoom),
            Kind::Note => self.content_zoom[0],
            Kind::Checklist => self.content_zoom[1],
            Kind::Kanban => self.content_zoom[2],
        }
    }

    fn header_set_zoom(&mut self, ctx: &egui::Context, zoom: f32) {
        if self.kind == Kind::Canvas {
            if let Some(doc) = self
                .workspace
                .documents
                .iter_mut()
                .find(|d| Some(d.id) == self.selected)
            {
                let size = ctx
                    .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-drawing-bounds")))
                    .map_or(egui::vec2(640.0, 480.0), |rect| rect.size());
                let world = [
                    (size.x * 0.5 - doc.canvas.pan[0]) / doc.canvas.zoom,
                    (size.y * 0.5 - doc.canvas.pan[1]) / doc.canvas.zoom,
                ];
                doc.canvas.zoom = zoom.clamp(0.15, 4.0);
                doc.canvas.pan = [
                    size.x * 0.5 - world[0] * doc.canvas.zoom,
                    size.y * 0.5 - world[1] * doc.canvas.zoom,
                ];
                self.change();
            }
        } else {
            self.content_zoom[match self.kind {
                Kind::Note => 0,
                Kind::Checklist => 1,
                _ => 2,
            }] = zoom.clamp(0.5, 2.0);
        }
    }

    fn zoom_choices(&mut self, ui: &mut egui::Ui) {
        for percent in [50, 75, 100, 125, 150, 200] {
            if ui
                .selectable_label(
                    (self.header_zoom() * 100.0 - percent as f32).abs() < 0.5,
                    format!("{percent}%"),
                )
                .clicked()
            {
                self.header_set_zoom(ui.ctx(), percent as f32 / 100.0);
                ui.close();
            }
        }
    }

    fn header_fit(&mut self, ctx: &egui::Context) {
        if self.kind == Kind::Canvas {
            if let Some(doc) = self
                .workspace
                .documents
                .iter_mut()
                .find(|d| Some(d.id) == self.selected)
            {
                let size = ctx
                    .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("canvas-drawing-bounds")))
                    .map_or(egui::vec2(640.0, 480.0), |rect| rect.size());
                self.canvas_editor.fit(&mut doc.canvas, size);
                self.change();
            }
        } else {
            self.header_set_zoom(ctx, 1.0);
        }
    }

    fn header_export_canvas(&mut self, png: bool) {
        let Some(doc) = self
            .workspace
            .documents
            .iter()
            .find(|d| Some(d.id) == self.selected)
        else {
            return;
        };
        let extension = if png { "png" } else { "svg" };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(extension, &[extension])
            .set_file_name(format!("canvas.{extension}"))
            .save_file()
        {
            let result = if png {
                doc.canvas.png(&doc.title)
            } else {
                Ok(doc.canvas.svg(&doc.title, true).into_bytes())
            };
            if let Err(error) =
                result.and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()))
            {
                self.error = Some(error);
            }
        }
    }
}
