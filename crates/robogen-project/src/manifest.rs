//! Project manifest contracts and filesystem loading.

use robogen_domain::Diagnostic;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    #[serde(default = "manifest_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub source: PathBuf,
}

const fn manifest_schema_version() -> u32 {
    1
}

#[derive(Debug, Error)]
pub enum ProjectIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] toml::de::Error),
    #[error("project source contains errors")]
    Diagnostics(Vec<Diagnostic>),
    #[error("project source path must stay inside the project: {0}")]
    UnsafeSource(PathBuf),
}

pub fn load_manifest(path: &Path) -> Result<(ProjectManifest, String), ProjectIoError> {
    let manifest: ProjectManifest = toml::from_str(&fs::read_to_string(path)?)?;
    if manifest.source.is_absolute()
        || manifest
            .source
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProjectIoError::UnsafeSource(manifest.source));
    }
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let source = fs::read_to_string(root.join(&manifest.source))?;
    Ok((manifest, source))
}
