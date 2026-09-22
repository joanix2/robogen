//! Manufacturing export adapters.

use robogen_cad::Mesh;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("mesh triangle references a missing vertex")]
    InvalidMesh,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("could not persist temporary export: {0}")]
    Persist(#[from] tempfile::PersistError),
}

pub trait MeshExporter {
    fn write(&self, mesh: &Mesh, output: &mut dyn Write) -> Result<(), ExportError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BinaryStlExporter;

impl MeshExporter for BinaryStlExporter {
    fn write(&self, mesh: &Mesh, output: &mut dyn Write) -> Result<(), ExportError> {
        if mesh.vertices.iter().any(|vertex| {
            vertex
                .position
                .iter()
                .any(|coordinate| !coordinate.is_finite())
        }) {
            return Err(ExportError::InvalidMesh);
        }
        output.write_all(&[0u8; 80])?;
        output.write_all(&(mesh.triangles.len() as u32).to_le_bytes())?;
        for triangle in &mesh.triangles {
            let [a, b, c] = triangle
                .indices
                .map(|index| mesh.vertices.get(index as usize).map(|v| v.position));
            let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                return Err(ExportError::InvalidMesh);
            };
            // RoboGen geometry uses SI metres; STL is unitless and manufacturing
            // tools conventionally interpret its coordinates as millimetres.
            let [a, b, c] = [a, b, c].map(to_millimetres);
            for value in normal(a, b, c).into_iter().chain(a).chain(b).chain(c) {
                output.write_all(&value.to_le_bytes())?;
            }
            output.write_all(&0u16.to_le_bytes())?;
        }
        Ok(())
    }
}

fn to_millimetres(point: [f32; 3]) -> [f32; 3] {
    point.map(|coordinate| coordinate * 1_000.0)
}

pub fn write_binary_stl(mesh: &Mesh, mut output: impl Write) -> Result<(), ExportError> {
    BinaryStlExporter.write(mesh, &mut output)
}

/// Writes to a sibling temporary file before replacing the destination.
pub fn export_binary_stl(mesh: &Mesh, path: &Path) -> Result<(), ExportError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    write_binary_stl(mesh, &mut temporary)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

fn normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length == 0.0 {
        [0.0; 3]
    } else {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use robogen_cad::{Triangle, Vertex};
    #[test]
    fn binary_stl_has_expected_record_size() -> Result<(), Box<dyn std::error::Error>> {
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [0., 0., 0.],
                },
                Vertex {
                    position: [1., 0., 0.],
                },
                Vertex {
                    position: [0., 1., 0.],
                },
            ],
            triangles: vec![Triangle { indices: [0, 1, 2] }],
        };
        let mut bytes = Vec::new();
        write_binary_stl(&mesh, &mut bytes)?;
        assert_eq!(bytes.len(), 84 + 50);
        assert_eq!(
            f32::from_le_bytes([bytes[96], bytes[97], bytes[98], bytes[99]]),
            0.0
        );
        assert_eq!(
            f32::from_le_bytes([bytes[108], bytes[109], bytes[110], bytes[111]]),
            1_000.0
        );
        Ok(())
    }
}
