use std::collections::BTreeMap;
use std::net::IpAddr;
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
    #[serde(default)]
    pub commands: BTreeMap<String, CommandSnippet>,
    #[serde(default)]
    pub ports: Vec<PortMapping>,
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

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct CommandSnippet {
    pub run: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PortMapping {
    pub container: u32,
    #[serde(default)]
    pub host: Option<u32>,
    #[serde(default)]
    pub bind: Option<String>,
}

impl PortMapping {
    pub fn host_port(&self) -> u32 {
        self.host.unwrap_or(self.container)
    }

    pub fn bind_address(&self) -> &str {
        self.bind.as_deref().unwrap_or("127.0.0.1")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    #[error("invalid command name `{name}`: command names must match [A-Za-z_][A-Za-z0-9_]*")]
    InvalidCommandName { name: String },
    #[error("invalid port `{field}` for [[ports]] entry {index}: {value} is outside 1..=65535")]
    InvalidPort {
        index: usize,
        field: &'static str,
        value: u32,
    },
    #[error("invalid bind address `{bind}` for [[ports]] entry {index}: expected an IP address")]
    InvalidBindAddress { index: usize, bind: String },
}

pub fn validate_config(config: &CampfireConfig) -> Result<(), ConfigValidationError> {
    for name in config.commands.keys() {
        if !is_valid_command_name(name) {
            return Err(ConfigValidationError::InvalidCommandName { name: name.clone() });
        }
    }

    for (index, port) in config.ports.iter().enumerate() {
        validate_port(index, "container", port.container)?;

        if let Some(host) = port.host {
            validate_port(index, "host", host)?;
        }

        let bind = port.bind_address();
        if bind.parse::<IpAddr>().is_err() {
            return Err(ConfigValidationError::InvalidBindAddress {
                index,
                bind: bind.to_string(),
            });
        }
    }

    Ok(())
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

fn is_valid_command_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }

    chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn validate_port(
    index: usize,
    field: &'static str,
    value: u32,
) -> Result<(), ConfigValidationError> {
    if !(1..=65535).contains(&value) {
        return Err(ConfigValidationError::InvalidPort {
            index,
            field,
            value,
        });
    }

    Ok(())
}
