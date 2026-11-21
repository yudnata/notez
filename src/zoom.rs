use crate::navigation::PanScrollArea;
use egui::{LayerId, Rect, Ui, UiBuilder, emath::TSTransform};

pub fn show<R>(ui: &mut Ui, zoom: &mut f32, contents: impl FnOnce(&mut Ui) -> R) -> R {
    let rect = ui.available_rect_before_wrap();
    let layer = LayerId::new(ui.layer_id().order, ui.id().with("zoomed-content"));
    let hovered = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| {
        rect.intersect(ui.clip_rect()).contains(p)
            && ui
                .ctx()
                .layer_id_at(p)
                .is_some_and(|at| at == layer || at == ui.layer_id())
    });
    if hovered {
        let factor = ui.input(|i| i.zoom_delta());
        *zoom = (*zoom * factor).clamp(0.5, 2.0);
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num0)) {
        *zoom = 1.0;
    }
    let transform = TSTransform::from_translation(rect.min.to_vec2())
        * TSTransform::from_scaling(*zoom)
        * TSTransform::from_translation(-rect.min.to_vec2());
    ui.ctx().set_sublayer(ui.layer_id(), layer);
    ui.ctx().set_transform_layer(layer, transform);
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("zoomed-content")
            .layer_id(layer)
            .max_rect(Rect::from_min_size(rect.min, rect.size() / *zoom))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(transform.inverse() * rect.intersect(ui.clip_rect()));
    let local_size = rect.size() / *zoom;
    let result = if *zoom > 1.001 && (local_size.x < 320.0 || local_size.y < 440.0) {
        egui::ScrollArea::both()
            .id_salt("zoom-overflow")
            .auto_shrink([false, false])
            .show_pannable(&mut child, [true, true], |ui| {
                ui.allocate_ui_with_layout(
                    local_size.max(egui::vec2(320.0, 600.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    contents,
                )
                .inner
            })
            .inner
    } else {
        contents(&mut child)
    };
    ui.allocate_rect(rect, egui::Sense::hover());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::PanScrollArea;

    #[test]
    fn scaled_content_keeps_clicks_and_right_drag_in_sync() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut zoom = 1.5;
        let mut checked = false;
        let mut checkbox = Rect::NOTHING;
        let mut offset = egui::Vec2::ZERO;
        let mut run = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut zoom, |ui| {
                            let output = egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show_pannable(ui, [false, true], |ui| {
                                    let response = ui.checkbox(&mut checked, "Tugas");
                                    checkbox =
                                        ui.ctx().layer_transform_to_global(ui.layer_id()).unwrap()
                                            * response.rect;
                                    ui.allocate_exact_size(
                                        egui::vec2(200.0, 1400.0),
                                        egui::Sense::hover(),
                                    );
                                });
                            offset = output.state.offset;
                        });
                    });
                },
            );
            (checkbox, checked, offset)
        };
        let (rect, _, _) = run(vec![]);
        let button = |pos, button, pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(vec![
            egui::Event::PointerMoved(rect.center()),
            button(rect.center(), egui::PointerButton::Primary, true),
        ]);
        let (_, checked, _) = run(vec![button(
            rect.center(),
            egui::PointerButton::Primary,
            false,
        )]);
        assert!(checked);
        let from = egui::pos2(280.0, 300.0);
        let to = from - egui::vec2(0.0, 90.0);
        run(vec![
            egui::Event::PointerMoved(from),
            button(from, egui::PointerButton::Secondary, true),
        ]);
        let (_, _, offset) = run(vec![egui::Event::PointerMoved(to)]);
        assert!((offset.y - 60.0).abs() < 0.01, "{offset:?}");
        run(vec![button(to, egui::PointerButton::Secondary, false)]);
        run(vec![
            egui::Event::PointerMoved(from),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -6.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let (_, _, offset) = run(vec![]);
        assert!((offset.y - 64.0).abs() < 0.01, "{offset:?}");
        run(vec![
            egui::Event::PointerMoved(from),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 6.0),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let (_, _, offset) = run(vec![]);
        assert!((offset.y - 60.0).abs() < 0.01, "{offset:?}");
    }
}
