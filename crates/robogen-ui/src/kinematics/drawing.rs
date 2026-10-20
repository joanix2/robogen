//! Painting only: semantic types and physical coordinates come from the snapshot.
use super::canvas::{DiagramMode, GraphCanvas, HitRegion, Selection};
use super::symbols;
use crate::{BORDER, MUTED, TEXT};
use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Ui};
use robogen_domain::{
    mechanics::{JointKind, KinematicJoint, MechanicalModel},
    RigidBodyId,
};
use std::collections::BTreeMap;

const COLORS: [Color32; 8] = [
    Color32::from_rgb(96, 166, 250),
    Color32::from_rgb(103, 214, 145),
    Color32::from_rgb(250, 195, 77),
    Color32::from_rgb(221, 134, 229),
    Color32::from_rgb(244, 133, 104),
    Color32::from_rgb(78, 208, 217),
    Color32::from_rgb(177, 187, 244),
    Color32::from_rgb(224, 172, 128),
];

pub(super) fn joint_label(joint: &KinematicJoint) -> String {
    let kind = match joint.kind {
        JointKind::Fixed => "Encastrement",
        JointKind::Revolute => "Pivot",
        JointKind::Prismatic => "Glissière",
        JointKind::Cylindrical => "Pivot glissant",
        JointKind::Helical => "Hélicoïdale",
    };
    let axis = joint
        .axis
        .map(|axis| {
            for (index, name) in ["X", "Y", "Z"].iter().enumerate() {
                if axis.0[index].abs() > 0.999999 {
                    return format!(
                        " · axe {}{name}",
                        if axis.0[index] < 0.0 { "−" } else { "" }
                    );
                }
            }
            format!(
                " · axe [{:.2}, {:.2}, {:.2}]",
                axis.0[0], axis.0[1], axis.0[2]
            )
        })
        .unwrap_or_default();
    let pitch = joint
        .pitch
        .map(|p| format!(" · pas {:.1} mm/tr", p.millimetres()))
        .unwrap_or_default();
    format!("{kind}{axis}{pitch}")
}

pub(super) fn paint(
    canvas: &GraphCanvas,
    ui: &Ui,
    rect: Rect,
    model: &MechanicalModel,
) -> Vec<HitRegion> {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(16, 23, 33));
    let spacing = (32.0 * canvas.zoom).clamp(12.0, 80.0);
    let start = canvas.screen(rect, Pos2::ZERO);
    let mut x = rect.left() + (start.x - rect.left()).rem_euclid(spacing);
    while x < rect.right() {
        let mut y = rect.top() + (start.y - rect.top()).rem_euclid(spacing);
        while y < rect.bottom() {
            painter.circle_filled(pos2(x, y), 0.8, BORDER);
            y += spacing;
        }
        x += spacing;
    }
    let mut bodies: Vec<_> = model.bodies.values().collect();
    bodies.sort_by_key(|body| &body.name);
    let colors: BTreeMap<_, _> = bodies
        .iter()
        .enumerate()
        .map(|(i, body)| (body.id, COLORS[i % COLORS.len()]))
        .collect();
    let hits = if canvas.mode == DiagramMode::Graph {
        graph(canvas, &painter, rect, model, &colors)
    } else {
        schematic(canvas, &painter, rect, model, &colors)
    };
    painter.text(
        rect.left_top() + vec2(12.0, 12.0),
        Align2::LEFT_TOP,
        if canvas.mode == DiagramMode::Graph {
            "Corps = nœuds · liaisons = arêtes"
        } else {
            "Projection au repos · couleurs par corps · positions en mm"
        },
        FontId::proportional(11.0),
        MUTED,
    );
    hits
}

fn text_hit(
    painter: &Painter,
    point: Pos2,
    align: Align2,
    label: &str,
    color: Color32,
    selection: Selection,
    hits: &mut Vec<HitRegion>,
) -> Rect {
    let rect = painter.text(point, align, label, FontId::proportional(11.0), color);
    hits.push(HitRegion {
        selection,
        rect: rect.expand(4.0),
    });
    rect
}

/// Returns the first free candidate anchor; otherwise raises the first one until it clears.
fn place_label(
    painter: &Painter,
    candidates: &[(Pos2, Align2)],
    label: &str,
    placed: &mut Vec<Rect>,
) -> (Pos2, Align2) {
    let size = painter
        .layout_no_wrap(label.to_owned(), FontId::proportional(11.0), TEXT)
        .size();
    let free = |anchor: Pos2, align: Align2, placed: &[Rect]| {
        let rect = align.anchor_size(anchor, size).expand(2.0);
        !placed.iter().any(|other| other.intersects(rect))
    };
    let chosen = candidates
        .iter()
        .find(|(anchor, align)| free(*anchor, *align, placed))
        .copied()
        .unwrap_or_else(|| {
            let (mut anchor, align) = candidates[0];
            for _ in 0..16 {
                if free(anchor, align, placed) {
                    break;
                }
                anchor.y -= size.y + 4.0;
            }
            (anchor, align)
        });
    placed.push(chosen.1.anchor_size(chosen.0, size));
    chosen
}

fn graph(
    canvas: &GraphCanvas,
    painter: &Painter,
    rect: Rect,
    model: &MechanicalModel,
    colors: &BTreeMap<RigidBodyId, Color32>,
) -> Vec<HitRegion> {
    let mut hits = Vec::new();
    let mut edges = Vec::new();
    for joint in model.joints.values() {
        let (Some(&a), Some(&b)) = (
            canvas.positions.get(&joint.parent),
            canvas.positions.get(&joint.child),
        ) else {
            continue;
        };
        let a = canvas.screen(rect, a);
        let b = canvas.screen(rect, b);
        let normal = vec2(-(b - a).y, (b - a).x).normalized();
        // Parallel edges receive distinct lanes, including reversed declarations.
        let parallel: Vec<_> = model
            .joints
            .values()
            .filter(|j| {
                (j.parent == joint.parent && j.child == joint.child)
                    || (j.child == joint.parent && j.parent == joint.child)
            })
            .collect();
        let rank = parallel.iter().position(|j| j.id == joint.id).unwrap_or(0);
        let lane = rank as f32 - parallel.len().saturating_sub(1) as f32 / 2.0;
        let normal = if joint.parent < joint.child {
            normal
        } else {
            -normal
        };
        let middle = a.lerp(b, 0.5) + normal * lane * 90.0;
        let selected = canvas.selected == Some(Selection::Joint(joint.id));
        let width = if selected { 3.0 } else { 1.8 };
        painter.line_segment([a, middle], Stroke::new(width, colors[&joint.parent]));
        painter.line_segment([middle, b], Stroke::new(width, colors[&joint.child]));
        painter.circle_filled(middle, 4.0, if selected { Color32::WHITE } else { MUTED });
        edges.push((joint, a, middle, b));
    }
    // Nodes and their names are painted before joint labels, which then avoid them.
    let mut placed: Vec<Rect> = Vec::new();
    for body in model.bodies.values() {
        let Some(&position) = canvas.positions.get(&body.id) else {
            continue;
        };
        let point = canvas.screen(rect, position);
        let color = colors[&body.id];
        let selected = canvas.selected == Some(Selection::Body(body.id));
        painter.circle_filled(point, 20.0, color.gamma_multiply(0.22));
        painter.circle_stroke(
            point,
            20.0,
            Stroke::new(if selected { 3.5 } else { 2.0 }, color),
        );
        let selection = Selection::Body(body.id);
        placed.push(Rect::from_center_size(point, vec2(44.0, 44.0)));
        let name_rect = painter.text(
            point + vec2(26.0, 4.0),
            Align2::LEFT_TOP,
            &body.name,
            FontId::proportional(11.0),
            color,
        );
        placed.push(name_rect);
        hits.push(HitRegion {
            selection,
            rect: name_rect.expand(4.0),
        });
        hits.push(HitRegion {
            selection,
            rect: Rect::from_center_size(point, vec2(48.0, 48.0)),
        });
        if body.grounded {
            symbols::ground(painter, point + vec2(0.0, -29.0), color);
        }
    }
    for (joint, a, middle, b) in edges {
        let selection = Selection::Joint(joint.id);
        let label = format!("{}\n{}", joint.name, joint_label(joint));
        // Near the child, sibling edges of a fan-out are furthest apart.
        let mut candidates = Vec::new();
        for shift in [0.0, 30.0, -30.0, 60.0, -60.0] {
            for along in [0.5, 0.65, 0.35, 0.8] {
                let at = if along < 0.5 {
                    a.lerp(middle, along * 2.0)
                } else {
                    middle.lerp(b, (along - 0.5) * 2.0)
                } + vec2(0.0, shift);
                candidates.push((at + vec2(10.0, 0.0), Align2::LEFT_CENTER));
                candidates.push((at + vec2(-10.0, 0.0), Align2::RIGHT_CENTER));
            }
        }
        let (anchor, align) = place_label(painter, &candidates, &label, &mut placed);
        text_hit(painter, anchor, align, &label, TEXT, selection, &mut hits);
        hits.push(HitRegion {
            selection,
            rect: Rect::from_center_size(middle, vec2(16.0, 16.0)),
        });
    }
    hits
}

fn schematic(
    canvas: &GraphCanvas,
    painter: &Painter,
    rect: Rect,
    model: &MechanicalModel,
    colors: &BTreeMap<RigidBodyId, Color32>,
) -> Vec<HitRegion> {
    let mut hits = Vec::new();
    let points: BTreeMap<_, _> = model
        .joints
        .values()
        .map(|joint| {
            (
                joint.id,
                canvas.screen(
                    rect,
                    canvas
                        .projection
                        .project(joint.origin.map(|v| v.millimetres())),
                ),
            )
        })
        .collect();
    let mut memberships: BTreeMap<RigidBodyId, Vec<Pos2>> = BTreeMap::new();
    for joint in model.joints.values() {
        let center = points[&joint.id];
        let direction = joint
            .axis
            .map(|a| canvas.projection.project(a.0).to_vec2())
            .unwrap_or(vec2(1.0, 0.0));
        let direction = if direction.length_sq() < 0.0001 {
            vec2(1.0, 0.0)
        } else {
            direction.normalized()
        };
        memberships
            .entry(joint.parent)
            .or_default()
            .push(center - direction * 23.0);
        memberships
            .entry(joint.child)
            .or_default()
            .push(center + direction * 23.0);
    }
    let mut isolated = 0;
    let mut label_rects: Vec<Rect> = Vec::new();
    for body in model.bodies.values() {
        let color = colors[&body.id];
        let anchor = if let Some(points) = memberships.get(&body.id) {
            let center = Pos2::ZERO
                + points
                    .iter()
                    .fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2())
                    / points.len() as f32;
            let anchor = if points.len() == 1 {
                center + vec2(0.0, 42.0)
            } else {
                center
            };
            let width = if canvas.selected == Some(Selection::Body(body.id)) {
                4.0
            } else {
                2.0
            };
            for point in points {
                painter.line_segment([*point, pos2(anchor.x, point.y)], Stroke::new(width, color));
                painter.line_segment([pos2(anchor.x, point.y), anchor], Stroke::new(width, color));
            }
            anchor
        } else {
            isolated += 1;
            rect.left_bottom() + vec2(90.0 + isolated as f32 * 110.0, -38.0)
        };
        let label = if memberships.contains_key(&body.id) {
            body.name.clone()
        } else {
            format!("{} · isolé", body.name)
        };
        label_rects.push(text_hit(
            painter,
            anchor + vec2(5.0, 10.0),
            Align2::LEFT_TOP,
            &label,
            color,
            Selection::Body(body.id),
            &mut hits,
        ));
        if body.grounded {
            symbols::ground(painter, anchor + vec2(0.0, 37.0), color);
        }
    }
    let mut overlap = false;
    let mut placed: Vec<Pos2> = Vec::new();
    for joint in model.joints.values() {
        let point = points[&joint.id];
        if placed.iter().any(|p| p.distance(point) < 36.0) {
            overlap = true;
        }
        placed.push(point);
        let direction = joint
            .axis
            .map(|a| canvas.projection.project(a.0).to_vec2())
            .unwrap_or(vec2(1.0, 0.0));
        symbols::joint(
            painter,
            point,
            direction,
            joint.kind,
            colors[&joint.parent],
            colors[&joint.child],
        );
        if canvas.selected == Some(Selection::Joint(joint.id)) {
            painter.circle_stroke(point, 25.0, Stroke::new(1.0, Color32::WHITE));
        }
        let selection = Selection::Joint(joint.id);
        let label = format!("{}\n{}", joint.name, joint_label(joint));
        let (anchor, align) = place_label(
            painter,
            &[(point + vec2(0.0, -28.0), Align2::CENTER_BOTTOM)],
            &label,
            &mut label_rects,
        );
        text_hit(painter, anchor, align, &label, TEXT, selection, &mut hits);
        hits.push(HitRegion {
            selection,
            rect: Rect::from_center_size(point, vec2(48.0, 48.0)),
        });
    }
    if overlap {
        painter.text(
            rect.left_bottom() + vec2(12.0, -10.0),
            Align2::LEFT_BOTTOM,
            "Liaisons superposées dans ce plan : changer de projection ou zoomer.",
            FontId::proportional(11.0),
            MUTED,
        );
    }
    hits
}
