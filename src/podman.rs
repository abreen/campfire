use std::path::{Path, PathBuf};

use crate::config::{CampfireConfig, ToolCheck};
use crate::host::ResolvedHostInputs;

pub fn build_enter_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, true);
    args.push(config.campfire.image.clone());
    args.push(config.campfire.shell.clone());
    args
}

pub fn build_tool_check_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
    tool: &ToolCheck,
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, false);
    args.push(config.campfire.image.clone());
    args.push("/bin/sh".to_string());
    args.push("-lc".to_string());
    args.push(tool.check.clone());
    args
}

fn base_run_args(
    config: &CampfireConfig,
    project_root: &Path,
    inputs: &ResolvedHostInputs,
    interactive: bool,
) -> Vec<String> {
    let mut args = vec!["run".to_string(), "--rm".to_string()];

    if interactive {
        args.push("-it".to_string());
    }

    args.extend([
        "--workdir".to_string(),
        config.workspace.path.clone(),
        "--volume".to_string(),
        format!(
            "{}:{}:rw",
            path_to_string(project_root),
            config.workspace.path
        ),
    ]);

    for file in &inputs.readonly_files {
        let file = path_to_string(file);
        args.push("--volume".to_string());
        args.push(format!("{file}:{file}:ro"));
    }

    for (name, value) in &inputs.env {
        args.push("--env".to_string());
        args.push(format!("{name}={value}"));
    }

    args
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
