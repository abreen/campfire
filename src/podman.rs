use std::path::{Path, PathBuf};

use crate::commands::build_snippet_shell_command;
use crate::config::{CampfireConfig, CommandSnippet, ToolCheck};
use crate::host::ResolvedHostInputs;

enum RunMode {
    InteractiveTty,
    Stdin,
    NonInteractive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterShellSetup {
    pub host_path: PathBuf,
    pub container_path: String,
}

pub fn build_enter_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, RunMode::InteractiveTty);
    args.push(config.campfire.image.clone());
    args.push(config.campfire.shell.clone());
    args
}

pub fn build_enter_args_with_setup(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
    setup: &EnterShellSetup,
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, RunMode::InteractiveTty);
    args.push("--volume".to_string());
    args.push(format!(
        "{}:{}:ro",
        path_to_string(&setup.host_path),
        setup.container_path
    ));

    if !is_bash_shell(&config.campfire.shell) {
        args.push("--env".to_string());
        args.push(format!("ENV={}", setup.container_path));
    }

    args.push(config.campfire.image.clone());
    args.push(config.campfire.shell.clone());

    if is_bash_shell(&config.campfire.shell) {
        args.push("--rcfile".to_string());
        args.push(setup.container_path.clone());
        args.push("-i".to_string());
    } else {
        args.push("-i".to_string());
    }

    args
}

pub fn build_tool_check_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
    tool: &ToolCheck,
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, RunMode::NonInteractive);
    args.push(config.campfire.image.clone());
    args.push(config.campfire.shell.clone());
    args.push("-lc".to_string());
    args.push(tool.check.clone());
    args
}

pub fn build_named_run_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
    command: &CommandSnippet,
    extra_args: &[String],
) -> Vec<String> {
    let shell_command = build_snippet_shell_command(command, extra_args);
    build_run_args(
        config,
        project_root,
        inputs,
        &[
            config.campfire.shell.clone(),
            "-lc".to_string(),
            shell_command,
        ],
    )
}

pub fn build_run_args(
    config: &CampfireConfig,
    project_root: PathBuf,
    inputs: &ResolvedHostInputs,
    command: &[String],
) -> Vec<String> {
    let mut args = base_run_args(config, &project_root, inputs, RunMode::Stdin);
    args.push(config.campfire.image.clone());
    args.extend(command.iter().cloned());
    args
}

fn base_run_args(
    config: &CampfireConfig,
    project_root: &Path,
    inputs: &ResolvedHostInputs,
    mode: RunMode,
) -> Vec<String> {
    let mut args = vec!["run".to_string(), "--rm".to_string()];

    match mode {
        RunMode::InteractiveTty => args.push("-it".to_string()),
        RunMode::Stdin => args.push("-i".to_string()),
        RunMode::NonInteractive => {}
    }

    args.extend(["--security-opt".to_string(), "label=disable".to_string()]);

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

fn is_bash_shell(shell: &str) -> bool {
    Path::new(shell).file_name().and_then(|name| name.to_str()) == Some("bash")
}
