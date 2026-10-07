use faer::{
    Mat, Side,
    linalg::solvers::Solve,
    sparse::{SparseColMat, Triplet},
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DofId(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct MatrixEntry {
    pub row: DofId,
    pub column: DofId,
    pub value: f64,
}

pub struct SymmetricSystem {
    pub lower: Vec<MatrixEntry>,
    pub rhs: Vec<f64>,
}

#[derive(Debug)]
pub struct LinearSolution {
    pub values: Vec<f64>,
    pub relative_residual: f64,
}

#[derive(Debug, Error)]
pub enum LinearError {
    #[error("Invalid sparse system or resource budget exceeded")]
    InvalidSystem,
    #[error("Sparse allocation or assembly failed")]
    Assembly,
    #[error("Stiffness matrix is not positive definite; check supports and connectivity")]
    NotPositiveDefinite,
    #[error("Linear solve produced a non-finite value or excessive residual")]
    Residual,
}

pub trait SparseLinearSolver: Send + Sync {
    fn backend_id(&self) -> &'static str;

    fn solve(
        &self,
        system: &SymmetricSystem,
        tolerance: f64,
    ) -> Result<LinearSolution, LinearError>;
}

pub struct FaerSolver;

impl SparseLinearSolver for FaerSolver {
    fn backend_id(&self) -> &'static str {
        "faer-0.22.6-sparse-llt"
    }

    fn solve(
        &self,
        system: &SymmetricSystem,
        tolerance: f64,
    ) -> Result<LinearSolution, LinearError> {
        let size = system.rhs.len();
        if size == 0
            || size > 8_000
            || system.lower.len() > 4_000_000
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || tolerance >= 1.0
            || system.rhs.iter().any(|value| !value.is_finite())
            || system.lower.iter().any(|entry| {
                entry.row.0 >= size || entry.column.0 > entry.row.0 || !entry.value.is_finite()
            })
        {
            return Err(LinearError::InvalidSystem);
        }
        let entries: Vec<_> = system
            .lower
            .iter()
            .map(|entry| Triplet::new(entry.row.0, entry.column.0, entry.value))
            .collect();
        let matrix = SparseColMat::<usize, f64>::try_new_from_triplets(size, size, &entries)
            .map_err(|_| LinearError::Assembly)?;
        let factor = matrix
            .sp_cholesky(Side::Lower)
            .map_err(|_| LinearError::NotPositiveDefinite)?;
        let rhs = Mat::from_fn(size, 1, |row, _| system.rhs[row]);
        let solution = factor.solve(&rhs);
        let values: Vec<_> = (0..size).map(|row| solution[(row, 0)]).collect();
        let mut residual = system.rhs.iter().map(|value| -value).collect::<Vec<_>>();
        for entry in &system.lower {
            residual[entry.row.0] += entry.value * values[entry.column.0];
            if entry.row != entry.column {
                residual[entry.column.0] += entry.value * values[entry.row.0];
            }
        }
        let norm = |items: &[f64]| items.iter().fold(0.0_f64, |norm, value| norm.hypot(*value));
        let rhs_norm = norm(&system.rhs);
        let relative_residual = norm(&residual) / if rhs_norm > 0.0 { rhs_norm } else { 1.0 };
        if values.iter().any(|value| !value.is_finite())
            || !relative_residual.is_finite()
            || relative_residual > tolerance
        {
            return Err(LinearError::Residual);
        }
        Ok(LinearSolution {
            values,
            relative_residual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_known_solution_and_duplicate_entries() -> Result<(), LinearError> {
        let system = SymmetricSystem {
            lower: [(0, 0, 2.0), (0, 0, 2.0), (1, 0, 1.0), (1, 1, 3.0)]
                .map(|(row, column, value)| MatrixEntry {
                    row: DofId(row),
                    column: DofId(column),
                    value,
                })
                .to_vec(),
            rhs: vec![6.0, 7.0],
        };
        let result = FaerSolver.solve(&system, 1e-12)?;
        assert!((result.values[0] - 1.0).abs() < 1e-12);
        assert!((result.values[1] - 2.0).abs() < 1e-12);
        assert!(result.relative_residual < 1e-12);
        Ok(())
    }

    #[test]
    fn sparse_rejects_singular_and_invalid_systems() {
        let mut system = SymmetricSystem {
            lower: vec![],
            rhs: vec![1.0],
        };
        assert!(matches!(
            FaerSolver.solve(&system, 1e-10),
            Err(LinearError::NotPositiveDefinite)
        ));
        system.lower.push(MatrixEntry {
            row: DofId(0),
            column: DofId(0),
            value: -1.0,
        });
        assert!(matches!(
            FaerSolver.solve(&system, 1e-10),
            Err(LinearError::NotPositiveDefinite)
        ));
        system.lower[0].row = DofId(1);
        assert!(matches!(
            FaerSolver.solve(&system, 1e-10),
            Err(LinearError::InvalidSystem)
        ));
        system.lower[0].row = DofId(0);
        system.rhs[0] = f64::NAN;
        assert!(matches!(
            FaerSolver.solve(&system, 1e-10),
            Err(LinearError::InvalidSystem)
        ));
    }
}
