use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use crate::commands::render_shell_functions;
use crate::config::{CampfireConfig, discover_config, validate_config};
use crate::host::{HostContext, HostInputError, ResolvedHostInputs, validate_host_inputs};
use crate::podman::{
    EnterShellSetup, build_enter_args, build_enter_args_with_setup, build_named_run_args,
    build_run_args, build_tool_check_args,
};

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
    let raw_args = env::args_os().collect::<Vec<_>>();
    let explicit_raw_run = run_uses_explicit_raw_command(&raw_args);
    let cli = Cli::parse_from(&raw_args);

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
            let setup = create_shell_setup(&session.config)?;
            let args = match &setup {
                Some(setup) => build_enter_args_with_setup(
                    &session.config,
                    session.project_root,
                    &session.inputs,
                    &setup.enter_shell_setup(),
                ),
                None => build_enter_args(&session.config, session.project_root, &session.inputs),
            };
            run_podman_status(args)
        }
        Commands::Run { command } => {
            let session = load_session()?;
            ensure_podman()?;
            let args = build_run_invocation_args(&session, &command, explicit_raw_run);
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
    validate_config(&config)?;
    let context = HostContext::current();
    let inputs =
        validate_host_inputs(&config, &context, &project_root).map_err(format_host_input_error)?;

    Ok(Session {
        config,
        project_root,
        inputs,
    })
}

fn build_run_invocation_args(
    session: &Session,
    command: &[String],
    explicit_raw_run: bool,
) -> Vec<String> {
    if !explicit_raw_run
        && let Some((name, extra_args)) = command.split_first()
        && let Some(snippet) = session.config.commands.get(name)
    {
        return build_named_run_args(
            &session.config,
            session.project_root.clone(),
            &session.inputs,
            snippet,
            extra_args,
        );
    }

    build_run_args(
        &session.config,
        session.project_root.clone(),
        &session.inputs,
        command,
    )
}

fn run_uses_explicit_raw_command(args: &[OsString]) -> bool {
    let run_index = args
        .iter()
        .skip(1)
        .position(|arg| arg == OsStr::new("run"))
        .map(|index| index + 1);

    run_index
        .and_then(|index| args.get(index + 1))
        .is_some_and(|arg| arg == OsStr::new("--"))
}

struct ShellSetupFile {
    host_path: PathBuf,
    container_path: String,
}

impl ShellSetupFile {
    fn enter_shell_setup(&self) -> EnterShellSetup {
        EnterShellSetup {
            host_path: self.host_path.clone(),
            container_path: self.container_path.clone(),
        }
    }
}

impl Drop for ShellSetupFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.host_path);
    }
}

fn create_shell_setup(config: &CampfireConfig) -> Result<Option<ShellSetupFile>> {
    if config.commands.is_empty() {
        return Ok(None);
    }

    let cache_dir = campfire_cache_dir();
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create {}", cache_dir.display()))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos();
    let host_path = cache_dir.join(format!("commands-{}-{timestamp}.sh", std::process::id()));
    fs::write(&host_path, render_shell_functions(&config.commands))
        .with_context(|| format!("failed to write {}", host_path.display()))?;

    Ok(Some(ShellSetupFile {
        host_path,
        container_path: "/tmp/campfire-commands.sh".to_string(),
    }))
}

fn campfire_cache_dir() -> PathBuf {
    if let Some(cache_home) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("campfire");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".cache/campfire");
    }

    env::temp_dir().join("campfire")
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
