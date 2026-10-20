//! Backend-neutral mass diagnostics and an explicit unavailable computation port.

mod diagnostic;
mod unavailable;

pub use diagnostic::{BinaryMassDiagnostic, DensityConnectivity, MassDiagnostic, MassIteration};
pub use unavailable::diagnose_mass;

/// No qualified minimum-mass engine is currently connected.
pub const MASS_BACKEND_AVAILABLE: bool = false;
pub const MASS_BACKEND: &str = "Minimum-mass backend unavailable";

#[cfg(test)]
mod tests;
