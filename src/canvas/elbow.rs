use super::{geometry as geo, model::*};
use egui::Rect;

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn axis(direction: Point) -> Point {
    if direction[0].abs() >= direction[1].abs() {
        [if direction[0] < 0.0 { -1.0 } else { 1.0 }, 0.0]
    } else {
        [0.0, if direction[1] < 0.0 { -1.0 } else { 1.0 }]
    }
}

fn rect(shape: &Element) -> Rect {
    Rect::from_min_size(shape.pos.into(), shape.size.into())
}

fn normal(shape: Option<&Element>, point: Point, other: Point) -> Point {
    let Some(shape) = shape else {
        return axis(geo::sub(other, point));
    };
    let delta = geo::sub(point, shape.center());
    match shape.kind {
        ElementKind::Ellipse => axis([
            delta[0] / shape.size[0].powi(2),
            delta[1] / shape.size[1].powi(2),
        ]),
        ElementKind::Diamond => axis([
            if delta[0].abs() < 0.001 {
                0.0
            } else {
                delta[0].signum() / shape.size[0]
            },
            if delta[1].abs() < 0.001 {
                0.0
            } else {
                delta[1].signum() / shape.size[1]
            },
        ]),
        _ => {
            let bounds = rect(shape);
            [
                (point[0] - bounds.left(), [-1.0, 0.0]),
                (bounds.right() - point[0], [1.0, 0.0]),
                (point[1] - bounds.top(), [0.0, -1.0]),
                (bounds.bottom() - point[1], [0.0, 1.0]),
            ]
            .into_iter()
            .min_by(|a, b| {
                a.0.abs().total_cmp(&b.0.abs()).then_with(|| {
                    dot(b.1, geo::sub(other, point)).total_cmp(&dot(a.1, geo::sub(other, point)))
                })
            })
            .unwrap()
            .1
        }
    }
}

fn stub(
    shape: Option<&Element>,
    other: Option<&Element>,
    point: Point,
    normal: Point,
    clearance: f32,
) -> Point {
    let mut length = clearance;
    if let Some(shape) = shape {
        let bounds = rect(shape);
        length += if normal[0] > 0.0 {
            bounds.right() - point[0]
        } else if normal[0] < 0.0 {
            point[0] - bounds.left()
        } else if normal[1] > 0.0 {
            bounds.bottom() - point[1]
        } else {
            point[1] - bounds.top()
        };
    }
    if let Some(other) = other {
        let bounds = rect(other);
        let gap = if normal[0] != 0.0 && point[1] >= bounds.top() && point[1] <= bounds.bottom() {
            Some(if normal[0] > 0.0 {
                bounds.left() - point[0]
            } else {
                point[0] - bounds.right()
            })
        } else if normal[1] != 0.0 && point[0] >= bounds.left() && point[0] <= bounds.right() {
            Some(if normal[1] > 0.0 {
                bounds.top() - point[1]
            } else {
                point[1] - bounds.bottom()
            })
        } else {
            None
        };
        if let Some(gap) = gap.filter(|gap| *gap > 0.0) {
            length = length.min(gap * 0.4);
        }
    }
    geo::add(point, [normal[0] * length, normal[1] * length])
}

fn simplify(points: impl IntoIterator<Item = Point>) -> Vec<Point> {
    let mut path: Vec<Point> = vec![];
    for point in points {
        if path
            .last()
            .is_some_and(|last| geo::distance(*last, point) < 0.001)
        {
            continue;
        }
        while path.len() >= 2 {
            let a = path[path.len() - 2];
            let b = path[path.len() - 1];
            let ab = geo::sub(b, a);
            let bc = geo::sub(point, b);
            if (ab[0] * bc[1] - ab[1] * bc[0]).abs() < 0.001 && dot(ab, bc) > 0.0 {
                path.pop();
            } else {
                break;
            }
        }
        path.push(point);
    }
    path
}

fn crosses(a: Point, b: Point, bounds: Rect) -> bool {
    let bounds = bounds.shrink(0.01);
    if (a[0] - b[0]).abs() < 0.001 {
        a[0] > bounds.left()
            && a[0] < bounds.right()
            && a[1].min(b[1]) < bounds.bottom()
            && a[1].max(b[1]) > bounds.top()
    } else {
        a[1] > bounds.top()
            && a[1] < bounds.bottom()
            && a[0].min(b[0]) < bounds.right()
            && a[0].max(b[0]) > bounds.left()
    }
}

pub(super) fn path(canvas: &CanvasData, arrow: &Element, a: Point, b: Point) -> Vec<Point> {
    let source = arrow
        .start_binding
        .as_ref()
        .and_then(|binding| canvas.element(binding.target));
    let target = arrow
        .end_binding
        .as_ref()
        .and_then(|binding| canvas.element(binding.target));
    let na = normal(source, a, b);
    let nb = normal(target, b, a);
    let clearance = 20.0 + arrow.style.width * 2.0;
    let s = stub(source, target, a, na, clearance);
    let t = stub(target, source, b, nb, clearance);
    let obstacles: Vec<_> = [source, target].into_iter().flatten().map(rect).collect();
    let middle = geo::midpoint(s, t);
    let mut xs = vec![middle[0]];
    let mut ys = vec![middle[1]];
    if let Some(bend) = arrow.bend {
        xs.insert(0, bend[0]);
        ys.insert(0, bend[1]);
    }
    for bounds in &obstacles {
        xs.extend([bounds.left() - clearance, bounds.right() + clearance]);
        ys.extend([bounds.top() - clearance, bounds.bottom() + clearance]);
    }
    let mut candidates = vec![vec![s, [t[0], s[1]], t], vec![s, [s[0], t[1]], t]];
    for x in &xs {
        candidates.push(vec![s, [*x, s[1]], [*x, t[1]], t]);
    }
    for y in &ys {
        candidates.push(vec![s, [s[0], *y], [t[0], *y], t]);
    }
    for x in &xs {
        for y in &ys {
            candidates.push(vec![s, [*x, s[1]], [*x, *y], [t[0], *y], t]);
            candidates.push(vec![s, [s[0], *y], [*x, *y], [*x, t[1]], t]);
        }
    }
    let mut best = None;
    let mut best_score = f32::INFINITY;
    for candidate in candidates {
        if candidate.windows(2).any(|edge| {
            obstacles
                .iter()
                .any(|bounds| crosses(edge[0], edge[1], *bounds))
        }) {
            continue;
        }
        let path = simplify(
            std::iter::once(a)
                .chain(candidate)
                .chain(std::iter::once(b)),
        );
        if path.len() < 2
            || dot(geo::sub(path[1], a), na) <= 0.0
            || dot(geo::sub(path[path.len() - 2], b), nb) <= 0.0
        {
            continue;
        }
        if path
            .windows(3)
            .any(|p| dot(geo::sub(p[1], p[0]), geo::sub(p[2], p[1])) < -0.001)
        {
            continue;
        }
        let length: f32 = path.windows(2).map(|p| geo::distance(p[0], p[1])).sum();
        let deviation = arrow.bend.map_or(0.0, |bend| {
            path.windows(2)
                .map(|p| geo::segment_distance(bend, p[0], p[1]))
                .fold(f32::INFINITY, f32::min)
        });
        let balance = (geo::distance(path[0], path[1])
            - geo::distance(path[path.len() - 2], *path.last().unwrap()))
        .abs();
        let score = length + (path.len() - 2) as f32 * 20.0 + balance * 0.05 + deviation * 4.0;
        if score < best_score {
            best_score = score;
            best = Some(path);
        }
    }
    best.unwrap_or_else(|| simplify([a, s, [s[0], t[1]], t, b]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagram() -> CanvasData {
        let mut data = CanvasData::default();
        for pos in [[20.0, 20.0], [210.0, 270.0]] {
            let mut shape = Element::new(data.id(), ElementKind::Rectangle, pos, Style::default());
            shape.size = [120.0, 80.0];
            shape.style.rough = false;
            data.elements.push(shape);
        }
        let mut arrow = Element::new(
            data.id(),
            ElementKind::Arrow,
            [240.0, 270.0],
            Style::default(),
        );
        arrow.points = vec![[240.0, 270.0], [110.0, 100.0]];
        arrow.route = ArrowRoute::Elbow;
        arrow.start_binding = Some(Binding {
            target: 2,
            anchor: Some([0.25, 0.0]),
        });
        arrow.end_binding = Some(Binding {
            target: 1,
            anchor: Some([0.75, 1.0]),
        });
        data.elements.push(arrow);
        data
    }

    fn check(data: &CanvasData) {
        let arrow = data.element(3).unwrap();
        let path = geo::arrow_path(data, arrow);
        let (a, b) = geo::endpoints(data, arrow);
        assert_eq!(path.first(), Some(&a));
        assert_eq!(path.last(), Some(&b));
        let na = normal(data.element(2), a, b);
        let nb = normal(data.element(1), b, a);
        let first = geo::sub(path[1], a);
        let last = geo::sub(path[path.len() - 2], b);
        assert!(
            dot(first, na) > 0.0 && (first[0] * na[1] - first[1] * na[0]).abs() < 0.01,
            "Wrong exit: {path:?}"
        );
        assert!(
            dot(last, nb) > 0.0 && (last[0] * nb[1] - last[1] * nb[0]).abs() < 0.01,
            "Wrong entry: {path:?}"
        );
        for edge in path.windows(2) {
            let d = geo::sub(edge[1], edge[0]);
            assert!(d[0].abs() < 0.001 || d[1].abs() < 0.001);
            assert!(geo::distance(edge[0], edge[1]) > 0.001);
            for shape in &data.elements[..2] {
                let mut shape = shape.clone();
                shape.style.rough = false;
                let outline = geo::outline(&shape);
                for step in 1..100 {
                    let p = geo::add(
                        edge[0],
                        [d[0] * step as f32 / 100.0, d[1] * step as f32 / 100.0],
                    );
                    let depth = outline
                        .windows(2)
                        .map(|e| geo::segment_distance(p, e[0], e[1]))
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        !geo::in_polygon(p, &outline) || depth < 0.02,
                        "Path enters shape {}: {path:?} at {p:?}",
                        shape.id
                    );
                }
            }
        }
        for p in path.windows(3) {
            assert!(
                dot(geo::sub(p[1], p[0]), geo::sub(p[2], p[1])) >= -0.001,
                "Path reverses: {path:?}"
            );
        }
        let handle = geo::bend_handle(data, arrow);
        assert!(
            path.windows(2)
                .any(|p| geo::segment_distance(handle, p[0], p[1]) < 0.001)
        );
    }

    #[test]
    fn bottom_and_top_ports_use_vertical_legs_and_a_balanced_middle_bend() {
        let data = diagram();
        check(&data);
        let path = geo::arrow_path(&data, data.element(3).unwrap());
        assert_eq!(
            path,
            vec![
                [240.0, 270.0],
                [240.0, 185.0],
                [110.0, 185.0],
                [110.0, 100.0]
            ]
        );
        let mut custom = data.clone();
        custom.element_mut(3).unwrap().bend = Some([175.0, 160.0]);
        check(&custom);
        assert!(
            geo::arrow_path(&custom, custom.element(3).unwrap())
                .iter()
                .any(|p| p[1] == 160.0)
        );
        let restored: CanvasData =
            serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(
            geo::arrow_path(&custom, custom.element(3).unwrap()),
            geo::arrow_path(&restored, restored.element(3).unwrap())
        );
        if let Some(folder) = std::env::var_os("NOTVAS_QA_DIR") {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(
                folder.join("elbow-routing.png"),
                data.png("Panah siku").unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn routing_respects_all_port_sides_and_detours_around_connected_shapes() {
        let ports = [[1.0, 0.5], [0.0, 0.5], [0.5, 0.0], [0.5, 1.0]];
        for kind in [
            ElementKind::Rectangle,
            ElementKind::Ellipse,
            ElementKind::Diamond,
        ] {
            for source_port in ports {
                for target_port in ports {
                    for pos in [[210.0, 270.0], [240.0, 20.0], [20.0, 250.0], [-220.0, 20.0]] {
                        let mut data = diagram();
                        data.element_mut(1).unwrap().kind = kind;
                        data.element_mut(2).unwrap().kind = kind;
                        data.element_mut(2).unwrap().pos = pos;
                        let arrow = data.element_mut(3).unwrap();
                        arrow.start_binding.as_mut().unwrap().anchor = Some(source_port);
                        arrow.end_binding.as_mut().unwrap().anchor = Some(target_port);
                        check(&data);
                    }
                }
            }
        }
        let mut narrow = diagram();
        narrow.element_mut(2).unwrap().pos = [20.0, 108.0];
        check(&narrow);
    }
}
