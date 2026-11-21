use crate::model::{ImageLayout, NoteImage, RichDocument};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

#[derive(Default)]
pub struct NoteImages {
    pub selected: Option<u64>,
    textures: HashMap<
        u64,
        (
            crate::canvas::model::ImageData,
            Result<egui::TextureHandle, String>,
        ),
    >,
    drag: Option<(u64, [f32; 2])>,
    gesture_before: Option<RichDocument>,
}

impl NoteImages {
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
        self.textures
            .retain(|id, _| doc.images.iter().any(|image| image.id == *id));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.selected = None;
        }
        let mut changed = false;
        let mut over_image = false;
        let mut bottom = ui.min_rect().bottom();
        let before = doc.clone();
        for image in &mut doc.images {
            let rect = image_rect(image, output, available);
            bottom = bottom.max(rect.bottom() + 14.0);
            let response = ui
                .interact(
                    rect,
                    ui.id().with(("note-image", image.id)),
                    Sense::click_and_drag(),
                )
                .on_hover_cursor(if image.layout == ImageLayout::Overlay {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::PointingHand
                });
            over_image |= response.contains_pointer();
            if response.clicked() || response.drag_started() {
                self.selected = Some(image.id);
                output.response.surrender_focus();
            }
            if image.layout == ImageLayout::Overlay {
                if response.drag_started() {
                    self.drag = Some((image.id, image.offset));
                    self.gesture_before = Some(before.clone());
                }
                if response.dragged()
                    && let Some((id, start)) = self.drag
                    && id == image.id
                {
                    let delta = response.total_drag_delta().unwrap_or_default();
                    let anchor_y = output
                        .galley
                        .pos_from_cursor(egui::text::CCursor::new(image.anchor))
                        .top();
                    let offset = [
                        (start[0] + delta.x).clamp(0.0, (available - rect.width()).max(0.0)),
                        (start[1] + delta.y).max(-anchor_y),
                    ];
                    changed |= image.offset != offset;
                    image.offset = offset;
                }
            }
            if response.drag_stopped() {
                self.drag = None;
            }
            let rect = image_rect(image, output, available);
            if self
                .textures
                .get(&image.id)
                .is_none_or(|(data, _)| data != &image.data)
            {
                let texture = image::load_from_memory(&image.data.bytes)
                    .map(|decoded| {
                        let rgba = decoded.to_rgba8();
                        ui.ctx().load_texture(
                            format!("note-image-{}", image.id),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [rgba.width() as usize, rgba.height() as usize],
                                &rgba,
                            ),
                            egui::TextureOptions::LINEAR,
                        )
                    })
                    .map_err(|e| e.to_string());
                self.textures
                    .insert(image.id, (image.data.clone(), texture));
            }
            match &self.textures[&image.id].1 {
                Ok(texture) => {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                Err(error) => {
                    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(242));
                    ui.put(rect, egui::Label::new("Gambar tidak bisa dibaca"))
                        .on_hover_text(error);
                }
            }
            if self.selected == Some(image.id) {
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    Stroke::new(1.0_f32, Color32::from_gray(95)),
                    egui::StrokeKind::Outside,
                );
                let handle = Rect::from_center_size(rect.right_bottom(), Vec2::splat(14.0));
                let resize = ui
                    .interact(
                        handle,
                        ui.id().with(("resize-note-image", image.id)),
                        Sense::drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                over_image |= resize.contains_pointer();
                if resize.drag_started() {
                    self.gesture_before = Some(before.clone());
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
                    let size = image.data.resize_corner([
                        (pointer.x - rect.left()).max(1.0),
                        (pointer.y - rect.top()).max(1.0),
                    ]);
                    let width = size[0].clamp(30.0_f32.min(available), available.max(1.0));
                    changed |= image.width != width;
                    image.width = width;
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
        Vec2::new(0.0, 6.0)
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
}
