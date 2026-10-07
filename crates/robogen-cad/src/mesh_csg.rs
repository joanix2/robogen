use crate::{CadError, Mesh, Triangle, Vertex};
use csgrs::csg::CSG;
use robogen_domain::{Angle, SourceSpan};
use robogen_ir::{SolidGeometry, SolidOperation};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 128;
const MAX_PRIMITIVE_POLYGONS: usize = 1_024;
const MAX_POLYGONS: usize = 4_096;
const MAX_VERTICES: usize = 16_384;
const MAX_BOOLEAN_INPUT: usize = 2_048;
const MAX_BOOLEAN_PAIR_COST: usize = 524_288;
const MAX_BOOLEAN_TOTAL_COST: usize = 2_097_152;
const CYLINDER_SEGMENTS: usize = 32;
const MIN_SIZE_MM: f64 = 0.001;
const MAX_COORDINATE_MM: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug)]
struct Surface {
    span: SourceSpan,
    color: Option<[u8; 3]>,
}

type Solid = CSG<Surface>;

fn invalid(span: SourceSpan, reason: &str) -> CadError {
    CadError::InvalidSolid {
        span,
        reason: reason.into(),
    }
}

fn limit(span: SourceSpan, limit: &'static str) -> CadError {
    CadError::SolidLimit { span, limit }
}

fn guarded<T>(span: SourceSpan, operation: impl FnOnce() -> T) -> Result<T, CadError> {
    catch_unwind(AssertUnwindSafe(operation)).map_err(|_| CadError::SolidBackendFailure { span })
}

fn translate(solid: &Solid, offset: [f64; 3]) -> Solid {
    Solid::from_polygons(
        &solid
            .polygons
            .iter()
            .map(|polygon| polygon.translate(offset.into()))
            .collect::<Vec<_>>(),
    )
}

// Adapter-private arithmetic; lengths are metres for compounds, mm for CSG.
#[derive(Clone, Copy)]
enum Transform {
    Translate([f64; 3]),
    Rotate([[f64; 3]; 3]),
}

fn rotate_point(matrix: [[f64; 3]; 3], point: [f64; 3]) -> [f64; 3] {
    matrix.map(|row| row.into_iter().zip(point).map(|(a, b)| a * b).sum())
}

fn rotation(angles: [Angle; 3], span: SourceSpan) -> Result<[[f64; 3]; 3], CadError> {
    if angles.iter().any(|angle| !angle.radians().is_finite()) {
        return Err(invalid(span, "rotation angles must be finite"));
    }
    let [(sx, cx), (sy, cy), (sz, cz)] = angles.map(|angle| angle.radians().sin_cos());
    Ok([
        [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
        [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
        [-sy, cy * sx, cy * cx],
    ])
}

fn rotate_solid(solid: &Solid, matrix: [[f64; 3]; 3]) -> Solid {
    let mut polygons = solid.polygons.clone();
    for polygon in &mut polygons {
        for vertex in &mut polygon.vertices {
            vertex.pos = rotate_point(matrix, [vertex.pos.x, vertex.pos.y, vertex.pos.z]).into();
            vertex.normal =
                rotate_point(matrix, [vertex.normal.x, vertex.normal.y, vertex.normal.z]).into();
        }
        let normal = polygon.plane.normal;
        polygon.plane.normal = rotate_point(matrix, [normal.x, normal.y, normal.z]).into();
        // A rotation about the origin does not change the signed plane distance.
    }
    Solid::from_polygons(&polygons)
}

pub(super) fn build(geometry: &SolidGeometry) -> Result<Mesh, CadError> {
    build_cancellable(geometry, &|| false)
}

pub(super) fn build_cancellable(
    geometry: &SolidGeometry,
    cancelled: &dyn Fn() -> bool,
) -> Result<Mesh, CadError> {
    if wraps_compound(geometry) {
        return build_compound(geometry, cancelled);
    }
    if cancelled() {
        return Err(CadError::Cancelled);
    }
    validate(geometry)?;
    let solid = evaluate(geometry, &mut 0, cancelled)?;
    guarded(geometry.span, || to_mesh(&solid, geometry.span, cancelled))?
}

fn wraps_compound(mut geometry: &SolidGeometry) -> bool {
    for _ in 0..=MAX_DEPTH {
        match &geometry.operation {
            SolidOperation::Compound { .. } => return true,
            SolidOperation::Color { shape, .. }
            | SolidOperation::Translate { shape, .. }
            | SolidOperation::Rotate { shape, .. } => geometry = shape,
            _ => return false,
        }
    }
    false
}

fn build_compound(
    geometry: &SolidGeometry,
    cancelled: &dyn Fn() -> bool,
) -> Result<Mesh, CadError> {
    let mut pending = vec![(geometry, 1, Vec::<Transform>::new(), None)];
    let mut result = Mesh::default();
    let mut bodies = 0;
    let mut visited = 0;
    while let Some((node, depth, transforms, color)) = pending.pop() {
        if cancelled() {
            return Err(CadError::Cancelled);
        }
        visited += 1;
        if depth > MAX_DEPTH || visited > 4096 {
            return Err(limit(node.span, "compound depth or nodes"));
        }
        if !wraps_compound(node) {
            bodies += 1;
            if bodies > 128 {
                return Err(limit(node.span, "128 compound bodies"));
            }
            let mut mesh = build_cancellable(node, cancelled)?;
            if result.vertices.len() + mesh.vertices.len() > 262_144
                || result.triangles.len() + mesh.triangles.len() > 524_288
            {
                return Err(limit(node.span, "compound output size"));
            }
            for vertex in &mut mesh.vertices {
                let mut point = vertex.position.map(f64::from);
                for transform in transforms.iter().rev() {
                    point = match transform {
                        Transform::Translate(offset) => {
                            std::array::from_fn(|axis| point[axis] + offset[axis])
                        }
                        Transform::Rotate(matrix) => rotate_point(*matrix, point),
                    };
                    if point
                        .iter()
                        .any(|value| !value.is_finite() || value.abs() * 1000.0 > MAX_COORDINATE_MM)
                    {
                        return Err(invalid(node.span, "compound transform out of bounds"));
                    }
                }
                vertex.position = point.map(|value| value as f32);
            }
            if let Some(rgb) = color {
                for triangle in &mut mesh.triangles {
                    triangle.color.get_or_insert(rgb);
                }
            }
            result.append(&mesh);
            continue;
        }
        match &node.operation {
            SolidOperation::Compound { shapes } => {
                if shapes.is_empty() {
                    return Err(invalid(node.span, "compound needs an operand"));
                }
                if shapes.len() + pending.len() > 128 {
                    return Err(limit(node.span, "128 compound bodies"));
                }
                pending.extend(
                    shapes
                        .iter()
                        .rev()
                        .map(|shape| (shape, depth + 1, transforms.clone(), color)),
                );
            }
            SolidOperation::Color { rgb, shape } => {
                pending.push((shape, depth + 1, transforms, Some(*rgb)))
            }
            SolidOperation::Translate {
                offset: shift,
                shape,
            } => {
                let offset = shift.map(|value| value.metres());
                if offset
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() * 1000.0 > MAX_COORDINATE_MM)
                {
                    return Err(invalid(node.span, "compound translation out of bounds"));
                }
                let mut transforms = transforms;
                transforms.push(Transform::Translate(offset));
                pending.push((shape, depth + 1, transforms, color));
            }
            SolidOperation::Rotate { angles, shape } => {
                let mut transforms = transforms;
                transforms.push(Transform::Rotate(rotation(*angles, node.span)?));
                pending.push((shape, depth + 1, transforms, color));
            }
            _ => return Err(invalid(node.span, "invalid compound wrapper")),
        }
    }
    Ok(result)
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
                Err(invalid(
                    node.span,
                    "dimensions must be finite and between 0.001 and 1000000 mm",
                ))
            } else {
                Ok(())
            }
        };
        match &node.operation {
            SolidOperation::Compound { .. } => {
                return Err(invalid(node.span, "compound cannot be a Boolean operand"))
            }
            SolidOperation::Color { shape, .. } => pending.push((shape, depth + 1)),
            SolidOperation::Box { size } => {
                for value in size {
                    dimension(value.metres() * 1_000.0)?;
                }
                polygons += 6;
            }
            SolidOperation::Cylinder { radius, height } => {
                dimension(radius.metres() * 1_000.0)?;
                dimension(height.metres() * 1_000.0)?;
                polygons += CYLINDER_SEGMENTS * 3;
            }
            SolidOperation::Translate { offset, shape } => {
                if offset.iter().any(|value| {
                    !value.metres().is_finite()
                        || value.metres().abs() * 1_000.0 > MAX_COORDINATE_MM
                }) {
                    return Err(invalid(
                        node.span,
                        "translation must be finite and within 1000000 mm",
                    ));
                }
                pending.push((shape, depth + 1));
            }
            SolidOperation::Rotate { angles, shape } => {
                rotation(*angles, node.span)?;
                pending.push((shape, depth + 1));
            }
            SolidOperation::Union { shapes } => {
                if shapes.is_empty() {
                    return Err(invalid(node.span, "union needs an operand"));
                }
                if shapes.len() > MAX_NODES {
                    return Err(limit(node.span, "128 solid nodes"));
                }
                pending.extend(shapes.iter().map(|shape| (shape, depth + 1)));
            }
            SolidOperation::Difference { base, tools } => {
                if tools.is_empty() {
                    return Err(invalid(node.span, "difference needs a tool"));
                }
                if tools.len() >= MAX_NODES {
                    return Err(limit(node.span, "128 solid nodes"));
                }
                pending.push((base, depth + 1));
                pending.extend(tools.iter().map(|shape| (shape, depth + 1)));
            }
        }
        if polygons > MAX_PRIMITIVE_POLYGONS {
            return Err(limit(node.span, "1024 primitive polygons"));
        }
    }
    Ok(())
}

fn check_solid(solid: &Solid, span: SourceSpan) -> Result<(), CadError> {
    if solid.polygons.len() > MAX_POLYGONS {
        return Err(limit(span, "4096 intermediate polygons"));
    }
    let mut vertices = 0;
    for polygon in &solid.polygons {
        vertices += polygon.vertices.len();
        if vertices > MAX_VERTICES {
            return Err(limit(span, "16384 polygon vertices"));
        }
        if polygon.vertices.len() < 3
            || polygon.vertices.iter().any(|vertex| {
                vertex
                    .pos
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)
            })
        {
            return Err(invalid(
                span,
                "backend produced invalid or out-of-range coordinates",
            ));
        }
    }
    Ok(())
}

fn combine(
    left: Solid,
    right: Solid,
    subtract: bool,
    span: SourceSpan,
    cost: &mut usize,
) -> Result<Solid, CadError> {
    let pair_cost = left.polygons.len() * right.polygons.len();
    *cost += pair_cost;
    if left.polygons.len() + right.polygons.len() > MAX_BOOLEAN_INPUT
        || pair_cost > MAX_BOOLEAN_PAIR_COST
        || *cost > MAX_BOOLEAN_TOTAL_COST
    {
        return Err(limit(span, "Boolean polygon work"));
    }
    if left.polygons.is_empty() {
        return Ok(if subtract { left } else { right });
    }
    if right.polygons.is_empty() {
        return Ok(left);
    }
    let result = guarded(span, || {
        if subtract {
            left.difference(&right)
        } else {
            left.union(&right)
        }
    })?;
    check_solid(&result, span)?;
    Ok(result)
}

fn evaluate(
    node: &SolidGeometry,
    cost: &mut usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Solid, CadError> {
    if cancelled() {
        return Err(CadError::Cancelled);
    }
    let metadata = Some(Surface {
        span: node.span,
        color: None,
    });
    let result = match &node.operation {
        SolidOperation::Compound { .. } => {
            return Err(invalid(node.span, "compound cannot be a Boolean operand"))
        }
        SolidOperation::Color { rgb, shape } => {
            let mut solid = evaluate(shape, cost, cancelled)?;
            for polygon in &mut solid.polygons {
                let surface = polygon.metadata.get_or_insert(Surface {
                    span: node.span,
                    color: None,
                });
                surface.color.get_or_insert(*rgb);
            }
            solid
        }
        SolidOperation::Box { size } => {
            let [width, length, height] = size.map(|value| value.metres() * 1_000.0);
            guarded(node.span, || Solid::cube(width, length, height, metadata))?
        }
        SolidOperation::Cylinder { radius, height } => guarded(node.span, || {
            Solid::cylinder(
                radius.metres() * 1_000.0,
                height.metres() * 1_000.0,
                CYLINDER_SEGMENTS,
                metadata,
            )
        })?,
        SolidOperation::Translate { offset, shape } => {
            let shape = evaluate(shape, cost, cancelled)?;
            guarded(node.span, || {
                translate(&shape, offset.map(|value| value.metres() * 1_000.0))
            })?
        }
        SolidOperation::Rotate { angles, shape } => {
            let matrix = rotation(*angles, node.span)?;
            let solid = evaluate(shape, cost, cancelled)?;
            guarded(node.span, || rotate_solid(&solid, matrix))?
        }
        SolidOperation::Union { shapes } => {
            let mut result = Solid::new();
            for shape in shapes {
                result = combine(
                    result,
                    evaluate(shape, cost, cancelled)?,
                    false,
                    node.span,
                    cost,
                )?;
            }
            result
        }
        SolidOperation::Difference { base, tools } => {
            let mut result = evaluate(base, cost, cancelled)?;
            for tool in tools {
                result = combine(
                    result,
                    evaluate(tool, cost, cancelled)?,
                    true,
                    node.span,
                    cost,
                )?;
            }
            result
        }
    };
    check_solid(&result, node.span)?;
    Ok(result)
}

fn to_mesh(
    solid: &Solid,
    span: SourceSpan,
    cancelled: &dyn Fn() -> bool,
) -> Result<Mesh, CadError> {
    if solid.polygons.is_empty() {
        return Err(invalid(span, "solid result is empty"));
    }
    let mut mesh = Mesh::default();
    let mut indices = BTreeMap::new();
    let mut positions = Vec::new();
    for polygon in &solid.polygons {
        if cancelled() {
            return Err(CadError::Cancelled);
        }
        for triangle in polygon.triangulate() {
            let mut face = [0; 3];
            for (slot, vertex) in triangle.iter().enumerate() {
                let position =
                    [vertex.pos.x, vertex.pos.y, vertex.pos.z].map(|value| value / 1_000.0);
                let key = position.map(|value| (value * 1e9).round() as i64);
                face[slot] = *indices.entry(key).or_insert_with(|| {
                    let index = mesh.vertices.len() as u32;
                    positions.push(position);
                    mesh.vertices.push(Vertex {
                        position: position.map(|value| value as f32),
                    });
                    index
                });
            }
            if face[0] == face[1] || face[1] == face[2] || face[0] == face[2] {
                return Err(invalid(
                    polygon.metadata.map_or(span, |surface| surface.span),
                    "triangle collapsed at mesh precision",
                ));
            }
            mesh.triangles.push(Triangle {
                indices: face,
                color: polygon.metadata.and_then(|surface| surface.color),
            });
        }
    }
    conform_edges(&mut mesh, &positions, cancelled)?;
    Ok(mesh)
}

fn conform_edges(
    mesh: &mut Mesh,
    positions: &[[f64; 3]],
    cancelled: &dyn Fn() -> bool,
) -> Result<(), CadError> {
    let triangles = std::mem::take(&mut mesh.triangles);
    for triangle in triangles {
        if cancelled() {
            return Err(CadError::Cancelled);
        }
        let mut boundary = Vec::new();
        for edge in 0..3 {
            let start = triangle.indices[edge];
            let end = triangle.indices[(edge + 1) % 3];
            let origin = positions[start as usize];
            let direction =
                std::array::from_fn::<_, 3, _>(|axis| positions[end as usize][axis] - origin[axis]);
            let squared_length: f64 = direction.iter().map(|value| value * value).sum();
            let mut points = vec![(0.0, start)];
            for (index, position) in positions.iter().enumerate() {
                if index == start as usize || index == end as usize {
                    continue;
                }
                let fraction = (0..3)
                    .map(|axis| (position[axis] - origin[axis]) * direction[axis])
                    .sum::<f64>()
                    / squared_length;
                if fraction <= 1e-7 || fraction >= 1.0 - 1e-7 {
                    continue;
                }
                let distance: f64 = (0..3)
                    .map(|axis| {
                        (position[axis] - origin[axis] - fraction * direction[axis]).powi(2)
                    })
                    .sum();
                if distance < 1e-18 {
                    points.push((fraction, index as u32));
                }
            }
            points.sort_by(|left, right| left.0.total_cmp(&right.0));
            boundary.extend(points.into_iter().map(|(_, index)| index));
        }
        if boundary.len() == 3 {
            mesh.triangles.push(triangle);
        } else {
            let center = mesh.vertices.len() as u32;
            mesh.vertices.push(Vertex {
                position: std::array::from_fn(|axis| {
                    (triangle
                        .indices
                        .iter()
                        .map(|index| positions[*index as usize][axis])
                        .sum::<f64>()
                        / 3.0) as f32
                }),
            });
            for edge in 0..boundary.len() {
                mesh.triangles.push(Triangle {
                    indices: [
                        center,
                        boundary[edge],
                        boundary[(edge + 1) % boundary.len()],
                    ],
                    color: triangle.color,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_box_volume() {
        let solid = Solid::cube(10.0, 10.0, 10.0, None);
        let volume: f64 = solid
            .polygons
            .iter()
            .flat_map(|polygon| polygon.triangulate())
            .map(|triangle| {
                triangle[0]
                    .pos
                    .coords
                    .dot(&triangle[1].pos.coords.cross(&triangle[2].pos.coords))
                    / 6.0
            })
            .sum();
        assert!((volume - 1_000.0).abs() < 0.001, "{volume}");
    }

    #[test]
    fn backend_translated_planes() {
        let solid = translate(&Solid::cube(10.0, 10.0, 10.0, None), [5.0, -2.0, 7.0]);
        for polygon in &solid.polygons {
            let edge_first = polygon.vertices[1].pos - polygon.vertices[0].pos;
            let edge_second = polygon.vertices[2].pos - polygon.vertices[0].pos;
            let normal = edge_first.cross(&edge_second).normalize();
            assert!(
                normal.dot(&polygon.plane.normal) > 0.999,
                "{normal:?} {:?}",
                polygon.plane
            );
            for vertex in &polygon.vertices {
                assert!(
                    (polygon.plane.normal.dot(&vertex.pos.coords) - polygon.plane.w).abs() < 1e-8
                );
            }
        }
    }
}
