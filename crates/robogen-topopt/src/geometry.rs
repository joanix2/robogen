//! Conservative cell classification for discretisation, not a CAD kernel.
use robogen_domain::SourceSpan;
use robogen_ir::{SolidGeometry, SolidOperation};

use crate::execution::{ExecutionError, failure};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
impl Bounds {
    pub fn dimensions(self) -> [f64; 3] {
        std::array::from_fn(|a| self.max[a] - self.min[a])
    }
    pub fn contains(self, other: Self) -> bool {
        (0..3).all(|a| self.min[a] <= other.min[a] && self.max[a] >= other.max[a])
    }
    fn transformed(self, matrix: [[f64; 3]; 3]) -> Self {
        let mut result = Self {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        };
        for corner in 0..8 {
            let p = std::array::from_fn(|a| {
                if corner & (1 << a) == 0 {
                    self.min[a]
                } else {
                    self.max[a]
                }
            });
            let q = rotate(matrix, p);
            for (a, value) in q.iter().enumerate() {
                result.min[a] = result.min[a].min(*value);
                result.max[a] = result.max[a].max(*value);
            }
        }
        result
    }
    fn translated(self, offset: [f64; 3]) -> Self {
        Self {
            min: std::array::from_fn(|a| self.min[a] + offset[a]),
            max: std::array::from_fn(|a| self.max[a] + offset[a]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relation {
    Inside,
    Outside,
    Boundary,
}

pub(crate) enum Region {
    Box(Bounds),
    Cylinder {
        radius: f64,
        height: f64,
    },
    Translate {
        offset: [f64; 3],
        shape: Box<Self>,
    },
    Rotate {
        matrix: [[f64; 3]; 3],
        shape: Box<Self>,
    },
    Union(Vec<Self>),
    Difference {
        base: Box<Self>,
        tools: Vec<Self>,
    },
}

impl Region {
    pub fn compile(
        geometry: &SolidGeometry,
        budget: &mut usize,
        depth: usize,
    ) -> Result<Self, ExecutionError> {
        if *budget == 0 || depth > 64 {
            return Err(failure(
                "E341",
                geometry.span,
                "Geometry exceeds discretisation resource limits",
            ));
        }
        *budget -= 1;
        let bad = || {
            failure(
                "E341",
                geometry.span,
                "Invalid or unsupported geometry in topology region",
            )
        };
        let child = |g: &SolidGeometry, budget: &mut usize| Self::compile(g, budget, depth + 1);
        Ok(match &geometry.operation {
            SolidOperation::Box { size } => {
                let max = size.map(|l| l.metres());
                if max.iter().any(|v| !v.is_finite() || *v <= 0.0) {
                    return Err(bad());
                }
                Self::Box(Bounds { min: [0.0; 3], max })
            }
            SolidOperation::Cylinder { radius, height } => {
                if !radius.metres().is_finite()
                    || radius.metres() <= 0.0
                    || !height.metres().is_finite()
                    || height.metres() <= 0.0
                {
                    return Err(bad());
                }
                Self::Cylinder {
                    radius: radius.metres(),
                    height: height.metres(),
                }
            }
            SolidOperation::Color { shape, .. } => child(shape, budget)?,
            SolidOperation::Translate { offset, shape } => {
                let offset = offset.map(|l| l.metres());
                if offset.iter().any(|v| !v.is_finite()) {
                    return Err(bad());
                }
                Self::Translate {
                    offset,
                    shape: Box::new(child(shape, budget)?),
                }
            }
            SolidOperation::Rotate { angles, shape } => {
                if angles.iter().any(|a| !a.radians().is_finite()) {
                    return Err(bad());
                }
                let [(sx, cx), (sy, cy), (sz, cz)] = angles.map(|a| a.radians().sin_cos());
                let matrix = [
                    [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
                    [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
                    [-sy, cy * sx, cy * cx],
                ];
                Self::Rotate {
                    matrix,
                    shape: Box::new(child(shape, budget)?),
                }
            }
            SolidOperation::Union { shapes } | SolidOperation::Compound { shapes } => {
                if shapes.is_empty() {
                    return Err(bad());
                }
                Self::Union(
                    shapes
                        .iter()
                        .map(|g| child(g, budget))
                        .collect::<Result<_, _>>()?,
                )
            }
            SolidOperation::Difference { base, tools } => Self::Difference {
                base: Box::new(child(base, budget)?),
                tools: tools
                    .iter()
                    .map(|g| child(g, budget))
                    .collect::<Result<_, _>>()?,
            },
            SolidOperation::Topology { .. } => {
                return Err(failure(
                    "E340",
                    geometry.span,
                    "Nested topology must be resolved before discretisation",
                ));
            }
        })
    }
    /// Combine translations before classification instead of repeatedly
    /// subtracting large world coordinates from small cell coordinates.
    pub fn shifted(self, delta: [f64; 3]) -> Self {
        match self {
            Self::Translate { offset, shape } => {
                shape.shifted(std::array::from_fn(|a| offset[a] + delta[a]))
            }
            Self::Union(shapes) => {
                Self::Union(shapes.into_iter().map(|s| s.shifted(delta)).collect())
            }
            Self::Difference { base, tools } => Self::Difference {
                base: Box::new(base.shifted(delta)),
                tools: tools.into_iter().map(|s| s.shifted(delta)).collect(),
            },
            shape if delta == [0.0; 3] => shape,
            shape => Self::Translate {
                offset: delta,
                shape: Box::new(shape),
            },
        }
    }
    pub fn into_grid_frame(
        mut self,
        span: SourceSpan,
    ) -> Result<(Self, [f64; 3], Bounds), ExecutionError> {
        self.checked_bounds(span)?;
        let mut translation = [0.0; 3];
        while let Self::Translate { offset, shape } = self {
            for a in 0..3 {
                translation[a] += offset[a];
            }
            self = *shape;
        }
        let bounds = self.checked_bounds(span)?;
        let origin = std::array::from_fn(|a| translation[a] + bounds.min[a]);
        if origin.iter().any(|v| !v.is_finite()) {
            return Err(failure("E341", span, "Non-finite grid origin"));
        }
        Ok((
            self.shifted(bounds.min.map(|v| -v)),
            origin,
            Bounds {
                min: [0.0; 3],
                max: bounds.dimensions(),
            },
        ))
    }

    pub fn bounds(&self) -> Bounds {
        match self {
            Self::Box(bounds) => *bounds,
            Self::Cylinder { radius, height } => Bounds {
                min: [-*radius, -*radius, 0.0],
                max: [*radius, *radius, *height],
            },
            Self::Translate { offset, shape } => shape.bounds().translated(*offset),
            Self::Rotate { matrix, shape } => shape.bounds().transformed(*matrix),
            Self::Union(shapes) => {
                let mut b = Bounds {
                    min: [f64::INFINITY; 3],
                    max: [f64::NEG_INFINITY; 3],
                };
                for s in shapes {
                    let c = s.bounds();
                    for a in 0..3 {
                        b.min[a] = b.min[a].min(c.min[a]);
                        b.max[a] = b.max[a].max(c.max[a]);
                    }
                }
                b
            }
            Self::Difference { base, .. } => base.bounds(),
        }
    }
    pub fn classify(&self, cell: Bounds) -> Relation {
        use Relation::*;
        match self {
            Self::Box(b) => {
                if (0..3).any(|a| cell.max[a] <= b.min[a] || cell.min[a] >= b.max[a]) {
                    Outside
                } else if b.contains(cell) {
                    Inside
                } else {
                    Boundary
                }
            }
            Self::Cylinder { radius, height } => {
                if cell.max[2] <= 0.0 || cell.min[2] >= *height {
                    return Outside;
                }
                let nearest: [f64; 2] = std::array::from_fn(|a| {
                    if cell.min[a] > 0.0 {
                        cell.min[a]
                    } else if cell.max[a] < 0.0 {
                        cell.max[a]
                    } else {
                        0.0
                    }
                });
                let farthest: [f64; 2] =
                    std::array::from_fn(|a| cell.min[a].abs().max(cell.max[a].abs()));
                if nearest[0].hypot(nearest[1]) >= *radius {
                    Outside
                } else if farthest[0].hypot(farthest[1]) <= *radius
                    && cell.min[2] >= 0.0
                    && cell.max[2] <= *height
                {
                    Inside
                } else {
                    Boundary
                }
            }
            Self::Translate { offset, shape } => {
                shape.classify(cell.translated(offset.map(|v| -v)))
            }
            Self::Rotate { matrix, shape } => {
                shape.classify(cell.transformed(std::array::from_fn(|a| {
                    std::array::from_fn(|b| matrix[b][a])
                })))
            }
            Self::Union(shapes) => {
                let mut uncertain = false;
                for shape in shapes {
                    match shape.classify(cell) {
                        Inside => return Inside,
                        Boundary => uncertain = true,
                        Outside => {}
                    }
                }
                if uncertain { Boundary } else { Outside }
            }
            Self::Difference { base, tools } => {
                let base = base.classify(cell);
                if base == Outside {
                    return Outside;
                }
                let mut uncertain = base == Boundary;
                for tool in tools {
                    match tool.classify(cell) {
                        Inside => return Outside,
                        Boundary => uncertain = true,
                        Outside => {}
                    }
                }
                if uncertain { Boundary } else { Inside }
            }
        }
    }
    pub fn checked_bounds(&self, span: SourceSpan) -> Result<Bounds, ExecutionError> {
        let b = self.bounds();
        if b.min.iter().chain(&b.max).any(|v| !v.is_finite())
            || b.dimensions().iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(failure(
                "E341",
                span,
                "Empty, non-finite or degenerate region bounds",
            ));
        }
        Ok(b)
    }
}
fn rotate(matrix: [[f64; 3]; 3], point: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|a| (0..3).map(|b| matrix[a][b] * point[b]).sum())
}

#[cfg(test)]
mod tests {
    use super::*;
    use robogen_domain::{Angle, Length};
    #[test]
    fn thin_keepout_and_csg_are_classified_by_cell_extent() {
        let cell = Bounds {
            min: [0.0; 3],
            max: [1.0; 3],
        };
        let hole = Region::Translate {
            offset: [0.1, 0.1, 0.0],
            shape: Box::new(Region::Cylinder {
                radius: 0.01,
                height: 1.0,
            }),
        };
        assert_eq!(hole.classify(cell), Relation::Boundary);
        let body = Region::Difference {
            base: Box::new(Region::Box(cell)),
            tools: vec![hole],
        };
        assert_eq!(body.classify(cell), Relation::Boundary);
        let safe = Bounds {
            min: [0.2; 3],
            max: [0.8; 3],
        };
        assert_eq!(body.classify(safe), Relation::Inside);
        let outside = Bounds {
            min: [2.0; 3],
            max: [3.0; 3],
        };
        assert_eq!(body.classify(outside), Relation::Outside);
        let union = Region::Union(vec![body, Region::Box(cell)]);
        assert_eq!(union.classify(cell), Relation::Inside);
    }
    #[test]
    fn rotated_translated_geometry_keeps_conservative_bounds() -> Result<(), ExecutionError> {
        let shape = SolidGeometry {
            span: SourceSpan::new(1, 2),
            operation: SolidOperation::Translate {
                offset: [2.0, 3.0, 4.0].map(Length::from_metres),
                shape: Box::new(SolidGeometry {
                    span: SourceSpan::default(),
                    operation: SolidOperation::Rotate {
                        angles: [0.0, 0.0, 90.0].map(Angle::from_degrees),
                        shape: Box::new(SolidGeometry {
                            span: SourceSpan::default(),
                            operation: SolidOperation::Box {
                                size: [1.0, 2.0, 3.0].map(Length::from_metres),
                            },
                        }),
                    },
                }),
            },
        };
        let region = Region::compile(&shape, &mut 100, 0)?;
        let bounds = region.checked_bounds(shape.span)?;
        for (a, expected) in [0.0, 3.0, 4.0].iter().enumerate() {
            assert!((bounds.min[a] - expected).abs() < 1e-12);
        }
        assert_eq!(
            region.classify(Bounds {
                min: [0.2, 3.2, 4.2],
                max: [1.8, 3.8, 6.8]
            }),
            Relation::Inside
        );
        assert_eq!(
            region.classify(Bounds {
                min: [2.2, 3.2, 4.2],
                max: [2.8, 3.8, 6.8]
            }),
            Relation::Outside
        );
        assert!(Region::compile(&shape, &mut 1, 0).is_err());
        Ok(())
    }
}
