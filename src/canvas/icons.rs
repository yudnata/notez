use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Vec2};

#[derive(Clone, Copy)]
pub(super) enum Icon {
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
    Ruler,
    Add,
}

pub(super) fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    shortcut: &str,
    selected: bool,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
    let response = ui.interact(rect, egui::Id::new(("canvas-tool", label)), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label)
    });
    if ui.is_rect_visible(rect) {
        let fill = if selected || response.is_pointer_button_down_on() {
            crate::BUTTON_ACTIVE
        } else if response.hovered() {
            crate::BUTTON_HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect(
            rect,
            8.0,
            fill,
            Stroke::new(0.7_f32, crate::BUTTON_BORDER),
            egui::StrokeKind::Inside,
        );
        paint(ui.painter(), rect.center(), icon);
        if selected {
            ui.painter()
                .circle_filled(rect.right_bottom() - Vec2::splat(6.0), 2.2, crate::TEXT);
        }
        if response.has_focus() {
            ui.painter().line_segment(
                [
                    rect.left_bottom() + Vec2::new(10.0, -3.0),
                    rect.right_bottom() + Vec2::new(-10.0, -3.0),
                ],
                Stroke::new(2.0_f32, crate::TEXT),
            );
        }
    }
    response.on_hover_text(if shortcut.is_empty() {
        label.to_owned()
    } else {
        format!("{label} ({shortcut})")
    })
}

fn paint(painter: &egui::Painter, center: Pos2, icon: Icon) {
    let origin = center - Vec2::splat(12.0);
    let p = |x, y| origin + Vec2::new(x, y);
    let stroke = Stroke::new(1.7_f32, crate::TEXT);
    let path = |points: &[[f32; 2]]| {
        painter.add(egui::Shape::line(
            points.iter().map(|[x, y]| p(*x, *y)).collect(),
            stroke,
        ));
    };
    match icon {
        Icon::Select => path(&[
            [4.0, 2.0],
            [5.0, 20.0],
            [10.0, 15.0],
            [14.0, 22.0],
            [17.0, 20.0],
            [13.0, 13.0],
            [20.0, 12.0],
            [4.0, 2.0],
        ]),
        Icon::Lasso => {
            painter.add(egui::Shape::ellipse_stroke(
                p(12.0, 9.0),
                Vec2::new(8.0, 6.0),
                stroke,
            ));
            path(&[[5.0, 13.0], [3.0, 18.0], [7.0, 21.0], [11.0, 19.0]]);
        }
        Icon::Pen => {
            path(&[
                [4.0, 20.0],
                [6.0, 13.0],
                [17.0, 2.0],
                [22.0, 7.0],
                [11.0, 18.0],
                [4.0, 20.0],
            ]);
            path(&[[6.0, 13.0], [11.0, 18.0]]);
            path(&[[14.0, 5.0], [19.0, 10.0]]);
        }
        Icon::Marker => {
            path(&[
                [5.0, 14.0],
                [15.0, 4.0],
                [21.0, 10.0],
                [11.0, 20.0],
                [5.0, 14.0],
            ]);
            path(&[[6.0, 15.0], [3.0, 21.0], [9.0, 18.0]]);
            path(&[[3.0, 22.0], [17.0, 22.0]]);
        }
        Icon::Eraser => {
            path(&[
                [3.0, 14.0],
                [12.0, 4.0],
                [21.0, 12.0],
                [12.0, 21.0],
                [9.0, 21.0],
                [3.0, 14.0],
            ]);
            path(&[[7.0, 10.0], [16.0, 17.0]]);
            path(&[[12.0, 21.0], [22.0, 21.0]]);
        }
        Icon::Sticky => {
            path(&[
                [4.0, 3.0],
                [20.0, 3.0],
                [20.0, 15.0],
                [14.0, 21.0],
                [4.0, 21.0],
                [4.0, 3.0],
            ]);
            path(&[[14.0, 21.0], [14.0, 15.0], [20.0, 15.0]]);
        }
        Icon::Text => {
            path(&[[5.0, 7.0], [5.0, 4.0], [19.0, 4.0], [19.0, 7.0]]);
            path(&[[12.0, 4.0], [12.0, 21.0]]);
            path(&[[8.0, 21.0], [16.0, 21.0]]);
        }
        Icon::Rectangle => {
            painter.rect_stroke(
                Rect::from_min_max(p(3.0, 5.0), p(21.0, 19.0)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Ellipse => {
            painter.add(egui::Shape::ellipse_stroke(
                center,
                Vec2::new(9.0, 7.0),
                stroke,
            ));
        }
        Icon::Diamond => path(&[
            [12.0, 2.0],
            [22.0, 12.0],
            [12.0, 22.0],
            [2.0, 12.0],
            [12.0, 2.0],
        ]),
        Icon::Arrow => {
            path(&[[4.0, 20.0], [20.0, 4.0]]);
            path(&[[9.0, 4.0], [20.0, 4.0], [20.0, 15.0]]);
        }
        Icon::Line => path(&[[4.0, 20.0], [20.0, 4.0]]),
        Icon::Pan => path(&[
            [5.0, 13.0],
            [5.0, 8.0],
            [8.0, 8.0],
            [8.0, 4.0],
            [11.0, 4.0],
            [11.0, 2.0],
            [14.0, 2.0],
            [14.0, 5.0],
            [17.0, 5.0],
            [17.0, 8.0],
            [20.0, 8.0],
            [20.0, 15.0],
            [16.0, 21.0],
            [9.0, 21.0],
            [3.0, 15.0],
            [3.0, 12.0],
            [5.0, 13.0],
            [8.0, 16.0],
        ]),
        Icon::Ruler => {
            path(&[
                [3.0, 16.0],
                [16.0, 3.0],
                [22.0, 9.0],
                [9.0, 22.0],
                [3.0, 16.0],
            ]);
            path(&[[7.0, 12.0], [10.0, 15.0]]);
            path(&[[11.0, 8.0], [13.0, 10.0]]);
            path(&[[15.0, 4.0], [18.0, 7.0]]);
        }
        Icon::Add => {
            path(&[[4.0, 12.0], [20.0, 12.0]]);
            path(&[[12.0, 4.0], [12.0, 20.0]]);
        }
    }
}
