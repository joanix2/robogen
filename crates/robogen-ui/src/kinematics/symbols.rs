//! Compact schematic conventions, oriented by the declared joint axis.
use egui::{vec2, Color32, Painter, Pos2, Stroke, Vec2};
use robogen_domain::mechanics::JointKind;

pub(super) fn ground(painter: &Painter, at: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    painter.line_segment([at - vec2(15.0, 0.0), at + vec2(15.0, 0.0)], stroke);
    for x in [-12.0, -4.0, 4.0, 12.0] {
        painter.line_segment([at + vec2(x, 0.0), at + vec2(x - 6.0, 7.0)], stroke);
    }
}

pub(super) fn joint(
    painter: &Painter,
    at: Pos2,
    direction: Vec2,
    kind: JointKind,
    a: Color32,
    b: Color32,
) {
    let perpendicular = direction.length_sq() < 0.0001;
    let u = if perpendicular {
        vec2(1.0, 0.0)
    } else {
        direction.normalized()
    };
    let v = vec2(-u.y, u.x);
    let point = |x, y| at + u * x + v * y;
    let line = |a, b, color| {
        painter.line_segment([a, b], Stroke::new(2.0, color));
    };
    painter.circle_filled(at, 20.0, Color32::from_rgb(16, 23, 33));
    line(point(-23.0, 0.0), point(-12.0, 0.0), a);
    line(point(12.0, 0.0), point(23.0, 0.0), b);
    match kind {
        JointKind::Revolute => {
            painter.circle_stroke(at, 11.0, Stroke::new(2.0, a));
            line(point(0.0, -8.0), point(0.0, 8.0), b);
            line(point(0.0, 0.0), point(23.0, 0.0), b);
        }
        JointKind::Prismatic | JointKind::Cylindrical | JointKind::Helical => {
            let corners = [
                point(-12.0, -9.0),
                point(12.0, -9.0),
                point(12.0, 9.0),
                point(-12.0, 9.0),
                point(-12.0, -9.0),
            ];
            for pair in corners.windows(2) {
                line(pair[0], pair[1], a);
            }
            line(point(-7.0, 0.0), point(23.0, 0.0), b);
            match kind {
                JointKind::Prismatic => line(point(-10.0, 7.0), point(10.0, -7.0), b),
                JointKind::Cylindrical => {
                    painter.circle_stroke(point(7.0, 0.0), 5.0, Stroke::new(1.5, b));
                }
                JointKind::Helical => {
                    let wave = [
                        point(-9.0, 5.0),
                        point(-5.0, -5.0),
                        point(0.0, 5.0),
                        point(5.0, -5.0),
                        point(9.0, 5.0),
                    ];
                    for pair in wave.windows(2) {
                        line(pair[0], pair[1], b);
                    }
                }
                _ => {}
            }
        }
        JointKind::Fixed => {
            line(point(-3.0, -12.0), point(-3.0, 12.0), a);
            line(point(3.0, -12.0), point(3.0, 12.0), b);
            for y in [-9.0, 0.0, 9.0] {
                line(point(-10.0, y + 4.0), point(-3.0, y), a);
            }
        }
    }
    if perpendicular {
        let dot = at + vec2(22.0, -19.0);
        painter.circle_stroke(dot, 4.0, Stroke::new(1.0, b));
        painter.circle_filled(dot, 1.2, b);
    } else {
        let tip = point(32.0, 0.0);
        line(point(25.0, 0.0), tip, Color32::GRAY);
        line(tip, point(29.0, -3.0), Color32::GRAY);
        line(tip, point(29.0, 3.0), Color32::GRAY);
    }
}
