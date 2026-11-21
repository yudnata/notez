use egui::{Button, ColorImage, TextureHandle, Ui, WidgetText};

fn icon_path(label: &str) -> &'static str {
    match label {
        "Zoom in" => "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14m5 12 6 6M7 10h6m-3-3v6",
        "Zoom out" => "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14m5 12 6 6M7 10h6",
        "Canvas" => "M4 19 16 7l3 3L7 22H3v-4M14 9l3 3M17 6l2-2 3 3-2 2",
        "Notes" => "M5 3h14v18H5zM8 8h8M8 12h8M8 16h5",
        "Checklist" => "m3 6 2 2 3-4M11 6h10m-18 7 2 2 3-4m3 2h10m-18 7 2 2 3-4m3 2h10",
        "Kanban" => "M3 4h5v16H3zm7 0h5v11h-5zm7 0h5v14h-5z",
        "Rapikan posisi" | "Kembali ke papan" => {
            "M3 3h7v7H3zm11 0h7v7h-7zM3 14h7v7H3zm11 0h7v7h-7z"
        }
        "Undo" => "m8 4-5 5 5 5M3 9h11a7 7 0 0 1 0 14",
        "Redo" => "m16 4 5 5-5 5m5-5H10a7 7 0 0 0 0 14",
        "Fit" | "Ukuran" => "M9 3H3v6m12-6h6v6M3 15v6h6m12-6v6h-6",
        "Warna" => {
            "M12 3a9 9 0 0 0 0 18h2a2 2 0 0 0 0-4h-1a2 2 0 0 1 0-4h5a3 3 0 0 0 3-3 9 9 0 0 0-9-7M7 9h.01M11 6h.01m5 2h.01M6 14h.01"
        }
        "Gambar..." => "M3 3h18v18H3zm0 14 5-5 4 4 4-6 5 7M8 7h.01",
        "Rata kiri" => "M4 6h16M4 10h10M4 14h16M4 18h10",
        "Rata tengah" => "M4 6h16M7 10h10M4 14h16M7 18h10",
        "Rata kanan" => "M4 6h16M10 10h10M4 14h16M10 18h10",
        "Rata kiri-kanan" => "M4 6h16M4 10h16M4 14h16M4 18h16",
        "Salin lokasi" => "M8 8h13v13H8zM3 16V3h13",
        "Template" => "M3 3h18v18H3zm0 6h18M10 9v12",
        "Ke depan" => "M8 8h13v13H8zM3 16V3h13",
        "Ke belakang" => "M3 3h13v13H3zm5 18h13V8",
        "Batal" | "Tutup pesan" => "m6 6 12 12M6 18 18 6",
        label if label.starts_with("Hapus") => "M4 7h16M9 7V3h6v4M6 7l1 14h10l1-14M10 11v6m4-6v6",
        label
            if label.starts_with('+')
                || label.starts_with("Tambah")
                || label.starts_with("Buat") =>
        {
            "M12 4v16M4 12h16"
        }
        label
            if label.starts_with("Buka")
                || label.starts_with("Workspace")
                || label.starts_with("Pilih folder") =>
        {
            "M3 7V4h7l2 3h9v4M3 7h9m-9 4h19l-3 10H3z"
        }
        label
            if label.starts_with("Ekspor")
                || label.starts_with("HTML")
                || label.starts_with("Teks (")
                || label.starts_with("PNG")
                || label.starts_with("SVG") =>
        {
            "M12 16V3m-5 5 5-5 5 5M4 14v7h16v-7"
        }
        label if label.starts_with("Duplikat") => "M8 8h13v13H8zM3 16V3h13",
        label if label.starts_with("Edit") => "M4 20h4L21 7l-4-4L4 16zm10-14 4 4",
        label if label.starts_with("Kembali") => "m10 5-7 7 7 7M3 12h18",
        _ => "M4 6h16M4 12h16M4 18h16M8 3v6m8 0v6m-8 0v6",
    }
}

pub fn button(ui: &Ui, label: &str) -> Button<'static> {
    button_with_text(ui, label, label.trim_start_matches("+ ").to_owned())
}

pub fn icon_button(ui: &Ui, label: &str, selected: bool) -> Button<'static> {
    let path = icon_path(label);
    let key = egui::Id::new(("action-icon", path));
    let cached = ui.ctx().data(|d| d.get_temp::<TextureHandle>(key));
    let texture = cached.unwrap_or_else(|| {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="{path}" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>"#);
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
            .expect("Static action icon must be valid SVG");
        let mut pixels = resvg::tiny_skia::Pixmap::new(48, 48).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(2.0, 2.0),
            &mut pixels.as_mut(),
        );
        let image = ColorImage::from_rgba_premultiplied([48, 48], pixels.data());
        let texture = ui.ctx().load_texture(
            format!("action-icon-{key:?}"),
            image,
            egui::TextureOptions::LINEAR,
        );
        ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
        texture
    });
    Button::image((texture.id(), egui::vec2(15.0, 15.0)))
        .image_tint_follows_text_color(true)
        .selected(selected)
}

pub fn button_with_text(ui: &Ui, label: &str, text: impl Into<WidgetText>) -> Button<'static> {
    let path = icon_path(label);
    let key = egui::Id::new(("action-icon", path));
    let cached = ui.ctx().data(|d| d.get_temp::<TextureHandle>(key));
    let texture = cached.unwrap_or_else(|| {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="{path}" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>"#);
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).expect("Static action icon must be valid SVG");
        let mut pixels = resvg::tiny_skia::Pixmap::new(48, 48).unwrap();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(2.0, 2.0), &mut pixels.as_mut());
        let image = ColorImage::from_rgba_premultiplied([48, 48], pixels.data());
        let texture = ui.ctx().load_texture(format!("action-icon-{key:?}"), image, egui::TextureOptions::LINEAR);
        ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
        texture
    });
    Button::image_and_text((texture.id(), egui::vec2(16.0, 16.0)), text)
        .image_tint_follows_text_color(true)
}

pub fn menu<R>(
    ui: &mut Ui,
    label: &str,
    contents: impl FnOnce(&mut Ui) -> R,
) -> (egui::Response, Option<egui::InnerResponse<R>>) {
    egui::containers::menu::MenuButton::from_button(button(ui, label)).ui(ui, contents)
}

pub fn drag_handle(ui: &mut Ui) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    for x in [-3.0, 3.0] {
        for y in [-5.0, 0.0, 5.0] {
            ui.painter()
                .circle_filled(rect.center() + egui::vec2(x, y), 1.2, crate::MUTED);
        }
    }
    response.on_hover_text("Seret untuk memindahkan kartu")
}

pub fn task_checkbox(ui: &mut Ui, checked: &mut bool, label: &str) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(44.0, 44.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.spacing_mut().icon_width = 28.0;
            ui.spacing_mut().icon_width_inner = 16.0;
            ui.spacing_mut().interact_size = egui::vec2(44.0, 44.0);
            let fill = if *checked {
                crate::TEXT
            } else {
                egui::Color32::TRANSPARENT
            };
            let ink = if *checked {
                egui::Color32::WHITE
            } else {
                crate::TEXT
            };
            let widgets = &mut ui.visuals_mut().widgets;
            for widget in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
            ] {
                widget.corner_radius = egui::CornerRadius::same(6);
                widget.bg_fill = fill;
                widget.bg_stroke = egui::Stroke::new(1.5_f32, egui::Color32::from_black_alpha(135));
                widget.fg_stroke = egui::Stroke::new(2.3_f32, ink);
            }
            let response = ui.add_sized([44.0, 44.0], egui::Checkbox::without_text(checked));
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox,
                    ui.is_enabled(),
                    *checked,
                    label,
                )
            });
            if response.has_focus() {
                ui.painter().rect_stroke(
                    response.rect.shrink(1.0),
                    8.0,
                    egui::Stroke::new(1.5_f32, crate::TEXT),
                    egui::StrokeKind::Inside,
                );
            }
            response.on_hover_text(if *checked {
                "Batalkan centang"
            } else {
                "Tandai selesai"
            })
        },
    )
    .inner
}
