//! CAD boundary with native rectangle extrusion and a private mesh CSG adapter.

mod mesh_csg;

use robogen_domain::{FeatureId, Length, PartId, SourceSpan};
use robogen_ir::{Feature, Part, SemanticModel, SolidGeometry};
use robogen_sketch::{Plane, Sketch, SketchEntity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    pub position: [f32; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Triangle {
    pub indices: [u32; 3],
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    pub fn append(&mut self, other: &Mesh) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.triangles
            .extend(other.triangles.iter().map(|triangle| Triangle {
                indices: triangle.indices.map(|index| index + offset),
            }));
    }
}

/// Stable geometry references are semantic, never kernel face indices.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum FaceRef {
    FeatureOutput { feature: FeatureId, role: FaceRole },
    Semantic(String),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum FaceRole {
    StartCap,
    EndCap,
    Side(u32),
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum CadError {
    #[error("sketch `{0}` has no supported closed profile")]
    EmptyProfile(String),
    #[error("extrusion depth must be positive")]
    NonPositiveDepth,
    #[error("feature refers to missing sketch")]
    MissingSketch,
    #[error("part `{0}` has no supported features")]
    EmptyPart(String),
    #[error("solid geometry is unavailable in this CAD backend")]
    SolidUnavailable { span: SourceSpan },
    #[error("invalid solid geometry: {reason}")]
    InvalidSolid { span: SourceSpan, reason: String },
    #[error("solid geometry exceeds the {limit} limit")]
    SolidLimit { span: SourceSpan, limit: &'static str },
    #[error("mesh CSG backend failed")]
    SolidBackendFailure { span: SourceSpan },
}

pub trait CadKernel: Send + Sync {
    fn extrude(&self, sketch: &Sketch, depth: Length) -> Result<Mesh, CadError>;

    fn solid(&self, geometry: &SolidGeometry) -> Result<Mesh, CadError> {
        Err(CadError::SolidUnavailable { span: geometry.span })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeCadKernel;

impl CadKernel for NativeCadKernel {
    fn solid(&self, geometry: &SolidGeometry) -> Result<Mesh, CadError> {
        mesh_csg::build(geometry)
    }

    fn extrude(&self, sketch: &Sketch, depth: Length) -> Result<Mesh, CadError> {
        if depth.metres() <= 0.0 {
            return Err(CadError::NonPositiveDepth);
        }
        let Some(SketchEntity::Rectangle(rect)) = sketch.entities.first() else {
            return Err(CadError::EmptyProfile(sketch.name.clone()));
        };
        let (x, y, w, h, d) = (
            rect.origin.x.metres() as f32,
            rect.origin.y.metres() as f32,
            rect.width.metres() as f32,
            rect.height.metres() as f32,
            depth.metres() as f32,
        );
        let local = [
            [x, y, 0.0],
            [x + w, y, 0.0],
            [x + w, y + h, 0.0],
            [x, y + h, 0.0],
            [x, y, d],
            [x + w, y, d],
            [x + w, y + h, d],
            [x, y + h, d],
        ];
        let position = |p: [f32; 3]| match sketch.plane {
            Plane::XY => p,
            Plane::XZ => [p[0], p[2], p[1]],
            Plane::YZ => [p[2], p[0], p[1]],
        };
        let vertices = local
            .into_iter()
            .map(|p| Vertex {
                position: position(p),
            })
            .collect();
        let faces = [
            [0, 2, 1],
            [0, 3, 2], // start cap
            [4, 5, 6],
            [4, 6, 7], // end cap
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        Ok(Mesh {
            vertices,
            triangles: faces
                .into_iter()
                .map(|indices| Triangle { indices })
                .collect(),
        })
    }
}

pub fn build_part_mesh(
    model: &SemanticModel,
    part: &Part,
    kernel: &dyn CadKernel,
) -> Result<Mesh, CadError> {
    let mut result = Mesh::default();
    for feature in &part.features {
        match feature {
            Feature::Extrude { sketch, depth, .. } => {
                let sketch = model
                    .sketches
                    .values()
                    .find(|candidate| candidate.id == *sketch)
                    .ok_or(CadError::MissingSketch)?;
                result.append(&kernel.extrude(sketch, *depth)?);
            }
            Feature::Solid { geometry, .. } => result.append(&kernel.solid(geometry)?),
        }
    }
    if result.triangles.is_empty() {
        Err(CadError::EmptyPart(part.name.clone()))
    } else {
        Ok(result)
    }
}

pub fn build_meshes(model: &SemanticModel) -> Result<BTreeMap<PartId, Mesh>, CadError> {
    model
        .parts
        .values()
        .map(|part| build_part_mesh(model, part, &NativeCadKernel).map(|mesh| (part.id, mesh)))
        .collect()
}

/// Builds one scene mesh from all parts. Part-local meshes remain available through `build_meshes`.
pub fn build_mesh(model: &SemanticModel) -> Result<Mesh, CadError> {
    let mut result = Mesh::default();
    for mesh in build_meshes(model)?.values() {
        result.append(mesh);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use robogen_domain::{Point2, SketchId, SourceSpan};
    use robogen_sketch::{Rectangle, SketchEntity};
    #[test]
    fn rectangle_extrusion_is_a_watertight_box_mesh() -> Result<(), CadError> {
        let sketch = Sketch {
            id: SketchId::from_name("s"),
            name: "s".into(),
            plane: Plane::XY,
            entities: vec![SketchEntity::Rectangle(Rectangle {
                name: "r".into(),
                origin: Point2::default(),
                width: Length::from_millimetres(10.0),
                height: Length::from_millimetres(20.0),
                span: SourceSpan::default(),
            })],
            constraints: vec![],
            source_span: SourceSpan::default(),
        };
        let mesh = NativeCadKernel.extrude(&sketch, Length::from_millimetres(3.0))?;
        assert_eq!((mesh.vertices.len(), mesh.triangles.len()), (8, 12));
        Ok(())
    }
}
