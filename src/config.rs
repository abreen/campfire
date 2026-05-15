use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not find Campfire.toml from {start}")]
    NotFound { start: PathBuf },
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct CampfireConfig {
    pub campfire: CampfireSection,
    #[serde(default)]
    pub workspace: WorkspaceSection,
    #[serde(default)]
    pub env: EnvSection,
    #[serde(default)]
    pub files: FilesSection,
    #[serde(default)]
    pub tools: BTreeMap<String, ToolCheck>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct CampfireSection {
    pub image: String,
    #[serde(default = "default_shell")]
    pub shell: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct WorkspaceSection {
    #[serde(default = "default_workspace_path")]
    pub path: String,
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            path: default_workspace_path(),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct EnvSection {
    #[serde(default)]
    pub pass: Vec<String>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct FilesSection {
    #[serde(default)]
    pub readonly: Vec<String>,
    #[serde(default)]
    pub required_readonly: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ToolCheck {
    pub check: String,
    #[serde(default)]
    pub contains: Option<String>,
}

pub fn discover_config(start: impl AsRef<Path>) -> Result<PathBuf, ConfigError> {
    let mut current = start.as_ref();

    loop {
        let candidate = current.join("Campfire.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(ConfigError::NotFound {
                    start: start.as_ref().to_path_buf(),
                });
            }
        }
    }
}

fn default_shell() -> String {
    "/bin/sh".to_string()
}

fn default_workspace_path() -> String {
    "/workspace".to_string()
}
