use super::{geometry as geo, model::*};

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn at(a: Point, b: Point, t: f32) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

// Intersect a stroke segment with the swept brush: two circles and their joining strip.
fn intervals(a: Point, b: Point, from: Point, to: Point, radius: f32) -> Vec<(f32, f32)> {
    let delta = geo::sub(b, a);
    let length2 = dot(delta, delta);
    if length2 < 1e-10 {
        return if geo::segment_distance(a, from, to) < radius {
            vec![(0.0, 1.0)]
        } else {
            vec![]
        };
    }
    let mut ranges = vec![];
    for center in [from, to] {
        let offset = geo::sub(a, center);
        let projection = dot(offset, delta);
        let discriminant =
            projection * projection - length2 * (dot(offset, offset) - radius * radius);
        if discriminant > 0.0 {
            let root = discriminant.sqrt();
            ranges.push((
                (-projection - root) / length2,
                (-projection + root) / length2,
            ));
        }
    }
    let sweep = geo::sub(to, from);
    let length = dot(sweep, sweep).sqrt();
    if length > 1e-5 {
        let axis = [sweep[0] / length, sweep[1] / length];
        let normal = [-axis[1], axis[0]];
        let offset = geo::sub(a, from);
        let mut range = (0.0_f32, 1.0_f32);
        for (basis, min, max) in [(axis, 0.0, length), (normal, -radius, radius)] {
            let origin = dot(offset, basis);
            let velocity = dot(delta, basis);
            if velocity.abs() < 1e-7 {
                if origin < min || origin > max {
                    range = (1.0, 0.0);
                    break;
                }
            } else {
                let t0 = (min - origin) / velocity;
                let t1 = (max - origin) / velocity;
                range.0 = range.0.max(t0.min(t1));
                range.1 = range.1.min(t0.max(t1));
            }
        }
        ranges.push(range);
    }
    for range in &mut ranges {
        range.0 = range.0.max(0.0);
        range.1 = range.1.min(1.0);
    }
    ranges.retain(|(start, end)| end - start > 1e-7);
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32)> = vec![];
    for range in ranges {
        if let Some(last) = merged.last_mut().filter(|last| range.0 <= last.1 + 1e-7) {
            last.1 = last.1.max(range.1);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn finish(current: &mut Vec<Point>, pieces: &mut Vec<Vec<Point>>) {
    if current.windows(2).any(|p| geo::distance(p[0], p[1]) > 1e-4) {
        pieces.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn trim_path(points: &[Point], from: Point, to: Point, radius: f32) -> Option<Vec<Vec<Point>>> {
    let mut pieces = vec![];
    let mut current = vec![];
    let mut changed = false;
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        let removed = intervals(a, b, from, to, radius);
        changed |= !removed.is_empty();
        let mut start = 0.0;
        for (left, right) in removed.into_iter().chain(std::iter::once((1.0, 1.0))) {
            if left > start {
                let first = at(a, b, start);
                if current
                    .last()
                    .is_some_and(|last| geo::distance(*last, first) > 1e-4)
                {
                    finish(&mut current, &mut pieces);
                }
                if current.is_empty() {
                    current.push(first);
                }
                current.push(at(a, b, left));
            }
            if right > left {
                finish(&mut current, &mut pieces);
            }
            start = right;
        }
    }
    finish(&mut current, &mut pieces);
    changed.then_some(pieces)
}

pub fn erase(data: &mut CanvasData, from: Point, to: Point, radius: f32) -> bool {
    let mut removed = vec![];
    let mut trimmed = vec![];
    for element in data.elements.iter().filter(|e| !e.locked) {
        if matches!(element.kind, ElementKind::Pen | ElementKind::Marker) {
            if let Some(pieces) = trim_path(
                &element.points,
                from,
                to,
                radius + element.style.width * 0.5,
            ) {
                if pieces.is_empty() {
                    removed.push(element.id);
                } else {
                    trimmed.push((element.id, pieces));
                }
            }
            continue;
        }
        let path = if element.kind == ElementKind::Arrow {
            let mut path = geo::arrow_path(data, element);
            path.extend(geo::arrowhead(&path, element.style.width));
            path
        } else if element.kind.has_path() {
            element.points.clone()
        } else {
            geo::outline(element)
        };
        let hit = (!element.kind.has_path()
            && (geo::in_polygon(from, &path) || geo::in_polygon(to, &path)))
            || path.windows(2).any(|edge| {
                !intervals(
                    edge[0],
                    edge[1],
                    from,
                    to,
                    radius + element.style.width * 0.5,
                )
                .is_empty()
            });
        if hit {
            removed.push(element.id);
        }
    }
    if removed.is_empty() && trimmed.is_empty() {
        return false;
    }
    data.remove(&removed);
    for (id, pieces) in trimmed {
        let index = data.elements.iter().position(|e| e.id == id).unwrap();
        let original = data.elements[index].clone();
        let replacements: Vec<_> = pieces
            .into_iter()
            .enumerate()
            .map(|(i, points)| {
                let mut piece = original.clone();
                if i > 0 {
                    piece.id = data.id();
                }
                piece.pos = points[0];
                piece.points = points;
                piece
            })
            .collect();
        data.elements.splice(index..=index, replacements);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_splits_sparse_strokes_and_keeps_unaffected_geometry() {
        for kind in [ElementKind::Pen, ElementKind::Marker] {
            let mut data = CanvasData::default();
            let mut pen = Element::new(data.id(), kind, [0.0, 0.0], Style::default());
            pen.points = vec![[0.0, 0.0], [100.0, 0.0]];
            data.elements.push(pen.clone());
            let before = data.clone();
            assert!(!erase(&mut data, [50.0, 30.0], [80.0, 30.0], 10.0));
            assert_eq!(data, before);
            assert!(erase(&mut data, [50.0, -50.0], [50.0, 50.0], 10.0));
            assert_eq!(data.elements.len(), 2);
            assert_eq!(data.elements[0].points, vec![[0.0, 0.0], [39.0, 0.0]]);
            assert_eq!(data.elements[1].points, vec![[61.0, 0.0], [100.0, 0.0]]);
            assert_eq!(data.elements[0].id, pen.id);
            assert_ne!(data.elements[0].id, data.elements[1].id);
            assert_eq!(data.elements[1].style, pen.style);
            assert!(!erase(&mut data, [50.0, 0.0], [50.0, 0.0], 10.0));
            assert!(data.validate().is_ok());
            let roundtrip: CanvasData =
                serde_json::from_str(&serde_json::to_string(&data).unwrap()).unwrap();
            assert_eq!(roundtrip, data);
        }
    }

    #[test]
    fn circle_removes_only_middle_and_can_erase_entire_remaining_stroke() {
        let pieces =
            trim_path(&[[0.0, 0.0], [100.0, 0.0]], [50.0, 0.0], [50.0, 0.0], 10.0).unwrap();
        assert_eq!(pieces.len(), 2);
        assert!(geo::distance(pieces[0][1], [40.0, 0.0]) < 0.001);
        assert!(geo::distance(pieces[1][0], [60.0, 0.0]) < 0.001);
        assert!(
            trim_path(&pieces[0], [0.0, 0.0], [50.0, 0.0], 5.0)
                .unwrap()
                .is_empty()
        );
        let bend = trim_path(
            &[[0.0, 0.0], [50.0, 0.0], [50.0, 50.0]],
            [25.0, 0.0],
            [25.0, 0.0],
            5.0,
        )
        .unwrap();
        assert!(geo::distance(bend[1][0], [30.0, 0.0]) < 0.001);
        assert_eq!(&bend[1][1..], &[[50.0, 0.0], [50.0, 50.0]]);
    }

    #[test]
    fn whole_objects_erase_on_contact_and_detach_arrows_but_locked_objects_survive() {
        let mut data = CanvasData::default();
        let mut shape = Element::new(
            data.id(),
            ElementKind::Diamond,
            [0.0, 0.0],
            Style::default(),
        );
        shape.style.rough = false;
        shape.text = "Satu paket".into();
        let mut locked = shape.clone();
        locked.id = data.id();
        locked.locked = true;
        let mut arrow = Element::new(data.id(), ElementKind::Arrow, [0.0, 0.0], Style::default());
        arrow.points = vec![[160.0, 50.0], [300.0, 50.0]];
        arrow.start_binding = Some(Binding {
            target: shape.id,
            anchor: None,
        });
        data.elements = vec![shape.clone(), locked.clone(), arrow];
        assert!(!erase(&mut data, [0.0, 0.0], [0.0, 0.0], 2.0));
        assert!(erase(&mut data, [80.0, -50.0], [80.0, 150.0], 3.0));
        assert!(data.element(shape.id).is_none());
        assert_eq!(data.element(locked.id), Some(&locked));
        assert!(data.elements.last().unwrap().start_binding.is_none());
    }
}
