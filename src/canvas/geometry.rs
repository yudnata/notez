use super::model::*;

pub fn distance(a: Point, b: Point) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
pub fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
pub fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn midpoint(a: Point, b: Point) -> Point {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}

pub fn boundary(element: &Element, towards: Point) -> Point {
    let center = element.center();
    let d = sub(towards, center);
    let rx = element.size[0].max(1.0) * 0.5;
    let ry = element.size[1].max(1.0) * 0.5;
    if d[0].abs() + d[1].abs() < 0.001 {
        return [center[0] + rx, center[1]];
    }
    let scale = match element.kind {
        ElementKind::Ellipse => 1.0 / ((d[0] / rx).powi(2) + (d[1] / ry).powi(2)).sqrt(),
        ElementKind::Diamond => 1.0 / (d[0].abs() / rx + d[1].abs() / ry),
        _ => (rx / d[0].abs().max(0.0001)).min(ry / d[1].abs().max(0.0001)),
    };
    [center[0] + d[0] * scale, center[1] + d[1] * scale]
}

pub fn endpoints(canvas: &CanvasData, element: &Element) -> (Point, Point) {
    let raw_start = element.points.first().copied().unwrap_or(element.pos);
    let raw_end = element
        .points
        .last()
        .copied()
        .unwrap_or(add(element.pos, element.size));
    let a = element
        .start_binding
        .as_ref()
        .and_then(|b| canvas.element(b.target));
    let b = element
        .end_binding
        .as_ref()
        .and_then(|b| canvas.element(b.target));
    let control = element
        .bend
        .filter(|_| element.route != ArrowRoute::Elbow)
        .or_else(|| {
            (element.route == ArrowRoute::Curve).then(|| default_curve_control(canvas, element))
        });
    let towards_a = control.unwrap_or_else(|| b.map_or(raw_end, Element::center));
    let towards_b = control.unwrap_or_else(|| a.map_or(raw_start, Element::center));
    (
        a.map_or(raw_start, |target| {
            bound_point(target, element.start_binding.as_ref().unwrap(), towards_a)
        }),
        b.map_or(raw_end, |target| {
            bound_point(target, element.end_binding.as_ref().unwrap(), towards_b)
        }),
    )
}

fn bound_point(target: &Element, binding: &Binding, towards: Point) -> Point {
    binding.anchor.map_or_else(
        || boundary(target, towards),
        |anchor| {
            [
                target.pos[0] + target.size[0] * anchor[0],
                target.pos[1] + target.size[1] * anchor[1],
            ]
        },
    )
}

pub fn arrow_path(canvas: &CanvasData, element: &Element) -> Vec<Point> {
    let (a, b) = endpoints(canvas, element);
    match element.route {
        ArrowRoute::Straight => vec![a, b],
        ArrowRoute::Elbow => super::elbow::path(canvas, element, a, b),
        ArrowRoute::Curve => {
            let control = bend_handle(canvas, element);
            (0..=32)
                .map(|i| {
                    let t = i as f32 / 32.0;
                    let s = 1.0 - t;
                    [
                        s * s * a[0] + 2.0 * s * t * control[0] + t * t * b[0],
                        s * s * a[1] + 2.0 * s * t * control[1] + t * t * b[1],
                    ]
                })
                .collect()
        }
    }
}

pub fn bend_handle(canvas: &CanvasData, element: &Element) -> Point {
    if element.route == ArrowRoute::Elbow {
        let path = arrow_path(canvas, element);
        let segments = if path.len() > 3 {
            &path[1..path.len() - 1]
        } else {
            &path[..]
        };
        return segments
            .windows(2)
            .max_by(|a, b| distance(a[0], a[1]).total_cmp(&distance(b[0], b[1])))
            .map_or_else(
                || midpoint(path[0], *path.last().unwrap()),
                |edge| midpoint(edge[0], edge[1]),
            );
    }
    if let Some(bend) = element.bend {
        return bend;
    }
    if element.route == ArrowRoute::Curve {
        default_curve_control(canvas, element)
    } else {
        let (a, b) = endpoints(canvas, element);
        midpoint(a, b)
    }
}

fn default_curve_control(canvas: &CanvasData, element: &Element) -> Point {
    let mut straight = element.clone();
    straight.route = ArrowRoute::Straight;
    straight.bend = None;
    let (a, b) = endpoints(canvas, &straight);
    let middle = midpoint(a, b);
    let d = sub(b, a);
    let length = distance(a, b).max(0.001);
    let offset = (length * 0.25).min(65.0);
    [
        middle[0] + d[1] / length * offset,
        middle[1] - d[0] / length * offset,
    ]
}

pub fn bounds(canvas: &CanvasData, element: &Element) -> egui::Rect {
    if element.kind.has_path() {
        let points = if element.kind == ElementKind::Arrow {
            arrow_path(canvas, element)
        } else {
            element.points.clone()
        };
        egui::Rect::from_points(
            &points
                .iter()
                .map(|p| egui::pos2(p[0], p[1]))
                .collect::<Vec<_>>(),
        )
    } else {
        egui::Rect::from_min_size(element.pos.into(), element.size.into())
    }
}

pub fn segment_distance(point: Point, a: Point, b: Point) -> f32 {
    distance(point, closest_on_segment(point, a, b))
}

fn closest_on_segment(point: Point, a: Point, b: Point) -> Point {
    let ab = sub(b, a);
    let ap = sub(point, a);
    let len = ab[0] * ab[0] + ab[1] * ab[1];
    if len < 0.0001 {
        return a;
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / len).clamp(0.0, 1.0);
    [a[0] + ab[0] * t, a[1] + ab[1] * t]
}

pub fn hit(canvas: &CanvasData, point: Point, tolerance: f32) -> Option<u64> {
    canvas
        .elements
        .iter()
        .rev()
        .find(|element| {
            if element.kind.has_path() {
                let points = if element.kind == ElementKind::Arrow {
                    arrow_path(canvas, element)
                } else {
                    element.points.clone()
                };
                points.windows(2).any(|p| {
                    segment_distance(point, p[0], p[1]) <= tolerance + element.style.width * 0.5
                })
            } else {
                bounds(canvas, element)
                    .expand(tolerance * 0.25)
                    .contains(point.into())
            }
        })
        .map(|e| e.id)
}

pub fn suggest_binding(
    canvas: &CanvasData,
    point: Point,
    exclude: Option<u64>,
    tolerance: f32,
) -> Option<(Binding, Point)> {
    canvas
        .elements
        .iter()
        .rev()
        .filter(|e| e.kind.bindable() && Some(e.id) != exclude)
        .filter(|e| bounds(canvas, e).expand(tolerance).contains(point.into()))
        .find_map(|e| {
            let mut geometry = e.clone();
            geometry.style.rough = false;
            let outline = outline(&geometry);
            let nearest = outline
                .windows(2)
                .map(|edge| closest_on_segment(point, edge[0], edge[1]))
                .min_by(|a, b| distance(point, *a).total_cmp(&distance(point, *b)))?;
            let nearest = if e.kind == ElementKind::Ellipse {
                boundary(e, nearest)
            } else {
                nearest
            };
            if !in_polygon(point, &outline) && distance(point, nearest) > tolerance {
                return None;
            }
            let anchor = [
                ((nearest[0] - e.pos[0]) / e.size[0]).clamp(0.0, 1.0),
                ((nearest[1] - e.pos[1]) / e.size[1]).clamp(0.0, 1.0),
            ];
            Some((
                Binding {
                    target: e.id,
                    anchor: Some(anchor),
                },
                nearest,
            ))
        })
}

pub fn outline(element: &Element) -> Vec<Point> {
    let (x, y) = (element.pos[0], element.pos[1]);
    let (w, h) = (element.size[0], element.size[1]);
    let mut points = if element.kind == ElementKind::Ellipse {
        (0..64)
            .map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / 64.0;
                [
                    x + w * (0.5 + angle.cos() * 0.5),
                    y + h * (0.5 + angle.sin() * 0.5),
                ]
            })
            .collect::<Vec<_>>()
    } else {
        let corners = if element.kind == ElementKind::Diamond {
            [
                [x + w * 0.5, y],
                [x + w, y + h * 0.5],
                [x + w * 0.5, y + h],
                [x, y + h * 0.5],
            ]
        } else {
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
        };
        (0..32)
            .map(|i| {
                let a = corners[i / 8];
                let b = corners[(i / 8 + 1) % 4];
                let t = (i % 8) as f32 / 8.0;
                [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
            })
            .collect()
    };
    if element.style.rough {
        for (i, point) in points.iter_mut().enumerate() {
            let seed = (element.id.wrapping_mul(31) as f32 + i as f32 * 1.71).sin();
            point[0] += seed * 0.65;
            point[1] += (seed * 3.1).sin() * 0.65;
        }
    }
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    points
}

pub fn arrowhead(path: &[Point], width: f32) -> Vec<Point> {
    let Some(&tip) = path.last() else {
        return vec![];
    };
    let previous = path
        .iter()
        .rev()
        .skip(1)
        .find(|&&p| distance(p, tip) > 0.1)
        .copied()
        .unwrap_or([tip[0] - 1.0, tip[1]]);
    let delta = sub(tip, previous);
    let len = distance(tip, previous).max(0.1);
    let unit = [delta[0] / len, delta[1] / len];
    let size = 10.0 + width * 1.5;
    vec![
        [
            tip[0] - unit[0] * size - unit[1] * size * 0.45,
            tip[1] - unit[1] * size + unit[0] * size * 0.45,
        ],
        tip,
        [
            tip[0] - unit[0] * size + unit[1] * size * 0.45,
            tip[1] - unit[1] * size - unit[0] * size * 0.45,
        ],
    ]
}

pub fn in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current[1] > point[1]) != (previous[1] > point[1]) {
            let x = (previous[0] - current[0]) * (point[1] - current[1])
                / (previous[1] - current[1])
                + current[0];
            if point[0] < x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagram() -> CanvasData {
        let mut canvas = CanvasData::default();
        let mut a = Element::new(
            canvas.id(),
            ElementKind::Rectangle,
            [0.0, 0.0],
            Style::default(),
        );
        a.size = [100.0, 80.0];
        let mut b = Element::new(
            canvas.id(),
            ElementKind::Ellipse,
            [300.0, 0.0],
            Style::default(),
        );
        b.size = [100.0, 80.0];
        let mut arrow = Element::new(
            canvas.id(),
            ElementKind::Arrow,
            [0.0, 0.0],
            Style::default(),
        );
        arrow.points = vec![a.center(), b.center()];
        arrow.start_binding = Some(Binding {
            target: a.id,
            anchor: None,
        });
        arrow.end_binding = Some(Binding {
            target: b.id,
            anchor: None,
        });
        canvas.elements = vec![a, b, arrow];
        canvas
    }

    #[test]
    fn arrows_attach_to_boundaries_follow_movement_and_detach_safely() {
        let mut canvas = diagram();
        assert_eq!(
            endpoints(&canvas, &canvas.elements[2]),
            ([100.0, 40.0], [300.0, 40.0])
        );
        canvas.elements[1].translate([0.0, 200.0]);
        let moved = endpoints(&canvas, &canvas.elements[2]);
        assert!(moved.1[1] > 200.0);
        canvas.remove(&[2]);
        let arrow = canvas.element(3).unwrap();
        assert!(arrow.end_binding.is_none());
        assert_eq!(endpoints(&canvas, arrow).1, moved.1);
    }

    #[test]
    fn duplicate_group_rebinds_its_own_arrows() {
        let mut canvas = diagram();
        let ids = canvas.duplicate(&[1, 2, 3], [40.0, 40.0]);
        let arrow = canvas.element(ids[2]).unwrap();
        assert_eq!(arrow.start_binding.as_ref().unwrap().target, ids[0]);
        assert_eq!(arrow.end_binding.as_ref().unwrap().target, ids[1]);
        assert_eq!(endpoints(&canvas, arrow).0, [140.0, 80.0]);
    }

    #[test]
    fn suggestions_follow_the_cursor_and_saved_anchors_follow_move_resize_and_duplicate() {
        let mut canvas = diagram();
        for (cursor, expected) in [
            ([110.0, 20.0], [100.0, 20.0]),
            ([-10.0, 60.0], [0.0, 60.0]),
            ([30.0, -10.0], [30.0, 0.0]),
            ([70.0, 90.0], [70.0, 80.0]),
        ] {
            let (binding, anchor) = suggest_binding(&canvas, cursor, None, 22.0).unwrap();
            assert_eq!(binding.target, 1);
            assert!(distance(anchor, expected) < 0.001);
            canvas.element_mut(3).unwrap().start_binding = Some(binding);
            canvas.element_mut(3).unwrap().points[1] = [-500.0, -500.0];
            canvas.element_mut(3).unwrap().end_binding = None;
            canvas.element_mut(3).unwrap().bend = Some([-500.0, -500.0]);
            assert!(distance(endpoints(&canvas, canvas.element(3).unwrap()).0, anchor) < 0.001);
        }
        let (binding, _) = suggest_binding(&canvas, [110.0, 20.0], None, 22.0).unwrap();
        canvas.element_mut(3).unwrap().start_binding = Some(binding);
        canvas.element_mut(1).unwrap().translate([20.0, 30.0]);
        canvas.element_mut(1).unwrap().size = [200.0, 160.0];
        assert_eq!(
            endpoints(&canvas, canvas.element(3).unwrap()).0,
            [220.0, 70.0]
        );
        let ids = canvas.duplicate(&[1, 3], [50.0, 50.0]);
        assert_eq!(
            endpoints(&canvas, canvas.element(ids[1]).unwrap()).0,
            [270.0, 120.0]
        );
        let mut restored: CanvasData =
            serde_json::from_str(&serde_json::to_string(&canvas).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(canvas, restored);
        let legacy: Binding = serde_json::from_str(r#"{"target":1}"#).unwrap();
        assert!(legacy.anchor.is_none());
        restored
            .element_mut(3)
            .unwrap()
            .start_binding
            .as_mut()
            .unwrap()
            .anchor = Some([f32::NAN, 0.5]);
        assert!(restored.validate().is_err());
    }

    #[test]
    fn transformed_shapes_reroute_both_arrow_ends_outside_the_connected_shapes() {
        for kind in [
            ElementKind::Rectangle,
            ElementKind::Ellipse,
            ElementKind::Diamond,
            ElementKind::Sticky,
        ] {
            for route in [ArrowRoute::Straight, ArrowRoute::Elbow, ArrowRoute::Curve] {
                for position in [
                    [500.0, 80.0],
                    [290.0, 220.0],
                    [290.0, -180.0],
                    [-100.0, 160.0],
                ] {
                    let mut canvas = diagram();
                    canvas.element_mut(1).unwrap().kind = kind;
                    canvas.element_mut(2).unwrap().kind = kind;
                    let arrow = canvas.element_mut(3).unwrap();
                    arrow.route = route;
                    arrow.start_binding.as_mut().unwrap().anchor = Some([1.0, 0.5]);
                    arrow.end_binding.as_mut().unwrap().anchor = Some([0.0, 0.5]);
                    canvas.element_mut(1).unwrap().pos = position;
                    assert!(canvas.reroute_connections(&[1]));
                    let path = arrow_path(&canvas, canvas.element(3).unwrap());
                    for target in &canvas.elements[..2] {
                        let mut shape = target.clone();
                        shape.style.rough = false;
                        let outline = outline(&shape);
                        for segment in path.windows(2) {
                            for step in 1..100 {
                                let t = step as f32 / 100.0;
                                let point = add(
                                    segment[0],
                                    [
                                        (segment[1][0] - segment[0][0]) * t,
                                        (segment[1][1] - segment[0][1]) * t,
                                    ],
                                );
                                let depth = outline
                                    .windows(2)
                                    .map(|edge| segment_distance(point, edge[0], edge[1]))
                                    .fold(f32::INFINITY, f32::min);
                                assert!(
                                    !in_polygon(point, &outline) || depth < 0.05,
                                    "{kind:?} {route:?} moved to {position:?}: arrow enters shape {} at {point:?}; path {path:?}",
                                    target.id
                                );
                            }
                        }
                    }
                    let mut restored: CanvasData =
                        serde_json::from_str(&serde_json::to_string(&canvas).unwrap()).unwrap();
                    restored.validate().unwrap();
                    assert_eq!(arrow_path(&restored, restored.element(3).unwrap()), path);
                }
            }
        }
    }

    #[test]
    fn suggestions_use_the_visible_outline_of_ovals_and_diamonds() {
        for kind in [ElementKind::Ellipse, ElementKind::Diamond] {
            let mut canvas = CanvasData::default();
            let mut shape = Element::new(canvas.id(), kind, [0.0, 0.0], Style::default());
            shape.size = [200.0, 200.0];
            canvas.elements.push(shape);
            assert!(suggest_binding(&canvas, [200.0, 0.0], None, 22.0).is_none());
            for (cursor, expected) in [
                ([210.0, 100.0], [200.0, 100.0]),
                ([-10.0, 100.0], [0.0, 100.0]),
                ([100.0, -10.0], [100.0, 0.0]),
                ([100.0, 210.0], [100.0, 200.0]),
            ] {
                let (_, anchor) = suggest_binding(&canvas, cursor, None, 22.0).unwrap();
                assert!(distance(anchor, expected) < 0.001);
            }
            assert!(suggest_binding(&canvas, [210.0, 100.0], Some(1), 22.0).is_none());
        }
    }
}
