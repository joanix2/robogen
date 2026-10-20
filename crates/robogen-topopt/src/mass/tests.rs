use super::*;
use crate::{execution::ComputeBackend, simp::CellRegion};
use robogen_domain::{Length, Mass, SourceSpan};
use serde_json::json;

fn diagnostic() -> Result<MassDiagnostic, serde_json::Error> {
    Ok(MassDiagnostic {
        compute_backend: ComputeBackend::Cpu,
        id: serde_json::from_value(json!("00000000-0000-0000-0000-000000000001"))?,
        name: "protocol-test".into(),
        span: SourceSpan::default(),
        document_revision: 7,
        origin: [Length::from_metres(0.0); 3],
        dimensions: [0.02, 0.01, 0.01].map(Length::from_metres),
        cells: [2, 1, 1],
        regions: vec![CellRegion::Preserve, CellRegion::Design],
        densities: Vec::new(),
        reference_densities: Vec::new(),
        mass: Mass::from_kilograms(0.0),
        reference_mass: Mass::from_kilograms(0.0),
        cases: vec![(
            serde_json::from_value(json!("00000000-0000-0000-0000-000000000002"))?,
            Length::from_metres(0.0),
            Length::from_metres(1e-4),
        )],
        history: Vec::new(),
        finished: false,
        feasible: false,
        converged: false,
        kkt_residual: None,
        binary: None,
        reconstruction_error: None,
    })
}

#[test]
fn threshold_connectivity_reports_broken_attachments_without_repairing_density()
-> Result<(), Box<dyn std::error::Error>> {
    let mut field = diagnostic()?;
    field.cells = [5, 1, 1];
    field.regions = vec![
        CellRegion::Preserve,
        CellRegion::Design,
        CellRegion::Design,
        CellRegion::Design,
        CellRegion::Preserve,
    ];
    field.densities = vec![1.0, 0.8, 0.2, 0.6, 1.0];
    field.reference_densities = vec![1.0; 5];
    let original = field.densities.clone();
    assert_eq!(field.connectivity(0.35, false)?.preserved_components, 2);
    assert_eq!(field.connection_threshold(false)?, Some(0.2));
    assert_eq!(field.connectivity(0.2, false)?.components, 1);
    assert_eq!(field.connection_threshold(true)?, Some(1.0));
    assert_eq!(field.densities, original);
    field.regions[2] = CellRegion::Void;
    field.densities[2] = 0.0;
    assert_eq!(field.connection_threshold(false)?, None);
    assert_eq!(field.connectivity(0.0, false)?.preserved_components, 2);
    field.cells = [2, 2, 1];
    field.regions = vec![
        CellRegion::Preserve,
        CellRegion::Void,
        CellRegion::Void,
        CellRegion::Preserve,
    ];
    field.densities = vec![1.0, 0.0, 0.0, 1.0];
    assert_eq!(field.connection_threshold(false)?, None); // edge contact is not a path
    assert!(field.connectivity(f64::NAN, false).is_err());
    field.cells = [usize::MAX, 2, 1];
    assert!(field.connectivity(0.5, false).is_err());
    Ok(())
}
