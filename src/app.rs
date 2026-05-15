use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use crate::config::{CampfireConfig, discover_config};
use crate::host::{HostContext, HostInputError, ResolvedHostInputs, validate_host_inputs};
use crate::podman::{build_enter_args, build_run_args, build_tool_check_args};

#[derive(Debug, Parser)]
#[command(
    name = "cf",
    version,
    about = "Enter repo-defined Campfire toolbox shells"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a starter Campfire.toml in the current directory.
    Init {
        /// Podman image used for the campfire shell.
        #[arg(long)]
        image: String,
    },
    /// Validate config, host inputs, Podman, and configured tool checks.
    Check,
    /// Open an interactive shell inside the campfire environment.
    Enter,
    /// Run a command inside the campfire environment.
    Run {
        /// Command and arguments to run after `--`.
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

pub fn run() -> Result<i32> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { image } => {
            init_config(&image)?;
            Ok(0)
        }
        Commands::Check => {
            let session = load_session()?;
            ensure_podman()?;
            run_tool_checks(&session)?;
            println!("Campfire check passed");
            Ok(0)
        }
        Commands::Enter => {
            let session = load_session()?;
            ensure_podman()?;
            let args = build_enter_args(&session.config, session.project_root, &session.inputs);
            run_podman_status(args)
        }
        Commands::Run { command } => {
            let session = load_session()?;
            ensure_podman()?;
            let args = build_run_args(
                &session.config,
                session.project_root,
                &session.inputs,
                &command,
            );
            run_podman_status(args)
        }
    }
}

struct Session {
    config: CampfireConfig,
    project_root: PathBuf,
    inputs: ResolvedHostInputs,
}

fn init_config(image: &str) -> Result<()> {
    if image.contains('"') || image.contains('\n') {
        bail!("image must not contain quotes or newlines");
    }

    let path = PathBuf::from("Campfire.toml");
    if path.exists() {
        bail!("Campfire.toml already exists");
    }

    fs::write(&path, starter_config(image)).context("failed to write Campfire.toml")?;
    println!("Created Campfire.toml");
    Ok(())
}

fn starter_config(image: &str) -> String {
    format!(
        r#"[campfire]
image = "{image}"
shell = "/bin/sh"

[workspace]
path = "/workspace"

[env]
pass = []
required = []
set = {{}}

[files]
readonly = []
required_readonly = []
"#
    )
}

fn load_session() -> Result<Session> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let config_path = discover_config(&cwd)?;
    let project_root = config_path
        .parent()
        .context("Campfire.toml has no parent directory")?
        .to_path_buf();
    let config = load_config(&config_path)?;
    let context = HostContext::current();
    let inputs =
        validate_host_inputs(&config, &context, &project_root).map_err(format_host_input_error)?;

    Ok(Session {
        config,
        project_root,
        inputs,
    })
}

fn load_config(path: &Path) -> Result<CampfireConfig> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&source).with_context(|| format!("failed to parse {}", path.display()))
}

fn ensure_podman() -> Result<()> {
    let output = Command::new("podman")
        .arg("--version")
        .output()
        .context("podman is not installed or not on PATH")?;

    if !output.status.success() {
        bail!(
            "podman --version failed: {}",
            combined_output(&output.stdout, &output.stderr)
        );
    }

    Ok(())
}

fn run_tool_checks(session: &Session) -> Result<()> {
    for (name, tool) in &session.config.tools {
        let args = build_tool_check_args(
            &session.config,
            session.project_root.clone(),
            &session.inputs,
            tool,
        );
        let output = Command::new("podman")
            .args(args)
            .output()
            .with_context(|| format!("failed to run tool check `{name}`"))?;
        let combined = combined_output(&output.stdout, &output.stderr);

        if !output.status.success() {
            bail!("tool check `{name}` failed: {combined}");
        }

        if let Some(expected) = &tool.contains
            && !combined.contains(expected)
        {
            bail!("tool check `{name}` did not contain `{expected}` in output: {combined}");
        }
    }

    Ok(())
}

fn run_podman_status(args: Vec<String>) -> Result<i32> {
    let status = Command::new("podman")
        .args(args)
        .status()
        .context("failed to run podman")?;
    Ok(status.code().unwrap_or(1))
}

fn format_host_input_error(error: HostInputError) -> anyhow::Error {
    let mut lines = Vec::new();

    if !error.missing_env.is_empty() {
        lines.push(format!(
            "missing required env vars: {}",
            error.missing_env.join(", ")
        ));
    }

    if !error.missing_files.is_empty() {
        let files = error
            .missing_files
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("missing required files: {files}"));
    }

    anyhow::anyhow!(lines.join("\n"))
}

fn combined_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(stdout));
    combined.push_str(&String::from_utf8_lossy(stderr));
    combined.trim().to_string()
}
