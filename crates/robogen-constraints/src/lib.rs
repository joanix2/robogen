//! Pluggable sketch constraint solver boundary and a deterministic native MVP.

use robogen_domain::Length;
use robogen_sketch::{ConstraintKind, Sketch, SketchEntity};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct SolveReport {
    pub sketch: Sketch,
    pub remaining_degrees_of_freedom: usize,
    pub fully_constrained: bool,
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ConstraintError {
    #[error("entity `{0}` does not exist")]
    UnknownEntity(String),
    #[error("conflicting {axis} dimensions: {first:?} and {second:?}")]
    ConflictingDimension {
        axis: &'static str,
        first: Length,
        second: Length,
    },
    #[error("rectangle `{0}` must have positive dimensions")]
    NonPositiveRectangle(String),
}

pub trait ConstraintSolver: Send + Sync {
    fn solve(&self, sketch: &Sketch) -> Result<SolveReport, ConstraintError>;
}

/// Conservative solver for rectangles. It validates constraints and reports
/// degrees of freedom; the backend boundary allows replacing it with a numeric solver.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeConstraintSolver;

impl ConstraintSolver for NativeConstraintSolver {
    fn solve(&self, sketch: &Sketch) -> Result<SolveReport, ConstraintError> {
        let mut dimensions: BTreeMap<(&str, &str), Length> = BTreeMap::new();
        let mut constrained = 0usize;
        validate_positive_dimensions(sketch)?;
        let mut solved = sketch.clone();
        for constraint in &sketch.constraints {
            match &constraint.kind {
                ConstraintKind::Horizontal(path) | ConstraintKind::Vertical(path) => {
                    ensure_entity(sketch, path)?;
                    constrained += 1;
                }
                ConstraintKind::Fix(path) => {
                    ensure_entity(sketch, path)?;
                    constrained += 2;
                }
                ConstraintKind::DistanceX { from, to } | ConstraintKind::DistanceY { from, to } => {
                    ensure_entity(sketch, from)?;
                    ensure_entity(sketch, to)?;
                    let axis = if matches!(constraint.kind, ConstraintKind::DistanceX { .. }) {
                        "x"
                    } else {
                        "y"
                    };
                    if let Some(value) = constraint.value {
                        let entity_name = from.split('.').next().unwrap_or(from);
                        let key = (axis, entity_name);
                        if let Some(first) = dimensions.insert(key, value) {
                            if (first.metres() - value.metres()).abs() > 1e-9 {
                                return Err(ConstraintError::ConflictingDimension {
                                    axis,
                                    first,
                                    second: value,
                                });
                            }
                        }
                        apply_dimension(&mut solved, entity_name, axis, value);
                    }
                    constrained += 1;
                }
            }
        }
        validate_positive_dimensions(&solved)?;
        let total_dof = sketch.entities.len() * 4;
        let remaining = total_dof.saturating_sub(constrained);
        Ok(SolveReport {
            sketch: solved,
            remaining_degrees_of_freedom: remaining,
            fully_constrained: remaining == 0,
        })
    }
}

fn apply_dimension(sketch: &mut Sketch, entity_name: &str, axis: &str, value: Length) {
    for entity in &mut sketch.entities {
        let SketchEntity::Rectangle(rectangle) = entity;
        if rectangle.name == entity_name {
            if axis == "x" {
                rectangle.width = value;
            } else {
                rectangle.height = value;
            }
        }
    }
}

fn validate_positive_dimensions(sketch: &Sketch) -> Result<(), ConstraintError> {
    for entity in &sketch.entities {
        let SketchEntity::Rectangle(rectangle) = entity;
        if rectangle.width.metres() <= 0.0 || rectangle.height.metres() <= 0.0 {
            return Err(ConstraintError::NonPositiveRectangle(
                rectangle.name.clone(),
            ));
        }
    }
    Ok(())
}

fn ensure_entity(sketch: &Sketch, path: &str) -> Result<(), ConstraintError> {
    let root = path.split('.').next().unwrap_or(path);
    if sketch.entities.iter().any(|entity| match entity {
        SketchEntity::Rectangle(rect) => rect.name == root,
    }) {
        Ok(())
    } else {
        Err(ConstraintError::UnknownEntity(root.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robogen_domain::{Point2, SketchId, SourceSpan};
    use robogen_sketch::{Plane, Rectangle, SketchConstraint};

    #[test]
    fn native_solver_rejects_missing_entity() {
        let sketch = Sketch {
            id: SketchId::from_name("x"),
            name: "x".into(),
            plane: Plane::XY,
            entities: vec![SketchEntity::Rectangle(Rectangle {
                name: "box".into(),
                origin: Point2::default(),
                width: Length::from_metres(1.0),
                height: Length::from_metres(1.0),
                span: SourceSpan::default(),
            })],
            constraints: vec![SketchConstraint {
                kind: ConstraintKind::Fix("missing".into()),
                value: None,
                span: SourceSpan::default(),
            }],
            source_span: SourceSpan::default(),
        };
        assert!(matches!(
            NativeConstraintSolver.solve(&sketch),
            Err(ConstraintError::UnknownEntity(_))
        ));
    }

    #[test]
    fn distance_constraint_updates_solved_geometry() -> Result<(), ConstraintError> {
        let sketch = Sketch {
            id: SketchId::from_name("x"),
            name: "x".into(),
            plane: Plane::XY,
            entities: vec![SketchEntity::Rectangle(Rectangle {
                name: "box".into(),
                origin: Point2::default(),
                width: Length::from_metres(1.0),
                height: Length::from_metres(1.0),
                span: SourceSpan::default(),
            })],
            constraints: vec![SketchConstraint {
                kind: ConstraintKind::DistanceX {
                    from: "box.left".into(),
                    to: "box.right".into(),
                },
                value: Some(Length::from_metres(2.0)),
                span: SourceSpan::default(),
            }],
            source_span: SourceSpan::default(),
        };
        let report = NativeConstraintSolver.solve(&sketch)?;
        let SketchEntity::Rectangle(rectangle) = &report.sketch.entities[0];
        assert_eq!(rectangle.width.metres(), 2.0);
        Ok(())
    }
}
