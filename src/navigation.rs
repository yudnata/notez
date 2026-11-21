use egui::{CursorIcon, Id, PointerButton, Pos2, Rect, ScrollArea, Ui, Vec2};

#[derive(Clone, Copy)]
struct RightDrag {
    origin: Pos2,
    last: Pos2,
    dragging: bool,
}

pub fn local_pointer(ui: &Ui, point: Pos2) -> Pos2 {
    ui.ctx()
        .layer_transform_from_global(ui.layer_id())
        .map_or(point, |transform| transform * point)
}

fn local_delta(ui: &Ui, delta: Vec2) -> Vec2 {
    ui.ctx()
        .layer_transform_from_global(ui.layer_id())
        .map_or(delta, |transform| delta * transform.scaling)
}

fn layer_visible_at(ui: &Ui, point: Pos2) -> bool {
    let at = ui.ctx().layer_id_at(point);
    at == Some(ui.layer_id())
        || at.is_some_and(|at| {
            ui.ctx()
                .memory(|memory| memory.areas().parent_layer(ui.layer_id()) == Some(at))
        })
}

pub fn right_drag(ui: &Ui, id: Id, rect: Rect) -> Vec2 {
    let key = id.with("right-drag");
    let (pressed, down, released, pointer, primary) = ui.input(|i| {
        (
            i.pointer.button_pressed(PointerButton::Secondary),
            i.pointer.secondary_down(),
            i.pointer.button_released(PointerButton::Secondary),
            i.pointer.interact_pos(),
            i.pointer.primary_down() || i.pointer.middle_down(),
        )
    });
    let mut gesture = ui.ctx().data(|d| d.get_temp::<RightDrag>(key));
    if pressed && !primary && ui.is_enabled() {
        gesture = pointer
            .filter(|p| {
                rect.intersect(ui.clip_rect())
                    .contains(local_pointer(ui, *p))
                    && layer_visible_at(ui, *p)
            })
            .map(|p| RightDrag {
                origin: local_pointer(ui, p),
                last: local_pointer(ui, p),
                dragging: false,
            });
    }
    let mut delta = Vec2::ZERO;
    if let (Some(state), Some(pointer)) = (&mut gesture, pointer) {
        let pointer = local_pointer(ui, pointer);
        if down && !primary {
            state.dragging |= pointer.distance(state.origin) > local_delta(ui, Vec2::splat(6.0)).x;
            if state.dragging {
                delta = pointer - state.last;
                state.last = pointer;
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
        }
    }
    if released || !down || primary {
        gesture = None;
    }
    ui.ctx().data_mut(|d| {
        if let Some(gesture) = gesture {
            d.insert_temp(key, gesture);
        } else {
            d.remove::<RightDrag>(key);
        }
    });
    delta
}

pub trait PanScrollArea {
    fn show_pannable<R>(
        self,
        ui: &mut Ui,
        axes: [bool; 2],
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R>;
}

impl PanScrollArea for ScrollArea {
    fn show_pannable<R>(
        self,
        ui: &mut Ui,
        axes: [bool; 2],
        contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        let touch = ui.input(|i| i.multi_touch());
        let right = ui.input(|i| {
            i.pointer.secondary_down() || i.pointer.button_released(PointerButton::Secondary)
        });
        let mut output = self
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: touch.is_none() && !right,
                ..Default::default()
            })
            .show(ui, contents);
        let mut delta = right_drag(ui, output.id, output.inner_rect);
        if let Some(touch) = touch.filter(|t| {
            output
                .inner_rect
                .intersect(ui.clip_rect())
                .contains(local_pointer(ui, t.start_pos))
                && layer_visible_at(ui, t.start_pos)
        }) {
            delta += local_delta(ui, touch.translation_delta);
        }
        for axis in 0..2 {
            if axes[axis] && delta[axis] != 0.0 {
                let max = (output.content_size[axis] - output.inner_rect.size()[axis]).max(0.0);
                output.state.offset[axis] =
                    (output.state.offset[axis] - delta[axis]).clamp(0.0, max);
            }
        }
        if delta != Vec2::ZERO {
            output.state.store(ui.ctx(), output.id);
            ui.ctx().request_repaint();
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        axes: [bool; 2],
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        let mut output = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 400.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    output = Some(ScrollArea::both().id_salt("navigation-test").show_pannable(
                        ui,
                        axes,
                        |ui| {
                            ui.allocate_exact_size(Vec2::new(1100.0, 1200.0), egui::Sense::hover());
                        },
                    ));
                });
            },
        );
        output.unwrap()
    }
    fn right(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn right_drag_scrolls_supported_axes_once_and_clamps_at_edges() {
        for axes in [[true, true], [true, false], [false, true]] {
            let ctx = egui::Context::default();
            let viewport = frame(&ctx, vec![], axes).inner_rect;
            let at = viewport.center();
            frame(
                &ctx,
                vec![egui::Event::PointerMoved(at), right(at, true)],
                axes,
            );
            let to = at - Vec2::new(60.0, 80.0);
            let output = frame(&ctx, vec![egui::Event::PointerMoved(to)], axes);
            assert_eq!(
                output.state.offset,
                Vec2::new(
                    if axes[0] { 60.0 } else { 0.0 },
                    if axes[1] { 80.0 } else { 0.0 }
                )
            );
            let output = frame(
                &ctx,
                vec![egui::Event::PointerMoved(Pos2::new(-5000.0, -5000.0))],
                axes,
            );
            let max = output.content_size - output.inner_rect.size();
            assert_eq!(
                output.state.offset,
                Vec2::new(
                    if axes[0] { max.x } else { 0.0 },
                    if axes[1] { max.y } else { 0.0 }
                )
            );
            frame(&ctx, vec![right(Pos2::new(-5000.0, -5000.0), false)], axes);
            let stopped = frame(&ctx, vec![egui::Event::PointerMoved(at)], axes);
            assert_eq!(stopped.state.offset, output.state.offset);
        }
    }
    #[test]
    fn two_finger_swipe_scrolls_without_mouse_buttons() {
        let ctx = egui::Context::default();
        let at = frame(&ctx, vec![], [true, true]).inner_rect.center();
        frame(&ctx, vec![egui::Event::PointerMoved(at)], [true, true]);
        let touch = |id, phase, pos| egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(id),
            phase,
            pos,
            force: None,
        };
        frame(
            &ctx,
            vec![
                egui::Event::PointerMoved(at),
                touch(1, egui::TouchPhase::Start, at - Vec2::new(20.0, 0.0)),
                touch(2, egui::TouchPhase::Start, at + Vec2::new(20.0, 0.0)),
            ],
            [true, true],
        );
        let to = at - Vec2::new(45.0, 70.0);
        let output = frame(
            &ctx,
            vec![
                touch(1, egui::TouchPhase::Move, to - Vec2::new(20.0, 0.0)),
                touch(2, egui::TouchPhase::Move, to + Vec2::new(20.0, 0.0)),
            ],
            [true, true],
        );
        assert_eq!(output.state.offset, Vec2::new(45.0, 70.0));
    }
}
