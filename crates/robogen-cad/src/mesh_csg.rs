use crate::{CadError, Mesh, Triangle, Vertex};
use csgrs::csg::CSG;
use robogen_domain::SourceSpan;
use robogen_ir::{SolidGeometry, SolidOperation};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 128;
const MAX_PRIMITIVE_POLYGONS: usize = 512;
const MAX_POLYGONS: usize = 2_048;
const MAX_VERTICES: usize = 8_192;
const MAX_BOOLEAN_INPUT: usize = 256;
const MAX_BOOLEAN_PAIR_COST: usize = 16_384;
const MAX_BOOLEAN_TOTAL_COST: usize = 65_536;
const CYLINDER_SEGMENTS: usize = 32;
const MIN_SIZE_MM: f64 = 0.001;
const MAX_COORDINATE_MM: f64 = 1_000_000.0;

type Solid = CSG<SourceSpan>;

fn invalid(span: SourceSpan, reason: &str) -> CadError {
    CadError::InvalidSolid { span, reason: reason.into() }
}

fn limit(span: SourceSpan, limit: &'static str) -> CadError {
    CadError::SolidLimit { span, limit }
}

fn guarded<T>(span: SourceSpan, operation: impl FnOnce() -> T) -> Result<T, CadError> {
    catch_unwind(AssertUnwindSafe(operation))
        .map_err(|_| CadError::SolidBackendFailure { span })
}

pub(super) fn build(geometry: &SolidGeometry) -> Result<Mesh, CadError> {
    validate(geometry)?;
    let solid = evaluate(geometry, &mut 0)?;
    guarded(geometry.span, || to_mesh(&solid, geometry.span))?
}

fn validate(geometry: &SolidGeometry) -> Result<(), CadError> {
    let mut pending = vec![(geometry, 1)];
    let mut nodes = 0;
    let mut polygons = 0;
    while let Some((node, depth)) = pending.pop() {
        nodes += 1;
        if nodes > MAX_NODES || pending.len() > MAX_NODES - nodes {
            return Err(limit(node.span, "128 solid nodes"));
        }
        if depth > MAX_DEPTH {
            return Err(limit(node.span, "32 solid levels"));
        }
        let dimension = |value: f64| {
            if !value.is_finite() || !(MIN_SIZE_MM..=MAX_COORDINATE_MM).contains(&value) {
                Err(invalid(node.span, "dimensions must be finite and between 0.001 and 1000000 mm"))
            } else {
                Ok(())
            }
        };
        match &node.operation {
            SolidOperation::Box { size } => {
                for value in size { dimension(value.metres() * 1_000.0)?; }
                polygons += 6;
            }
            SolidOperation::Cylinder { radius, height } => {
                dimension(radius.metres() * 1_000.0)?;
                dimension(height.metres() * 1_000.0)?;
                polygons += CYLINDER_SEGMENTS * 3;
            }
            SolidOperation::Translate { offset, shape } => {
                if offset.iter().any(|value| !value.metres().is_finite()
                    || value.metres().abs() * 1_000.0 > MAX_COORDINATE_MM) {
                    return Err(invalid(node.span, "translation must be finite and within 1000000 mm"));
                }
                pending.push((shape, depth + 1));
            }
            SolidOperation::Union { shapes } => {
                if shapes.is_empty() { return Err(invalid(node.span, "union needs an operand")); }
                if shapes.len() > MAX_NODES { return Err(limit(node.span, "128 solid nodes")); }
                pending.extend(shapes.iter().map(|shape| (shape, depth + 1)));
            }
            SolidOperation::Difference { base, tools } => {
                if tools.is_empty() { return Err(invalid(node.span, "difference needs a tool")); }
                if tools.len() >= MAX_NODES { return Err(limit(node.span, "128 solid nodes")); }
                pending.push((base, depth + 1));
                pending.extend(tools.iter().map(|shape| (shape, depth + 1)));
            }
        }
        if polygons > MAX_PRIMITIVE_POLYGONS {
            return Err(limit(node.span, "512 primitive polygons"));
        }
    }
    Ok(())
}

fn check_solid(solid: &Solid, span: SourceSpan) -> Result<(), CadError> {
    if solid.polygons.len() > MAX_POLYGONS {
        return Err(limit(span, "2048 intermediate polygons"));
    }
    let mut vertices = 0;
    for polygon in &solid.polygons {
        vertices += polygon.vertices.len();
        if vertices > MAX_VERTICES { return Err(limit(span, "8192 polygon vertices")); }
        if polygon.vertices.len() < 3 || polygon.vertices.iter().any(|vertex|
            vertex.pos.iter().any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)) {
            return Err(invalid(span, "backend produced invalid or out-of-range coordinates"));
        }
    }
    Ok(())
}

fn combine(left: Solid, right: Solid, subtract: bool, span: SourceSpan, cost: &mut usize) -> Result<Solid, CadError> {
    let pair_cost = left.polygons.len() * right.polygons.len();
    *cost += pair_cost;
    if left.polygons.len() + right.polygons.len() > MAX_BOOLEAN_INPUT
        || pair_cost > MAX_BOOLEAN_PAIR_COST || *cost > MAX_BOOLEAN_TOTAL_COST {
        return Err(limit(span, "Boolean polygon work"));
    }
    if left.polygons.is_empty() {
        return Ok(if subtract { left } else { right });
    }
    if right.polygons.is_empty() { return Ok(left); }
    let result = guarded(span, || if subtract { left.difference(&right) } else { left.union(&right) })?;
    check_solid(&result, span)?;
    Ok(result)
}

fn evaluate(node: &SolidGeometry, cost: &mut usize) -> Result<Solid, CadError> {
    let result = match &node.operation {
        SolidOperation::Box { size } => {
            let [width, length, height] = size.map(|value| value.metres() * 1_000.0);
            guarded(node.span, || Solid::cube(width, length, height, Some(node.span)))?
        }
        SolidOperation::Cylinder { radius, height } => guarded(node.span, ||
            Solid::cylinder(radius.metres() * 1_000.0, height.metres() * 1_000.0, CYLINDER_SEGMENTS, Some(node.span)))?,
        SolidOperation::Translate { offset, shape } => {
            let shape = evaluate(shape, cost)?;
            guarded(node.span, || shape.translate(offset.map(|value| value.metres() * 1_000.0).into()))?
        }
        SolidOperation::Union { shapes } => {
            let mut result = Solid::new();
            for shape in shapes {
                result = combine(result, evaluate(shape, cost)?, false, node.span, cost)?;
            }
            result
        }
        SolidOperation::Difference { base, tools } => {
            let mut result = evaluate(base, cost)?;
            for tool in tools {
                result = combine(result, evaluate(tool, cost)?, true, node.span, cost)?;
            }
            result
        }
    };
    check_solid(&result, node.span)?;
    Ok(result)
}

fn to_mesh(solid: &Solid, span: SourceSpan) -> Result<Mesh, CadError> {
    if solid.polygons.is_empty() { return Err(invalid(span, "solid result is empty")); }
    let mut mesh = Mesh::default();
    let mut indices = BTreeMap::new();
    for polygon in &solid.polygons {
        for triangle in polygon.triangulate() {
            let mut face = [0; 3];
            for (slot, vertex) in triangle.iter().enumerate() {
                let position = [vertex.pos.x, vertex.pos.y, vertex.pos.z].map(|value| (value / 1_000.0) as f32);
                let key = position.map(|value| if value == 0.0 { 0 } else { value.to_bits() });
                face[slot] = *indices.entry(key).or_insert_with(|| {
                    let index = mesh.vertices.len() as u32;
                    mesh.vertices.push(Vertex { position });
                    index
                });
            }
            if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
                return Err(invalid(polygon.metadata.unwrap_or(span), "triangle collapsed at mesh precision"));
            }
            mesh.triangles.push(Triangle { indices: face });
        }
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_box_volume() {
        let solid = Solid::cube(10.0, 10.0, 10.0, None);
        let volume: f64 = solid.polygons.iter().flat_map(|polygon| polygon.triangulate())
            .map(|triangle| triangle[0].pos.coords.dot(&triangle[1].pos.coords.cross(&triangle[2].pos.coords)) / 6.0).sum();
        assert!((volume - 1_000.0).abs() < 0.001, "{volume}");
    }

    #[test]
    fn backend_translated_planes() {
        let solid = Solid::cube(10.0, 10.0, 10.0, None).translate([5.0, 0.0, 0.0].into());
        for polygon in &solid.polygons {
            let edge_first = polygon.vertices[1].pos - polygon.vertices[0].pos;
            let edge_second = polygon.vertices[2].pos - polygon.vertices[0].pos;
            let normal = edge_first.cross(&edge_second).normalize();
            assert!(normal.dot(&polygon.plane.normal) > 0.999, "{normal:?} {:?}", polygon.plane);
        }
    }
}