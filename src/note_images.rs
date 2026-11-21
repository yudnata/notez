use crate::{
    canvas::model::{TextureSlot, paint_image_loading},
    model::{ImageLayout, NoteImage, RichDocument},
};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

#[derive(Default)]
pub struct NoteImages {
    pub selected: Option<u64>,
    pub text_selection: std::ops::Range<usize>,
    pub last_available_width: f32,
    textures: HashMap<(usize, usize), TextureSlot>,
    drag: Option<(u64, [f32; 2])>,
    gesture_before: Option<RichDocument>,
}

impl NoteImages {
    pub fn reset_interaction(&mut self) {
        self.selected = None;
        self.text_selection = 0..0;
        self.drag = None;
        self.gesture_before = None;
    }

    pub fn is_loading(&self) -> bool {
        self.textures.values().any(TextureSlot::is_loading)
    }

    pub fn dragging(&self) -> bool {
        self.gesture_before.is_some()
    }

    pub fn finish_gesture(&mut self, ui: &egui::Ui, doc: &RichDocument) -> Option<RichDocument> {
        if !ui.input(|i| i.pointer.primary_down()) {
            return self.gesture_before.take().filter(|before| before != doc);
        }
        None
    }

    pub fn toolbar(&mut self, ui: &mut egui::Ui, doc: &mut RichDocument) -> bool {
        let Some(image) = doc
            .images
            .iter_mut()
            .find(|image| Some(image.id) == self.selected)
        else {
            self.selected = None;
            return false;
        };
        let mut changed = false;
        let mut delete = false;
        ui.horizontal_wrapped(|ui| {
            changed |= ui
                .selectable_value(&mut image.layout, ImageLayout::Text, "Layout teks")
                .changed();
            changed |= ui
                .selectable_value(&mut image.layout, ImageLayout::Overlay, "Overlay teks")
                .changed();
            ui.label("Lebar");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut image.width)
                        .range(30.0..=4000.0)
                        .suffix(" px"),
                )
                .changed();
            if ui
                .add(crate::controls::button(ui, "Hapus gambar"))
                .clicked()
            {
                delete = true;
            }
            if ui
                .small_button("×")
                .on_hover_text("Selesai mengatur gambar · Esc")
                .clicked()
            {
                self.selected = None;
            }
        });
        if delete {
            let id = image.id;
            doc.remove_image(id);
            self.selected = None;
            changed = true;
        }
        changed
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut RichDocument,
        output: &egui::text_edit::TextEditOutput,
        available: f32,
    ) -> bool {
        self.last_available_width = available;
        if self.textures.len() > 64 {
            self.textures.retain(|key, _| {
                doc.images
                    .iter()
                    .any(|image| image.data.cache_key() == *key)
            });
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.selected = None;
        }
        let mut changed = false;
        let mut over_image = false;
        let mut bottom = ui.min_rect().bottom();
        let mut snapshot = None;
        for index in 0..doc.images.len() {
            let rect = image_rect(&doc.images[index], output, available);
            bottom = bottom.max(rect.bottom() + 14.0);
            let image_id = doc.images[index].id;
            let layout = doc.images[index].layout;
            let response = ui
                .interact(
                    rect,
                    ui.id().with(("note-image", image_id)),
                    Sense::click_and_drag(),
                )
                .on_hover_cursor(if layout == ImageLayout::Overlay {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::PointingHand
                });
            over_image |= response.contains_pointer();
            if response.clicked() || response.drag_started() {
                self.selected = Some(image_id);
                output.response.surrender_focus();
            }
            if layout == ImageLayout::Overlay {
                if response.drag_started() {
                    let before = snapshot.get_or_insert_with(|| doc.clone()).clone();
                    self.drag = Some((image_id, doc.images[index].offset));
                    self.gesture_before = Some(before);
                }
                if response.dragged()
                    && let Some((id, start)) = self.drag
                    && id == image_id
                {
                    let delta = response.total_drag_delta().unwrap_or_default();
                    let anchor_y = output
                        .galley
                        .pos_from_cursor(egui::text::CCursor::new(doc.images[index].anchor))
                        .top();
                    let offset = [
                        (start[0] + delta.x).clamp(0.0, (available - rect.width()).max(0.0)),
                        (start[1] + delta.y).max(-anchor_y),
                    ];
                    changed |= doc.images[index].offset != offset;
                    doc.images[index].offset = offset;
                }
            }
            if response.drag_stopped() {
                self.drag = None;
            }
            let rect = image_rect(&doc.images[index], output, available);
            let slot = doc.images[index].data.poll_texture(
                &mut self.textures,
                ui.ctx(),
                &format!("note-image-{image_id}"),
            );
            match slot {
                TextureSlot::Loading(_) => {
                    paint_image_loading(ui.painter(), ui.ctx(), rect);
                }
                TextureSlot::Ready(texture) => {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                TextureSlot::Failed(error) => {
                    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(242));
                    ui.put(rect, egui::Label::new("Gambar tidak bisa dibaca"))
                        .on_hover_text(error.as_str());
                }
            }
            let in_text_selection = !self.text_selection.is_empty()
                && self.text_selection.contains(&doc.images[index].anchor);
            if in_text_selection {
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(113, 79, 171, 32),
                );
            }
            if self.selected == Some(image_id) || in_text_selection {
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    Stroke::new(1.2_f32, Color32::from_rgb(113, 79, 171)),
                    egui::StrokeKind::Outside,
                );
                let handle = Rect::from_center_size(rect.right_bottom(), Vec2::splat(14.0));
                let resize = ui
                    .interact(
                        handle,
                        ui.id().with(("resize-note-image", image_id)),
                        Sense::drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                over_image |= resize.contains_pointer();
                if resize.drag_started() {
                    let before = snapshot.get_or_insert_with(|| doc.clone()).clone();
                    self.gesture_before = Some(before);
                }
                ui.painter()
                    .rect_filled(handle.shrink(2.0), 2.0, Color32::WHITE);
                ui.painter().rect_stroke(
                    handle.shrink(2.0),
                    2.0,
                    Stroke::new(1.0_f32, Color32::from_gray(95)),
                    egui::StrokeKind::Inside,
                );
                if resize.dragged()
                    && let Some(pointer) = resize.interact_pointer_pos()
                {
                    let size = doc.images[index].data.resize_corner([
                        (pointer.x - rect.left()).max(1.0),
                        (pointer.y - rect.top()).max(1.0),
                    ]);
                    let width = size[0].clamp(30.0_f32.min(available), available.max(1.0));
                    changed |= doc.images[index].width != width;
                    doc.images[index].width = width;
                }
            }
        }
        if ui.input(|i| i.pointer.primary_clicked())
            && !over_image
            && output.response.contains_pointer()
        {
            self.selected = None;
        }
        if bottom > ui.min_rect().bottom() {
            ui.allocate_space(egui::vec2(0.0, bottom - ui.min_rect().bottom()));
        }
        changed
    }
}

pub fn image_rect(
    image: &NoteImage,
    output: &egui::text_edit::TextEditOutput,
    available: f32,
) -> Rect {
    let cursor = output.galley.pos_from_cursor(egui::text::CCursor {
        index: image.anchor,
        prefer_next_row: true,
    });
    let size = image.display_size(available);
    let offset = if image.layout == ImageLayout::Overlay {
        egui::vec2(
            image.offset[0].clamp(0.0, (available - size.x).max(0.0)),
            image.offset[1].max(-cursor.top()),
        )
    } else {
        let x = match image.align {
            crate::model::TextAlign::Left | crate::model::TextAlign::Justify => 0.0,
            crate::model::TextAlign::Center => ((available - size.x) * 0.5).max(0.0),
            crate::model::TextAlign::Right => (available - size.x).max(0.0),
        };
        Vec2::new(x, 6.0)
    };
    Rect::from_min_size(
        output.galley_pos + egui::vec2(0.0, cursor.top()) + offset,
        size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TextStyle, test_image};

    fn frame(
        ctx: &egui::Context,
        editor: &mut NoteImages,
        doc: &mut RichDocument,
        width: f32,
        events: Vec<egui::Event>,
    ) -> (Rect, f32) {
        let mut geometry = (Rect::NOTHING, 0.0);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 700.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let available = ui.available_width() - 8.0;
                    let before = doc.clone();
                    let mut text = doc.text.clone();
                    let mut layouter = |ui: &egui::Ui, _: &dyn egui::TextBuffer, width| {
                        ui.fonts_mut(|fonts| {
                            fonts.layout_job(crate::layout_job(&before, width, 1.5))
                        })
                    };
                    let output = egui::TextEdit::multiline(&mut text)
                        .desired_width(available)
                        .frame(false)
                        .layouter(&mut layouter)
                        .show(ui);
                    geometry.0 = image_rect(&doc.images[0], &output, available);
                    geometry.1 = output.galley_pos.y
                        + output
                            .galley
                            .pos_from_cursor(egui::text::CCursor::new(doc.images[0].anchor + 2))
                            .top();
                    editor.show(ui, doc, &output, available);
                });
            },
        );
        geometry
    }

    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn inline_image_reserves_text_space_fits_narrow_notes_and_resizes_proportionally() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = NoteImages::default();
        let mut doc = RichDocument::default();
        doc.reconcile("Di atas\nDi bawah".into(), TextStyle::default());
        let id = doc.insert_image(8, test_image(), "foto.png".into(), TextStyle::default());
        editor.selected = Some(id);
        for width in [620.0, 210.0] {
            let (rect, below) = frame(&ctx, &mut editor, &mut doc, width, vec![]);
            assert!(rect.left() >= 0.0 && rect.right() <= width, "{rect:?}");
            assert!(
                below >= rect.bottom(),
                "Text below ({below}) overlaps image {rect:?}"
            );
            assert!((rect.width() / rect.height() - 2.0).abs() < 0.001);
        }
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        let (rect, _) = frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        let from = rect.right_bottom();
        let to = from + egui::vec2(100.0, 80.0);
        frame(
            &ctx,
            &mut editor,
            &mut doc,
            620.0,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        frame(
            &ctx,
            &mut editor,
            &mut doc,
            620.0,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![button(to, false)]);
        assert!(doc.images[0].width > 320.0);
        let (resized, below) = frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        assert!((resized.width() / resized.height() - 2.0).abs() < 0.001);
        assert!(below >= resized.bottom());
    }

    #[test]
    fn overlay_can_move_over_text_without_changing_text_flow() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = NoteImages::default();
        let mut doc = RichDocument::default();
        doc.insert_image(0, test_image(), "foto.png".into(), TextStyle::default());
        doc.reconcile(
            format!("{}Teks di belakang gambar", doc.text),
            TextStyle::default(),
        );
        doc.images[0].layout = ImageLayout::Overlay;
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        let (rect, below) = frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        assert!(below < rect.bottom());
        let from = rect.center();
        let to = from + egui::vec2(70.0, 50.0);
        frame(
            &ctx,
            &mut editor,
            &mut doc,
            620.0,
            vec![egui::Event::PointerMoved(from), button(from, true)],
        );
        frame(
            &ctx,
            &mut editor,
            &mut doc,
            620.0,
            vec![egui::Event::PointerMoved(to)],
        );
        let to = to + egui::vec2(20.0, 10.0);
        frame(
            &ctx,
            &mut editor,
            &mut doc,
            620.0,
            vec![egui::Event::PointerMoved(to)],
        );
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        assert_eq!(doc.images[0].offset, [90.0, 60.0]);
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![button(to, false)]);
        assert_eq!(editor.selected, Some(doc.images[0].id));
        assert_eq!(doc.images[0].offset, [90.0, 60.0]);
        let (_, after) = frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        assert_eq!(below, after);
    }

    #[test]
    fn loading_spinner_renders_while_decoding_and_resolves_to_texture() {
        fn has_loading_text(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Text(text) => text.galley.job.text == "Memuat gambar...",
                egui::Shape::Vec(shapes) => shapes.iter().any(has_loading_text),
                _ => false,
            }
        }
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut editor = NoteImages::default();
        let mut doc = RichDocument::default();
        let img = test_image();
        let key = img.cache_key();
        doc.insert_image(0, img, "foto.png".into(), TextStyle::default());
        let pending = std::sync::Arc::new(std::sync::Mutex::new(None));
        editor
            .textures
            .insert(key, TextureSlot::Loading(pending.clone()));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(620.0, 700.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let available = ui.available_width() - 8.0;
                    let before = doc.clone();
                    let mut text = doc.text.clone();
                    let mut layouter = |ui: &egui::Ui, _: &dyn egui::TextBuffer, width| {
                        ui.fonts_mut(|fonts| {
                            fonts.layout_job(crate::layout_job(&before, width, 1.5))
                        })
                    };
                    let edit = egui::TextEdit::multiline(&mut text)
                        .desired_width(available)
                        .frame(false)
                        .layouter(&mut layouter)
                        .show(ui);
                    editor.show(ui, &mut doc, &edit, available);
                });
            },
        );
        assert!(editor.is_loading());
        assert!(output.shapes.iter().any(|s| has_loading_text(&s.shape)));
        *pending.lock().unwrap() = Some(crate::canvas::model::ImageData::decode_color_image(
            &doc.images[0].data.bytes,
        ));
        frame(&ctx, &mut editor, &mut doc, 620.0, vec![]);
        assert!(!editor.is_loading());
        assert!(matches!(
            editor.textures.get(&key),
            Some(TextureSlot::Ready(_))
        ));
    }
}
