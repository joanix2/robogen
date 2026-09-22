//! Renderer- and solver-independent parametric sketch model.

use robogen_domain::{Length, Point2, SketchId, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Plane {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    pub name: String,
    pub origin: Point2,
    pub width: Length,
    pub height: Length,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SketchEntity {
    Rectangle(Rectangle),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    Horizontal(String),
    Vertical(String),
    DistanceX { from: String, to: String },
    DistanceY { from: String, to: String },
    Fix(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SketchConstraint {
    pub kind: ConstraintKind,
    pub value: Option<Length>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sketch {
    pub id: SketchId,
    pub name: String,
    pub plane: Plane,
    pub entities: Vec<SketchEntity>,
    pub constraints: Vec<SketchConstraint>,
    pub source_span: SourceSpan,
}

impl Sketch {
    pub fn bounding_size(&self) -> Option<(Length, Length)> {
        let SketchEntity::Rectangle(rect) = self.entities.first()?;
        Some((rect.width, rect.height))
    }
}
