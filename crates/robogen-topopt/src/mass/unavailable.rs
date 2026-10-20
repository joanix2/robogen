//! Minimum-mass execution port; no qualified engine is connected.

use super::{MASS_BACKEND, MassDiagnostic};
use crate::{
    TopologyRequest,
    execution::{ExecutionError, ExecutionSettings, failure},
};

/// Reject execution without fabricating progress or scientific output.
///
/// Keep the source span and give cancellation precedence over availability.
pub fn diagnose_mass(
    request: &TopologyRequest,
    _settings: &ExecutionSettings,
    cancelled: &dyn Fn() -> bool,
    _progress: &mut dyn FnMut(&MassDiagnostic),
) -> Result<MassDiagnostic, ExecutionError> {
    let span = request.specification.span;
    if cancelled() {
        return Err(failure("E345", span, "Mass calculation cancelled"));
    }
    Err(failure("E348", span, MASS_BACKEND))
}
