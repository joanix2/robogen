//! Deterministic layered layout from explicit connectivity; loops remain ordinary edges.
use egui::{pos2, Pos2};
use robogen_domain::{mechanics::MechanicalModel, RigidBodyId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const COLUMN: f32 = 260.0;
const ROW: f32 = 110.0;

/// Places each component as a spanning tree rooted at its grounded body (or its most
/// connected body). Layer = distance from the root; chords of a loop do not affect it.
pub(super) fn graph_positions(model: &MechanicalModel) -> BTreeMap<RigidBodyId, Pos2> {
    let mut neighbours: BTreeMap<RigidBodyId, Vec<RigidBodyId>> = BTreeMap::new();
    for joint in model.joints.values() {
        neighbours
            .entry(joint.parent)
            .or_default()
            .push(joint.child);
        neighbours
            .entry(joint.child)
            .or_default()
            .push(joint.parent);
    }
    for list in neighbours.values_mut() {
        list.sort_by_key(|id| &model.bodies[id].name);
        list.dedup();
    }
    let degree = |id: &RigidBodyId| neighbours.get(id).map_or(0, Vec::len);
    let mut order: Vec<_> = model.bodies.keys().copied().collect();
    order.sort_by_key(|id| {
        (
            !model.bodies[id].grounded,
            std::cmp::Reverse(degree(id)),
            &model.bodies[id].name,
        )
    });

    let mut placed = BTreeSet::new();
    let mut result = BTreeMap::new();
    let mut offset = 0.0;
    for root in order {
        if placed.contains(&root) {
            continue;
        }
        let mut children: BTreeMap<RigidBodyId, Vec<RigidBodyId>> = BTreeMap::new();
        let mut depth = BTreeMap::from([(root, 0usize)]);
        let mut queue = VecDeque::from([root]);
        placed.insert(root);
        while let Some(id) = queue.pop_front() {
            for &other in neighbours.get(&id).into_iter().flatten() {
                if placed.insert(other) {
                    depth.insert(other, depth[&id] + 1);
                    children.entry(id).or_default().push(other);
                    queue.push_back(other);
                }
            }
        }
        let mut cursor = 0.0;
        let columns = place(root, &children, &depth, &mut cursor, &mut result);
        for id in depth.keys() {
            if let Some(point) = result.get_mut(id) {
                point.x += offset;
            }
        }
        offset += (columns + 1.0) * COLUMN;
    }
    result
}

/// Leaves take consecutive columns; a parent is centred above its children.
/// Returns the number of columns used by the subtree.
fn place(
    id: RigidBodyId,
    children: &BTreeMap<RigidBodyId, Vec<RigidBodyId>>,
    depth: &BTreeMap<RigidBodyId, usize>,
    cursor: &mut f32,
    result: &mut BTreeMap<RigidBodyId, Pos2>,
) -> f32 {
    let start = *cursor;
    let x = match children.get(&id) {
        Some(list) if !list.is_empty() => {
            let mut xs = Vec::new();
            for child in list {
                place(*child, children, depth, cursor, result);
                xs.push(result[child].x);
            }
            (xs[0] + xs[xs.len() - 1]) / 2.0
        }
        _ => {
            let x = *cursor * COLUMN;
            *cursor += 1.0;
            x
        }
    };
    result.insert(id, pos2(x, depth[&id] as f32 * ROW));
    *cursor - start
}
