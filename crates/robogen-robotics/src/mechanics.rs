//! Structural validation only; no loop closure, mobility or motion solver.
use robogen_domain::{
    Diagnostic, RigidBodyId,
    mechanics::{JointKind, KinematicJoint, MechanicalModel},
};
use std::collections::{BTreeMap, BTreeSet};

/// Validate declared bodies and joints without inferring mechanical connections.
///
/// Closed mobile chains and isolated bodies are valid descriptions. Success does
/// not establish that their rest geometry closes, or that motion is possible.
pub fn validate_mechanism(model: &MechanicalModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut parts = BTreeSet::new();
    for (id, body) in &model.bodies {
        if *id != body.id {
            diagnostics.push(Diagnostic::error(
                "E370",
                "Rigid body key and record identity disagree",
                body.span,
            ));
        }
        for part in &body.parts {
            if !parts.insert(*part) {
                diagnostics.push(Diagnostic::error(
                    "E372",
                    "A part may belong to at most one rigid body",
                    body.span,
                ));
            }
        }
    }

    let components = fixed_components(model);
    let mut actuators = BTreeSet::new();
    for (id, joint) in &model.joints {
        if *id != joint.id {
            diagnostics.push(Diagnostic::error(
                "E371",
                "Joint key and record identity disagree",
                joint.span,
            ));
        }
        let valid_endpoints = joint.parent != joint.child
            && model.bodies.contains_key(&joint.parent)
            && model.bodies.contains_key(&joint.child);
        if !valid_endpoints {
            diagnostics.push(Diagnostic::error(
                "E373",
                "Joint endpoints must exist and identify distinct bodies",
                joint.span,
            ));
        }
        validate_joint_fields(joint, &mut diagnostics);
        if let Some(actuator) = joint.actuator {
            if joint.kind != JointKind::Revolute {
                diagnostics.push(Diagnostic::error(
                    "E377",
                    "An actuator may drive a revolute joint only",
                    joint.span,
                ));
            }
            if !actuators.insert(actuator) {
                diagnostics.push(Diagnostic::error(
                    "E378",
                    "An actuator may drive at most one joint",
                    joint.span,
                ));
            }
        }
        if valid_endpoints
            && joint.kind != JointKind::Fixed
            && components.get(&joint.parent) == components.get(&joint.child)
        {
            diagnostics.push(Diagnostic::error(
                "E379",
                "Moving joint endpoints are locked by fixed connections or ground",
                joint.span,
            ));
        }
    }
    diagnostics
}

fn validate_joint_fields(joint: &KinematicJoint, diagnostics: &mut Vec<Diagnostic>) {
    if joint
        .origin
        .iter()
        .any(|length| !length.metres().is_finite())
    {
        diagnostics.push(Diagnostic::error(
            "E374",
            "Joint origin must contain finite lengths",
            joint.span,
        ));
    }
    let valid_axis = match joint.axis {
        Some(axis) => {
            axis.0.iter().all(|value| value.is_finite())
                && (axis.0.iter().map(|value| value * value).sum::<f64>() - 1.0).abs() <= 1e-9
        }
        None => joint.kind == JointKind::Fixed,
    };
    if !valid_axis {
        diagnostics.push(Diagnostic::error(
            "E375",
            "Moving joints require a finite unit axis; supplied axes must be unit directions",
            joint.span,
        ));
    }
    let valid_pitch = match (joint.kind, joint.pitch) {
        (JointKind::Helical, Some(pitch)) => pitch.metres().is_finite() && pitch.metres() != 0.0,
        (JointKind::Helical, None) => false,
        (_, pitch) => pitch.is_none(),
    };
    if !valid_pitch {
        diagnostics.push(Diagnostic::error(
            "E376",
            "Only helical joints require a finite nonzero signed pitch",
            joint.span,
        ));
    }
}

/// Ground is a shared fixed connection. Components use body IDs, not entity indices.
fn fixed_components(model: &MechanicalModel) -> BTreeMap<RigidBodyId, RigidBodyId> {
    let mut neighbors: BTreeMap<RigidBodyId, Vec<RigidBodyId>> = BTreeMap::new();
    let mut connect = |parent, child| {
        neighbors.entry(parent).or_default().push(child);
        neighbors.entry(child).or_default().push(parent);
    };
    let mut grounded = model.bodies.iter().filter(|(_, body)| body.grounded);
    if let Some((first, _)) = grounded.next() {
        for (other, _) in grounded {
            connect(*first, *other);
        }
    }
    for joint in model.joints.values() {
        if joint.kind == JointKind::Fixed
            && model.bodies.contains_key(&joint.parent)
            && model.bodies.contains_key(&joint.child)
        {
            connect(joint.parent, joint.child);
        }
    }
    let mut components = BTreeMap::new();
    for id in model.bodies.keys() {
        if components.contains_key(id) {
            continue;
        }
        let mut pending = vec![*id];
        components.insert(*id, *id);
        while let Some(body) = pending.pop() {
            for neighbor in neighbors.get(&body).into_iter().flatten() {
                if !components.contains_key(neighbor) {
                    components.insert(*neighbor, *id);
                    pending.push(*neighbor);
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests;
