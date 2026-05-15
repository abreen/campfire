use std::fs;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn real_podman_check_runs_with_workspace_env_and_readonly_file() {
    if std::env::var("CAMPFIRE_RUN_PODMAN_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping real Podman integration test; set CAMPFIRE_RUN_PODMAN_TESTS=1");
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    let secret = project.path().join("outer-secret.txt");
    fs::write(project.path().join("workspace.txt"), "workspace-value").expect("workspace file");
    fs::write(&secret, "secret-value").expect("secret file");

    let image = std::env::var("CAMPFIRE_PODMAN_TEST_IMAGE")
        .unwrap_or_else(|_| "docker.io/library/alpine:3.20".to_string());
    let secret = secret.display();
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{image}"

[env]
pass = ["CAMPFIRE_ITEST_ENV"]
required = ["CAMPFIRE_ITEST_ENV"]
set = {{ CAMPFIRE_ITEST_FILE = "{secret}" }}

[files]
required_readonly = ["{secret}"]

[tools.integration]
check = "test \"$CAMPFIRE_ITEST_ENV\" = \"outer-value\" && test \"$(cat /workspace/workspace.txt)\" = \"workspace-value\" && test \"$(cat \"$CAMPFIRE_ITEST_FILE\")\" = \"secret-value\" && echo campfire-podman-ok"
contains = "campfire-podman-ok"
"#
        ),
    )
    .expect("write config");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .env("CAMPFIRE_ITEST_ENV", "outer-value")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Campfire check passed"));
}
