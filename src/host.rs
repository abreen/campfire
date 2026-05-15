use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::CampfireConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostContext {
    pub env: BTreeMap<String, String>,
    pub home: PathBuf,
}

impl HostContext {
    pub fn new(env: BTreeMap<String, String>, home: PathBuf) -> Self {
        Self { env, home }
    }

    pub fn current() -> Self {
        let env = env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .collect();
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));

        Self { env, home }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedHostInputs {
    pub env: BTreeMap<String, String>,
    pub readonly_files: Vec<PathBuf>,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("missing required host inputs")]
pub struct HostInputError {
    pub missing_env: Vec<String>,
    pub missing_files: Vec<PathBuf>,
}

pub fn expand_user_path(raw: &str, home: &Path) -> PathBuf {
    if raw == "~" {
        return home.to_path_buf();
    }

    if let Some(rest) = raw.strip_prefix("~/") {
        return home.join(rest);
    }

    PathBuf::from(raw)
}

pub fn validate_host_inputs(
    config: &CampfireConfig,
    context: &HostContext,
) -> Result<ResolvedHostInputs, HostInputError> {
    let mut env = BTreeMap::new();
    let mut missing_env = Vec::new();

    for name in &config.env.pass {
        if let Some(value) = context.env.get(name) {
            env.insert(name.clone(), value.clone());
        }
    }

    for name in &config.env.required {
        match context.env.get(name) {
            Some(value) => {
                env.insert(name.clone(), value.clone());
            }
            None => missing_env.push(name.clone()),
        }
    }

    for (name, value) in &config.env.set {
        env.insert(name.clone(), value.clone());
    }

    let mut readonly_files = Vec::new();
    let mut missing_files = Vec::new();

    for raw in &config.files.readonly {
        let path = expand_user_path(raw, &context.home);
        if path.exists() {
            push_unique(&mut readonly_files, path);
        }
    }

    for raw in &config.files.required_readonly {
        let path = expand_user_path(raw, &context.home);
        if path.exists() {
            push_unique(&mut readonly_files, path);
        } else {
            missing_files.push(path);
        }
    }

    if missing_env.is_empty() && missing_files.is_empty() {
        Ok(ResolvedHostInputs {
            env,
            readonly_files,
        })
    } else {
        Err(HostInputError {
            missing_env,
            missing_files,
        })
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}
