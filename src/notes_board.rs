use crate::model::{Document, Kind, NOTE_COLORS};
use crate::navigation::PanScrollArea;
use egui::{Color32, FontFamily, FontId, Rect, RichText, Sense, Stroke, Vec2};

pub struct NotesBoard {
    pub overview: bool,
    pub query: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;
    #[test]
    fn title_search_filters_cards_and_preserves_the_original_board_positions() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut workspace = crate::model::Workspace::default();
        let mut ids = vec![];
        for title in ["Rencana liburan", "猫 Proyek kerja", "Agenda harian"] {
            ids.push(workspace.add(Kind::Note));
            workspace.documents.last_mut().unwrap().title = title.into();
        }
        workspace.documents[1].note_position = Some([4000.0, 4000.0]);
        workspace.documents[2]
            .body
            .reconcile("猫 Proyek kerja".into(), crate::model::TextStyle::default());
        let before = workspace.clone();
        let mut board = NotesBoard::default();
        let run = |board: &mut NotesBoard, workspace: &mut crate::model::Workspace, events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        board.show(ui, &mut workspace.documents);
                    });
                },
            );
        };
        run(&mut board, &mut workspace, vec![]);
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("notes-search")));
        run(
            &mut board,
            &mut workspace,
            vec![egui::Event::Text("猫 KERJA".into())],
        );
        run(&mut board, &mut workspace, vec![]);
        assert_eq!(board.query, "猫 KERJA");
        assert!(
            ctx.read_response(egui::Id::new(("note-card", ids[0])))
                .is_none()
        );
        assert!(
            ctx.read_response(egui::Id::new(("note-card", ids[2])))
                .is_none()
        );
        let matched = ctx
            .read_response(egui::Id::new(("note-card", ids[1])))
            .unwrap();
        assert!(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)).contains_rect(matched.rect)
        );
        assert_eq!(workspace, before);
        board.query = "Tidak ditemukan".into();
        for _ in 0..2 {
            run(&mut board, &mut workspace, vec![]);
        }
        for id in &ids {
            assert!(
                ctx.read_response(egui::Id::new(("note-card", *id)))
                    .is_none()
            );
        }
        board.query.clear();
        for _ in 0..2 {
            run(&mut board, &mut workspace, vec![]);
        }
        for id in ids {
            assert!(
                ctx.read_response(egui::Id::new(("note-card", id)))
                    .is_some()
            );
        }
        assert_eq!(workspace, before);
    }

    #[test]
    fn right_drag_on_a_card_scrolls_without_repositioning_or_opening_it() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut workspace = crate::model::Workspace::default();
        for _ in 0..10 {
            workspace.add(Kind::Note);
        }
        let before = workspace.clone();
        let id = workspace.documents[0].id;
        let mut board = NotesBoard::default();
        let mut result = BoardResponse::default();
        let mut run = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        result = board.show(ui, &mut workspace.documents);
                    });
                },
            );
        };
        run(vec![]);
        let at = ctx
            .read_response(egui::Id::new(("note-card", id)))
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
        let to = at - Vec2::new(60.0, 80.0);
        run(vec![egui::Event::PointerMoved(to)]);
        run(vec![right(to, false)]);
        run(vec![]);
        run(vec![]);
        assert_eq!(workspace, before);
        assert!(!result.changed && result.open.is_none() && result.delete.is_none());
        let moved = ctx
            .read_response(egui::Id::new(("note-card", id)))
            .unwrap()
            .rect;
        assert!(moved.center().y < at.y);
    }
    #[test]
    fn ten_readable_cards_open_a_full_note_on_click() {
        let ctx = egui::Context::default();
        crate::configure(&ctx);
        let mut workspace = crate::model::Workspace::default();
        for i in 0..10 {
            workspace.add(Kind::Note);
            workspace.documents.last_mut().unwrap().title = format!("Catatan {}", i + 1);
        }
        let mut board = NotesBoard::default();
        let id = workspace.documents[0].id;
        let mut opened = None;
        let mut run = |events: Vec<egui::Event>| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 800.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        opened = board.show(ui, &mut workspace.documents).open;
                    });
                },
            );
        };
        run(vec![]);
        let rect = ctx
            .read_response(egui::Id::new(("note-card", id)))
            .unwrap()
            .rect;
        let at = rect.center();
        run(vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        run(vec![egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(opened, Some(id));
        assert_eq!(rect.size(), Vec2::new(208.0, 160.0));
        assert_ne!(
            workspace.documents[0].note_color,
            workspace.documents[1].note_color
        );
    }
}
impl Default for NotesBoard {
    fn default() -> Self {
        Self {
            overview: true,
            query: String::new(),
        }
    }
}
#[derive(Default)]
pub struct BoardResponse {
    pub open: Option<u64>,
    pub delete: Option<u64>,
    pub changed: bool,
    pub create: bool,
}

pub fn note_palette(ui: &mut egui::Ui, selected: &mut [u8; 3]) -> bool {
    let mut changed = false;
    for (c, name) in NOTE_COLORS.into_iter().zip(crate::model::NOTE_COLOR_NAMES) {
        let response = ui.add(
            egui::Button::new(
                RichText::new(format!(
                    "{}  {name}",
                    if *selected == c { "✓" } else { " " }
                ))
                .color(Color32::BLACK),
            )
            .fill(Color32::from_rgb(c[0], c[1], c[2]))
            .min_size(Vec2::new(160.0, 32.0)),
        );
        if response.clicked() {
            *selected = c;
            changed = true;
            ui.close();
        }
    }
    changed
}

fn checklist_preview(painter: &egui::Painter, rect: Rect, clip: Rect, doc: &Document, zoom: f32) {
    let painter = painter.with_clip_rect(
        Rect::from_min_max(
            rect.min + Vec2::new(16.0, 88.0 * zoom),
            rect.max - Vec2::splat(16.0),
        )
        .intersect(clip),
    );
    let done = doc.tasks.iter().filter(|task| task.done).count();
    let font = FontId::proportional(16.0 * zoom);
    painter.text(
        rect.min + Vec2::new(16.0, 88.0 * zoom),
        egui::Align2::LEFT_TOP,
        format!("{done}/{} selesai", doc.tasks.len()),
        font.clone(),
        crate::TEXT,
    );
    for (index, task) in doc.tasks.iter().take(3).enumerate() {
        let at = rect.min + Vec2::new(16.0, 94.0 + index as f32 * 18.0);
        let check = Rect::from_min_size(at + Vec2::new(0.0, 2.0), Vec2::splat(10.0));
        painter.rect(
            check,
            2.0,
            if task.done {
                Color32::BLACK
            } else {
                Color32::TRANSPARENT
            },
            Stroke::new(0.9_f32, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
        if task.done {
            painter.line_segment(
                [
                    check.min + Vec2::new(2.0, 5.0),
                    check.min + Vec2::new(4.0, 7.0),
                ],
                Stroke::new(1.0_f32, Color32::WHITE),
            );
            painter.line_segment(
                [
                    check.min + Vec2::new(4.0, 7.0),
                    check.min + Vec2::new(8.0, 3.0),
                ],
                Stroke::new(1.0_f32, Color32::WHITE),
            );
        }
        let mut job = egui::text::LayoutJob::simple(
            task.text.clone(),
            font.clone(),
            Color32::BLACK,
            rect.width() - 48.0,
        );
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');
        if task.done {
            for section in &mut job.sections {
                section.format.strikethrough = Stroke::new(1.0_f32, Color32::BLACK);
            }
        }
        let galley = painter.ctx().fonts_mut(|fonts| fonts.layout_job(job));
        painter.galley(at + Vec2::new(16.0, 0.0), galley, Color32::BLACK);
    }
}

impl NotesBoard {
    pub fn show(&mut self, ui: &mut egui::Ui, documents: &mut [Document]) -> BoardResponse {
        self.show_kind(ui, documents, Kind::Note)
    }

    pub fn show_kind(
        &mut self,
        ui: &mut egui::Ui,
        documents: &mut [Document],
        kind: Kind,
    ) -> BoardResponse {
        let checklist = kind == Kind::Checklist;
        let noun = if checklist { "daftar" } else { "catatan" };
        let mut result = BoardResponse::default();
        let wide = ui.available_width() >= 640.0;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if checklist {
                    "Papan checklist"
                } else {
                    "Papan catatan"
                })
                .font(FontId::new(32.0, FontFamily::Name("hand".into()))),
            );
            if wide {
                self.actions(ui, documents, kind, &mut result);
            }
        });
        if !wide {
            self.actions(ui, documents, kind, &mut result);
        }
        let mut search_changed = false;
        ui.horizontal(|ui| {
            search_changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.query)
                        .id(egui::Id::new(if checklist {
                            "checklist-search"
                        } else {
                            "notes-search"
                        }))
                        .hint_text(format!("Cari judul {noun}…"))
                        .desired_width((ui.available_width() - 44.0).clamp(120.0, 360.0)),
                )
                .changed();
            if !self.query.is_empty()
                && ui
                    .small_button("×")
                    .on_hover_text("Hapus pencarian")
                    .clicked()
            {
                self.query.clear();
                search_changed = true;
            }
        });
        ui.add_space(16.0);
        let query = self.query.trim().to_lowercase();
        let searching = !query.is_empty();
        let zoom = 0.8;
        let card = Vec2::new(260.0, 200.0) * zoom;
        let gap = 24.0;
        let columns = ((ui.available_width() - 48.0) / (card.x + gap))
            .floor()
            .max(1.0) as usize;
        let mut positions = vec![];
        let mut extent = ui.available_size().max(Vec2::new(card.x + 48.0, 280.0));
        for (index, doc) in documents
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind == kind)
            .filter(|(_, d)| {
                let title = if d.title.trim().is_empty() {
                    "tanpa judul".into()
                } else {
                    d.title.to_lowercase()
                };
                query.split_whitespace().all(|word| title.contains(word))
            })
            .enumerate()
        {
            let (document_index, doc) = doc;
            let default_position = [
                24.0 + (index % columns) as f32 * (card.x + gap),
                24.0 + (index / columns) as f32 * (card.y + gap),
            ];
            let position = if searching {
                default_position
            } else {
                doc.note_position.unwrap_or(default_position)
            };
            extent.x = extent.x.max(position[0] + card.x + 24.0);
            extent.y = extent.y.max(position[1] + card.y + 24.0);
            positions.push((document_index, position));
        }
        let no_results = searching && positions.is_empty();
        let mut scroll = egui::ScrollArea::both()
            .id_salt(if checklist {
                "checklist-board-scroll"
            } else {
                "notes-board-scroll"
            })
            .auto_shrink([false, false]);
        if search_changed {
            scroll = scroll.scroll_offset(Vec2::ZERO);
        }
        scroll.show_pannable(ui, [true, true], |ui| {
            let (board, _) = ui.allocate_exact_size(extent, Sense::hover());
            let painter = ui.painter_at(board.intersect(ui.clip_rect()));
            crate::paint_grid(&painter, board);
            if no_results {
                painter.text(
                    board.min + Vec2::splat(24.0),
                    egui::Align2::LEFT_TOP,
                    format!("Tidak ada {noun} yang cocok."),
                    FontId::proportional(14.0),
                    crate::MUTED,
                );
            }
            for (index, position) in positions {
                let doc = &mut documents[index];
                let rect =
                    Rect::from_min_size(board.min + Vec2::new(position[0], position[1]), card);
                let response = ui.interact(
                    rect,
                    egui::Id::new((
                        if checklist {
                            "checklist-card"
                        } else {
                            "note-card"
                        },
                        doc.id,
                    )),
                    Sense::click_and_drag(),
                );
                painter.rect_filled(
                    rect.translate(Vec2::new(3.0, 4.0)),
                    3.0,
                    Color32::from_black_alpha(18),
                );
                painter.rect_filled(
                    rect,
                    3.0,
                    Color32::from_rgb(doc.note_color[0], doc.note_color[1], doc.note_color[2]),
                );
                if response.hovered() || response.has_focus() {
                    painter.rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.5_f32, Color32::from_gray(80)),
                        egui::StrokeKind::Outside,
                    );
                }
                let ink = Color32::BLACK;
                let mut title = egui::text::LayoutJob::simple(
                    if doc.title.trim().is_empty() {
                        "Tanpa judul".into()
                    } else {
                        doc.title.clone()
                    },
                    FontId::new(25.0 * zoom, FontFamily::Name("hand".into())),
                    ink,
                    card.x - 32.0,
                );
                title.wrap.max_rows = 2;
                title.wrap.overflow_character = Some('…');
                let galley = ui.fonts_mut(|f| f.layout_job(title));
                painter.galley(rect.min + Vec2::new(16.0, 13.0), galley, ink);
                painter.line_segment(
                    [
                        rect.min + Vec2::new(16.0, 78.0 * zoom),
                        rect.min + Vec2::new(card.x - 16.0, 78.0 * zoom),
                    ],
                    Stroke::new(0.7_f32, ink.gamma_multiply(0.3)),
                );
                if checklist {
                    checklist_preview(&painter, rect, ui.clip_rect(), doc, zoom);
                } else {
                    let preview = if doc.body.text.trim().is_empty() {
                        String::new()
                    } else {
                        doc.body.plain_text().chars().take(180).collect::<String>()
                    };
                    let galley = painter.layout(
                        preview,
                        FontId::new(16.0 * zoom, FontFamily::Proportional),
                        ink,
                        card.x - 32.0,
                    );
                    painter
                        .with_clip_rect(
                            Rect::from_min_max(
                                rect.min + Vec2::new(16.0, 88.0 * zoom),
                                rect.max - Vec2::splat(16.0),
                            )
                            .intersect(ui.clip_rect()),
                        )
                        .galley(rect.min + Vec2::new(16.0, 88.0 * zoom), galley, ink);
                }
                if response.clicked() {
                    result.open = Some(doc.id);
                }
                if response.dragged_by(egui::PointerButton::Primary)
                    && !searching
                    && ui.input(|i| i.multi_touch().is_none())
                {
                    let delta = response.drag_delta();
                    doc.note_position = Some([
                        (position[0] + delta.x).max(0.0),
                        (position[1] + delta.y).max(0.0),
                    ]);
                    result.changed |= delta != Vec2::ZERO;
                }
                response.context_menu(|ui| {
                    if ui
                        .add(crate::controls::button(ui, &format!("Buka {noun}")))
                        .clicked()
                    {
                        result.open = Some(doc.id);
                        ui.close();
                    }
                    ui.separator();
                    ui.label(format!("Warna {noun}"));
                    result.changed |= note_palette(ui, &mut doc.note_color);
                    ui.separator();
                    if ui
                        .add(crate::controls::button(ui, &format!("Hapus {noun}…")))
                        .clicked()
                    {
                        result.delete = Some(doc.id);
                        ui.close();
                    }
                });
            }
        });
        result
    }

    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        documents: &mut [Document],
        kind: Kind,
        result: &mut BoardResponse,
    ) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(crate::controls::button(ui, "Rapikan posisi"))
                    .clicked()
                {
                    for doc in documents.iter_mut().filter(|d| d.kind == kind) {
                        doc.note_position = None;
                    }
                    result.changed = true;
                }
                if ui
                    .add(crate::controls::button(
                        ui,
                        if kind == Kind::Checklist {
                            "+ Daftar baru"
                        } else {
                            "+ Catatan baru"
                        },
                    ))
                    .clicked()
                {
                    result.create = true;
                }
            });
        });
    }
}
