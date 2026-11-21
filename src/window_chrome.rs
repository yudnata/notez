use egui::{Context, Pos2, Rect, ResizeDirection, Sense, Stroke, Ui, Vec2, ViewportCommand};

pub fn controls(ui: &mut Ui) -> Option<ViewportCommand> {
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let mut command = None;
    for (label, action) in [
        ("Tutup", ViewportCommand::Close),
        (
            if maximized { "Pulihkan" } else { "Maksimalkan" },
            ViewportCommand::Maximized(!maximized),
        ),
        ("Minimalkan", ViewportCommand::Minimized(true)),
    ] {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 28.0), Sense::hover());
        let response = ui
            .interact(
                rect,
                egui::Id::new(("window-control", label)),
                Sense::click(),
            )
            .on_hover_text(label);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });
        if response.hovered() || response.has_focus() {
            ui.painter().rect_filled(rect, 6.0, crate::BUTTON_ACTIVE);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                6.0,
                Stroke::new(1.0_f32, crate::TEXT),
                egui::StrokeKind::Inside,
            );
        }
        let center = rect.center();
        let stroke = Stroke::new(1.3_f32, crate::TEXT);
        match label {
            "Tutup" => {
                ui.painter().line_segment(
                    [center - Vec2::splat(5.0), center + Vec2::splat(5.0)],
                    stroke,
                );
                ui.painter().line_segment(
                    [center + Vec2::new(-5.0, 5.0), center + Vec2::new(5.0, -5.0)],
                    stroke,
                );
            }
            "Minimalkan" => {
                ui.painter().line_segment(
                    [center + Vec2::new(-5.0, 3.0), center + Vec2::new(5.0, 3.0)],
                    stroke,
                );
            }
            "Pulihkan" => {
                ui.painter().rect_stroke(
                    Rect::from_center_size(center + Vec2::new(2.0, -2.0), Vec2::splat(9.0)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                let front = Rect::from_center_size(center + Vec2::new(-1.0, 1.0), Vec2::splat(9.0));
                ui.painter().rect_filled(front, 0.0, crate::PANEL);
                ui.painter()
                    .rect_stroke(front, 0.0, stroke, egui::StrokeKind::Inside);
            }
            _ => {
                ui.painter().rect_stroke(
                    Rect::from_center_size(center, Vec2::new(11.0, 10.0)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
        }
        if response.clicked() {
            command = Some(action);
        }
    }
    command
}

pub fn drag_area(ui: &mut Ui, rect: Rect) {
    if !rect.is_positive() {
        return;
    }
    let response = ui.interact(
        rect,
        egui::Id::new("window-title-drag"),
        Sense::click_and_drag(),
    );
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if response.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

fn resize_direction(rect: Rect, pointer: Pos2) -> Option<ResizeDirection> {
    if !rect.contains(pointer) {
        return None;
    }
    let left = pointer.x < rect.left() + 5.0;
    let right = pointer.x > rect.right() - 5.0;
    let top = pointer.y < rect.top() + 5.0;
    let bottom = pointer.y > rect.bottom() - 5.0;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    }
}

pub fn resize_edges(ctx: &Context) {
    if ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let direction = ctx
        .input(|i| i.pointer.hover_pos())
        .and_then(|pointer| resize_direction(ctx.content_rect(), pointer));
    if let Some(direction) = direction {
        ctx.set_cursor_icon(match direction {
            ResizeDirection::North | ResizeDirection::South => egui::CursorIcon::ResizeVertical,
            ResizeDirection::East | ResizeDirection::West => egui::CursorIcon::ResizeHorizontal,
            ResizeDirection::NorthWest | ResizeDirection::SouthEast => egui::CursorIcon::ResizeNwSe,
            ResizeDirection::NorthEast | ResizeDirection::SouthWest => egui::CursorIcon::ResizeNeSw,
        });
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_buttons_drag_and_resize_emit_the_expected_native_commands() {
        let ctx = Context::default();
        crate::configure(&ctx);
        let run = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    resize_edges(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(command) = controls(ui) {
                                        ctx.send_viewport_cmd(command);
                                    }
                                },
                            );
                        });
                        drag_area(
                            ui,
                            Rect::from_min_size(Pos2::new(160.0, 60.0), Vec2::new(300.0, 34.0)),
                        );
                    });
                },
            )
        };
        let has = |output: &egui::FullOutput, command| {
            output
                .viewport_output
                .values()
                .any(|viewport| viewport.commands.contains(&command))
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(vec![]);
        for (label, command) in [
            ("Minimalkan", ViewportCommand::Minimized(true)),
            ("Maksimalkan", ViewportCommand::Maximized(true)),
            ("Tutup", ViewportCommand::Close),
        ] {
            let at = ctx
                .read_response(egui::Id::new(("window-control", label)))
                .unwrap()
                .rect
                .center();
            run(vec![egui::Event::PointerMoved(at), button(at, true)]);
            let output = run(vec![button(at, false)]);
            assert!(has(&output, command), "{label}");
        }
        for _ in 0..45 {
            run(vec![]);
        }
        let at = Pos2::new(240.0, 75.0);
        run(vec![egui::Event::PointerMoved(at), button(at, true)]);
        run(vec![button(at, false)]);
        run(vec![button(at, true)]);
        let output = run(vec![button(at, false)]);
        assert!(has(&output, ViewportCommand::Maximized(true)));
        run(vec![egui::Event::PointerMoved(at), button(at, true)]);
        let output = run(vec![egui::Event::PointerMoved(at + Vec2::new(35.0, 0.0))]);
        assert!(has(&output, ViewportCommand::StartDrag));
        run(vec![button(at + Vec2::new(35.0, 0.0), false)]);
        let corner = Pos2::new(2.0, 2.0);
        let output = run(vec![
            egui::Event::PointerMoved(corner),
            button(corner, true),
        ]);
        assert!(has(
            &output,
            ViewportCommand::BeginResize(ResizeDirection::NorthWest)
        ));
        assert_eq!(
            resize_direction(
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.0)),
                Pos2::new(250.0, 250.0)
            ),
            None
        );
    }
}
