use super::*;
use robogen_domain::{
    mechanics::{AxisDirection, JointKind, KinematicJoint, RigidBody},
    Length,
};

fn model() -> MechanicalModel {
    let mut model = MechanicalModel::default();
    for name in ["A", "B", "C"] {
        let id = RigidBodyId::from_name(name);
        model.bodies.insert(
            id,
            RigidBody {
                id,
                name: name.into(),
                parts: vec![],
                grounded: name == "A",
                span: SourceSpan::default(),
            },
        );
    }
    for (name, a, b, position) in [
        ("AB", "A", "B", [0., 0., 0.]),
        ("BC", "B", "C", [60., 30., 0.]),
        ("AC", "A", "C", [0., 60., 30.]),
    ] {
        let id = JointId::from_name(name);
        model.joints.insert(
            id,
            KinematicJoint {
                id,
                name: name.into(),
                parent: RigidBodyId::from_name(a),
                child: RigidBodyId::from_name(b),
                kind: JointKind::Revolute,
                origin: position.map(Length::from_millimetres),
                axis: Some(AxisDirection([1., 0., 0.])),
                pitch: None,
                actuator: None,
                span: SourceSpan::default(),
            },
        );
    }
    model
}
#[test]
fn zoom_preserves_world_point_under_pointer_and_clamps() {
    let mut canvas = GraphCanvas::default();
    let rect = Rect::from_min_size(pos2(80., 120.), vec2(800., 500.));
    canvas.pan = vec2(50., -40.);
    let world = pos2(100., -70.);
    let pointer = canvas.screen(rect, world);
    canvas.zoom_at(rect, pointer, 1.5);
    assert!(canvas.screen(rect, world).distance(pointer) < 0.001);
    canvas.zoom_at(rect, pointer, 100.);
    assert_eq!(canvas.zoom, 8.);
    canvas.zoom_at(rect, pointer, 0.0001);
    assert_eq!(canvas.zoom, 0.02);
}
#[test]
fn projection_uses_document_axes_and_picking_selects_topmost_symbol() {
    assert_eq!(Projection::Xy.project([10., 20., 30.]), pos2(10., -20.));
    assert_eq!(Projection::Xz.project([10., 20., 30.]), pos2(10., -30.));
    assert_eq!(Projection::Yz.project([10., 20., 30.]), pos2(20., -30.));
    let rect = Rect::from_center_size(pos2(20., 20.), vec2(30., 30.));
    let body = Selection::Body(RigidBodyId::from_name("A"));
    let joint = Selection::Joint(JointId::from_name("AB"));
    assert_eq!(
        pick(
            &[
                HitRegion {
                    selection: body,
                    rect
                },
                HitRegion {
                    selection: joint,
                    rect
                }
            ],
            rect.center()
        ),
        Some(joint)
    );
    assert_eq!(
        pick(
            &[HitRegion {
                selection: body,
                rect
            }],
            pos2(200., 200.)
        ),
        None
    );
}
fn frame(
    canvas: &mut GraphCanvas,
    model: &MechanicalModel,
    context: &egui::Context,
    events: Vec<egui::Event>,
) -> Rect {
    let mut rect = Rect::NOTHING;
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900., 600.))),
            events,
            ..Default::default()
        },
        |context| {
            egui::CentralPanel::default().show(context, |ui| {
                rect = ui.available_rect_before_wrap();
                canvas.show(ui, model);
            });
        },
    );
    rect
}
fn button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}
#[test]
fn pointer_drag_moves_only_graph_layout_and_schematic_drag_pans() {
    let model = model();
    let original = model.clone();
    let id = RigidBodyId::from_name("A");
    let context = egui::Context::default();
    let mut canvas = GraphCanvas::default();
    let rect = frame(&mut canvas, &model, &context, vec![]);
    let initial = canvas.positions[&id];
    let node = canvas.screen(rect, initial);
    frame(
        &mut canvas,
        &model,
        &context,
        vec![egui::Event::PointerMoved(node), button(node, true)],
    );
    let end = node + vec2(100., 60.);
    frame(
        &mut canvas,
        &model,
        &context,
        vec![egui::Event::PointerMoved(end)],
    );
    frame(&mut canvas, &model, &context, vec![button(end, false)]);
    assert_eq!(canvas.selected, Some(Selection::Body(id)));
    assert!(canvas.positions[&id].distance(initial) > 30.);
    canvas.reset_layout();
    frame(&mut canvas, &model, &context, vec![]);
    assert_eq!(canvas.positions[&id], initial);
    canvas.set_mode(DiagramMode::Schematic);
    frame(&mut canvas, &model, &context, vec![]);
    let pan = canvas.pan;
    let background = rect.left_bottom() + vec2(12., -12.);
    frame(
        &mut canvas,
        &model,
        &context,
        vec![
            egui::Event::PointerMoved(background),
            button(background, true),
        ],
    );
    let end = background + vec2(50., -40.);
    frame(
        &mut canvas,
        &model,
        &context,
        vec![egui::Event::PointerMoved(end)],
    );
    frame(&mut canvas, &model, &context, vec![button(end, false)]);
    assert!((canvas.pan - pan).length() > 40.);
    assert_eq!(canvas.positions[&id], initial);
    assert_eq!(model, original);
    frame(&mut canvas, &MechanicalModel::default(), &context, vec![]);
    assert!(canvas.selected.is_none());
}
#[test]
fn graph_layout_keeps_all_vertices_of_closed_and_disconnected_components() {
    let mut model = model();
    let isolated = RigidBodyId::from_name("D");
    model.bodies.insert(
        isolated,
        RigidBody {
            id: isolated,
            name: "D".into(),
            parts: vec![],
            grounded: false,
            span: SourceSpan::default(),
        },
    );
    let positions = layout::graph_positions(&model);
    assert_eq!(positions.len(), 4);
    assert_eq!(positions, layout::graph_positions(&model));
    for a in positions.values() {
        for b in positions.values() {
            if a != b {
                assert!(a.distance(*b) > 50.);
            }
        }
    }
    assert_eq!(model.joints.len(), 3);
}

fn add_body(model: &mut MechanicalModel, name: &str) -> RigidBodyId {
    let id = RigidBodyId::from_name(name);
    model.bodies.insert(
        id,
        RigidBody {
            id,
            name: name.into(),
            parts: vec![],
            grounded: false,
            span: SourceSpan::default(),
        },
    );
    id
}

fn add_joint(model: &mut MechanicalModel, parent: RigidBodyId, child: RigidBodyId) {
    let id = JointId::from_name(&format!("{parent:?}{child:?}"));
    model.joints.insert(
        id,
        KinematicJoint {
            id,
            name: "J".into(),
            parent,
            child,
            kind: JointKind::Revolute,
            origin: [Length::from_millimetres(0.); 3],
            axis: Some(AxisDirection([1., 0., 0.])),
            pitch: None,
            actuator: None,
            span: SourceSpan::default(),
        },
    );
}

#[test]
fn grounded_body_is_the_root_and_loop_chords_keep_layers() {
    let positions = layout::graph_positions(&model());
    let [a, b, c] = ["A", "B", "C"].map(|name| positions[&RigidBodyId::from_name(name)]);
    assert_eq!(a.y, 0.);
    assert_eq!(b.y, c.y);
    assert!(b.y > a.y);
    assert!((a.x - (b.x + c.x) / 2.).abs() < 0.01);
}

#[test]
fn branching_chains_get_separate_columns_and_stay_apart() {
    let mut model = MechanicalModel::default();
    let torso = add_body(&mut model, "Torso");
    for side in ["L", "R"] {
        for limb in ["Leg", "Arm"] {
            let mut previous = torso;
            for link in 0..5 {
                let next = add_body(&mut model, &format!("{side}{limb}{link}"));
                add_joint(&mut model, previous, next);
                previous = next;
            }
        }
    }
    let positions = layout::graph_positions(&model);
    assert_eq!(positions.len(), 21);
    assert_eq!(positions[&torso].y, 0.);
    for (index, a) in positions.values().enumerate() {
        for b in positions.values().skip(index + 1) {
            assert!(a.distance(*b) >= 109., "{a:?} {b:?}");
        }
    }
}
