use super::*;
use egui::{
    ColorImage, Pos2, Rect, TextureId,
    epaint::{ImageData, Primitive, Vertex},
};
use std::collections::HashMap;

fn sample(texture: &ColorImage, uv: egui::Vec2) -> [f32; 4] {
    let x = (uv.x * texture.size[0] as f32 - 0.5).clamp(0.0, texture.size[0] as f32 - 1.0);
    let y = (uv.y * texture.size[1] as f32 - 0.5).clamp(0.0, texture.size[1] as f32 - 1.0);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(texture.size[0] - 1);
    let y1 = (y0 + 1).min(texture.size[1] - 1);
    let mut result = [0.0; 4];
    for (px, py, weight) in [
        (x0, y0, (1.0 - x.fract()) * (1.0 - y.fract())),
        (x1, y0, x.fract() * (1.0 - y.fract())),
        (x0, y1, (1.0 - x.fract()) * y.fract()),
        (x1, y1, x.fract() * y.fract()),
    ] {
        for (out, value) in result
            .iter_mut()
            .zip(texture.pixels[py * texture.size[0] + px].to_array())
        {
            *out += value as f32 / 255.0 * weight;
        }
    }
    result
}

fn edge(a: Pos2, b: Pos2, p: Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

fn triangle(image: &mut image::RgbaImage, clip: Rect, texture: &ColorImage, mut v: [Vertex; 3]) {
    let mut area = edge(v[0].pos, v[1].pos, v[2].pos);
    if area.abs() < 0.0001 {
        return;
    }
    if area < 0.0 {
        v.swap(1, 2);
        area = -area;
    }
    let bounds = Rect::from_points(&[v[0].pos, v[1].pos, v[2].pos])
        .intersect(clip)
        .intersect(Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(image.width() as f32, image.height() as f32),
        ));
    if !bounds.is_positive() {
        return;
    }
    let inclusive = |a: Pos2, b: Pos2| b.y < a.y || (b.y == a.y && b.x > a.x);
    let top_left = [
        inclusive(v[1].pos, v[2].pos),
        inclusive(v[2].pos, v[0].pos),
        inclusive(v[0].pos, v[1].pos),
    ];
    let colors = v.map(|vertex| vertex.color.to_array());
    for y in bounds.top().floor() as u32..bounds.bottom().ceil() as u32 {
        for x in bounds.left().floor() as u32..bounds.right().ceil() as u32 {
            let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
            if !clip.contains(p) {
                continue;
            }
            let edges = [
                edge(v[1].pos, v[2].pos, p),
                edge(v[2].pos, v[0].pos, p),
                edge(v[0].pos, v[1].pos, p),
            ];
            if edges
                .iter()
                .zip(top_left)
                .any(|(e, inclusive)| *e < 0.0 || (*e == 0.0 && !inclusive))
            {
                continue;
            }
            let w = edges.map(|e| e / area);
            let uv = v[0].uv.to_vec2() * w[0] + v[1].uv.to_vec2() * w[1] + v[2].uv.to_vec2() * w[2];
            let texel = sample(texture, uv);
            let mut source = [0.0; 4];
            for channel in 0..4 {
                source[channel] = texel[channel]
                    * (colors[0][channel] as f32 * w[0]
                        + colors[1][channel] as f32 * w[1]
                        + colors[2][channel] as f32 * w[2])
                    / 255.0;
            }
            let destination = image.get_pixel_mut(x, y);
            for channel in 0..4 {
                destination.0[channel] = ((source[channel]
                    + destination.0[channel] as f32 / 255.0 * (1.0 - source[3]))
                    * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[test]
#[ignore = "Renders documentation images from Notvas UI with an isolated demo workspace"]
fn render_documentation_screenshots() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let demo = root.join("test-data/docs-demo");
    let destination = root.join("docs/screenshots");
    std::fs::create_dir_all(&destination).unwrap();
    for (name, kind, overview) in [
        ("canvas", Kind::Canvas, false),
        ("notes", Kind::Note, true),
        ("note-editor", Kind::Note, false),
        ("checklist", Kind::Checklist, true),
        ("checklist-editor", Kind::Checklist, false),
    ] {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = ui_tests::app();
        app.workspace = storage::load(&demo.join("canvas/workspace.notvas")).unwrap();
        app.kind = kind;
        app.select(
            app.workspace
                .documents
                .iter()
                .find(|doc| doc.kind == kind)
                .map(|doc| doc.id),
        );
        app.notes_board.overview = overview;
        app.checklist_board.overview = overview;
        app.load_failed = false;
        app.status = "Data demo - tersimpan lokal".into();
        let mut textures: HashMap<TextureId, ColorImage> = HashMap::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 820.0))),
                    ..Default::default()
                },
                |ctx| app.draw(ctx),
            );
            for (id, delta) in &output.textures_delta.set {
                let ImageData::Color(pixels) = &delta.image;
                if let Some([x, y]) = delta.pos {
                    let target = textures.get_mut(id).unwrap();
                    for row in 0..pixels.size[1] {
                        let offset = (row + y) * target.size[0] + x;
                        target.pixels[offset..offset + pixels.size[0]].copy_from_slice(
                            &pixels.pixels[row * pixels.size[0]..(row + 1) * pixels.size[0]],
                        );
                    }
                } else {
                    textures.insert(*id, (**pixels).clone());
                }
            }
            if frame == 3 {
                let mut image =
                    image::RgbaImage::from_pixel(1280, 820, image::Rgba([247, 247, 247, 255]));
                for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
                    let Primitive::Mesh(mesh) = primitive.primitive else {
                        panic!("Unsupported paint callback");
                    };
                    let texture = textures.get(&mesh.texture_id).unwrap();
                    for indices in mesh.indices.chunks_exact(3) {
                        triangle(
                            &mut image,
                            primitive.clip_rect,
                            texture,
                            [
                                mesh.vertices[indices[0] as usize],
                                mesh.vertices[indices[1] as usize],
                                mesh.vertices[indices[2] as usize],
                            ],
                        );
                    }
                }
                let file = destination.join(format!("{name}.png"));
                image.save(&file).unwrap();
                println!("{}", file.display());
            }
            for id in output.textures_delta.free {
                textures.remove(&id);
            }
        }
    }
}
