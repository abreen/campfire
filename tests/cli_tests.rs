use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn init_writes_starter_config() {
    let project = tempfile::tempdir().expect("project tempdir");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .args(["init", "--image", "fedora:latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created Campfire.toml"));

    let config = fs::read_to_string(project.path().join("Campfire.toml")).expect("read config");
    assert!(config.contains("image = \"fedora:latest\""));
    assert!(config.contains("path = \"/workspace\""));
    assert!(config.contains("pass = []"));
}

#[test]
fn init_refuses_to_overwrite_existing_config() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        "[campfire]\nimage = \"fedora\"\n",
    )
    .expect("write config");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .args(["init", "--image", "alpine"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Campfire.toml already exists"));
}

#[test]
fn check_uses_fake_podman_and_validates_tool_output() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[env]
pass = ["AWS_PROFILE"]
required = ["AWS_PROFILE"]

[tools.aws]
check = "aws --version"
contains = "aws-cli/2.15."
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "aws-cli/2.15.99 Python/3.11");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("AWS_PROFILE", "dev")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Campfire check passed"));

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("--version"));
    assert!(calls.contains("aws --version"));
}

#[test]
fn check_fails_when_tool_output_does_not_contain_expected_text() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[tools.aws]
check = "aws --version"
contains = "aws-cli/2.15."
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "aws-cli/1.32.0 Python/3.11");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "tool check `aws` did not contain `aws-cli/2.15.`",
        ))
        .stderr(predicate::str::contains("aws-cli/1.32.0"));
}

#[test]
fn check_reports_missing_required_inputs_before_running_podman() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[env]
required = ["AWS_PROFILE"]
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "unused");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env_remove("AWS_PROFILE")
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "missing required env vars: AWS_PROFILE",
        ));

    assert!(
        !log.exists(),
        "podman should not run when host inputs are missing"
    );
}

#[test]
fn check_ignores_unrelated_non_utf8_env_vars() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[tools.aws]
check = "aws --version"
contains = "aws-cli/2.15."
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "aws-cli/2.15.99 Python/3.11");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env(
            OsString::from_vec(b"CAMPFIRE_\xFF_UNRELATED".to_vec()),
            OsString::from_vec(b"ignored-\xFF-value".to_vec()),
        )
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Campfire check passed"));
}

#[test]
fn enter_executes_podman_with_workspace_mount() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"
shell = "/bin/bash"

[workspace]
path = "/workspace"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "entered");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("enter")
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("run --rm -it --security-opt label=disable --workdir /workspace"));
    assert!(calls.contains(&format!("{}:/workspace:rw", project.path().display())));
    assert!(calls.contains("fedora /bin/bash"));
}

#[test]
fn enter_publishes_configured_ports() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "entered");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("enter")
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("--publish 127.0.0.1:8080:8080"));
}

#[test]
fn enter_refreshes_passed_env_between_invocations() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[env]
pass = ["AWS_PROFILE"]
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "entered");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("AWS_PROFILE", "first-profile")
        .arg("enter")
        .assert()
        .success();

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("AWS_PROFILE", "second-profile")
        .arg("enter")
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("--env AWS_PROFILE=first-profile"));
    assert!(calls.contains("--env AWS_PROFILE=second-profile"));
}

#[test]
fn enter_refreshes_required_files_between_invocations() {
    let project = tempfile::tempdir().expect("project tempdir");
    let home = project.path().join("home/alex");
    let aws_dir = home.join(".aws");
    let credentials = aws_dir.join("credentials");
    fs::create_dir_all(&aws_dir).expect("aws dir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[files]
required_readonly = ["~/.aws/credentials"]
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "entered");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("HOME", &home)
        .arg("enter")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required files"));

    fs::write(&credentials, "[default]\naws_access_key_id = test\n").expect("credentials");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("HOME", &home)
        .arg("enter")
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains(&format!(
        "{}:{}:ro",
        credentials.display(),
        credentials.display()
    )));
}

#[test]
fn enter_resolves_relative_readonly_files_from_project_root_when_run_in_subdir() {
    let project = tempfile::tempdir().expect("project tempdir");
    let nested = project.path().join("services/api");
    let config_dir = project.path().join("config");
    let settings = config_dir.join("settings.toml");
    fs::create_dir_all(&nested).expect("nested dirs");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(&settings, "api_key = \"test\"\n").expect("settings");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[files]
required_readonly = ["config/settings.toml"]
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "entered");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(&nested)
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("enter")
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    let settings = settings.canonicalize().expect("canonical settings path");
    assert!(calls.contains(&format!("{}:{}:ro", settings.display(), settings.display())));
}

#[test]
fn run_executes_podman_with_user_command() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "ran");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .args(["run", "--", "sh", "-lc", "echo hi"])
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("run --rm -i --security-opt label=disable --workdir /workspace"));
    assert!(calls.contains(&format!("{}:/workspace:rw", project.path().display())));
    assert!(calls.contains("fedora sh -lc echo hi"));
}

#[test]
fn run_publishes_configured_ports() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080
host = 18080
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "ran");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .args(["run", "--", "sh", "-lc", "echo hi"])
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("--publish 127.0.0.1:18080:8080"));
}

#[test]
fn run_executes_configured_command_by_name() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"
shell = "/bin/bash"

[commands.gs]
run = "git status"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "ran");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .args(["run", "gs", "-sb"])
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("fedora /bin/bash -lc git status '-sb'"));
}

#[test]
fn run_delimiter_forces_raw_command_when_name_conflicts() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[commands.sh]
run = "echo configured"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "ran");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .args(["run", "--", "sh", "-lc", "echo raw"])
        .assert()
        .success();

    let calls = fs::read_to_string(log).expect("podman log");
    assert!(calls.contains("fedora sh -lc echo raw"));
    assert!(!calls.contains("echo configured"));
}

#[test]
fn check_rejects_invalid_command_names_before_running_podman() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[commands.bad-name]
run = "git status"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "unused");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid command name `bad-name`"));

    assert!(
        !log.exists(),
        "podman should not run when config validation fails"
    );
}

#[test]
fn check_rejects_invalid_ports_before_running_podman() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080
bind = "localhost"
"#,
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "unused");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid bind address `localhost`"));

    assert!(
        !log.exists(),
        "podman should not run when config validation fails"
    );
}

#[test]
fn run_propagates_podman_exit_code() {
    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        "[campfire]\nimage = \"fedora\"\n",
    )
    .expect("write config");
    let fake_bin = tempfile::tempdir().expect("fake bin");
    let log = project.path().join("podman.log");
    write_fake_podman(fake_bin.path(), &log, "failed");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("PATH", fake_path(fake_bin.path()))
        .env("PODMAN_LOG", &log)
        .env("PODMAN_EXIT_CODE", "7")
        .args(["run", "--", "false"])
        .assert()
        .code(7);
}

fn write_fake_podman(dir: &Path, log: &Path, tool_output: &str) {
    fs::create_dir_all(dir).expect("fake bin dir");
    let script = format!(
        r#"#!/bin/sh
printf '%s\n' "$*" >> "${{PODMAN_LOG:-{log}}}"
if [ "$1" = "--version" ]; then
  echo "podman version 5.0.0"
  exit 0
fi
echo "{tool_output}"
exit "${{PODMAN_EXIT_CODE:-0}}"
"#,
        log = log.display(),
        tool_output = tool_output
    );
    let path = dir.join("podman");
    fs::write(&path, script).expect("write fake podman");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("chmod fake podman");
}

fn fake_path(fake_bin: &Path) -> String {
    let old_path = std::env::var("PATH").unwrap_or_default();
    format!("{}:{old_path}", fake_bin.display())
}
