use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn real_podman_check_runs_with_workspace_env_and_readonly_file() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    let secret = project.path().join("outer-secret.txt");
    fs::write(project.path().join("workspace.txt"), "workspace-value").expect("workspace file");
    fs::write(&secret, "secret-value").expect("secret file");

    let image = podman_test_image();
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

#[test]
fn real_podman_run_writes_through_to_host_workspace() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{}"
"#,
            podman_test_image()
        ),
    )
    .expect("write config");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .args([
            "run",
            "--",
            "sh",
            "-lc",
            "printf campfire-write > /workspace/new-file.txt",
        ])
        .assert()
        .success();

    let written = fs::read_to_string(project.path().join("new-file.txt")).expect("read file");
    assert_eq!(written, "campfire-write");
}

#[test]
fn real_podman_run_preserves_stdin() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{}"
"#,
            podman_test_image()
        ),
    )
    .expect("write config");

    let mut command = Command::cargo_bin("cf").expect("cf binary");
    command
        .current_dir(project.path())
        .args(["run", "--", "sh", "-lc", "cat > /workspace/stdin.txt"])
        .write_stdin("campfire-stdin")
        .assert()
        .success();

    let written = fs::read_to_string(project.path().join("stdin.txt")).expect("read file");
    assert_eq!(written, "campfire-stdin");
}

#[test]
fn real_podman_check_reads_project_relative_required_file_from_subdir() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    let nested = project.path().join("services/api");
    let config_dir = project.path().join("config");
    let secret = config_dir.join("secret.txt");
    fs::create_dir_all(&nested).expect("nested dir");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(&secret, "campfire-relative-secret").expect("secret file");

    let image = podman_test_image();
    let secret = secret.canonicalize().expect("canonical secret path");
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{image}"

[env]
set = {{ CAMPFIRE_RELATIVE_SECRET = "{}" }}

[files]
required_readonly = ["config/secret.txt"]

[tools.secret]
check = "test \"$(cat \"$CAMPFIRE_RELATIVE_SECRET\")\" = \"campfire-relative-secret\" && echo relative-secret-ok"
contains = "relative-secret-ok"
"#,
            secret.display()
        ),
    )
    .expect("write config");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(nested)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Campfire check passed"));
}

#[test]
fn real_podman_named_commands_run_with_workspace_and_stdin() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{}"

[commands.write_note]
run = "printf named-write > /workspace/named.txt"

[commands.capture]
run = "cat > /workspace/named-stdin.txt"
"#,
            podman_test_image()
        ),
    )
    .expect("write config");

    std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .args(["run", "write_note"])
        .assert()
        .success();

    let written = fs::read_to_string(project.path().join("named.txt")).expect("read file");
    assert_eq!(written, "named-write");

    let mut command = Command::cargo_bin("cf").expect("cf binary");
    command
        .current_dir(project.path())
        .args(["run", "capture"])
        .write_stdin("named-stdin")
        .assert()
        .success();

    let written = fs::read_to_string(project.path().join("named-stdin.txt")).expect("read file");
    assert_eq!(written, "named-stdin");
}

#[test]
fn real_podman_run_publishes_configured_ports_to_host() {
    if skip_unless_enabled() {
        return;
    }

    let project = tempfile::tempdir().expect("project tempdir");
    let host_port = free_local_port();
    fs::write(
        project.path().join("Campfire.toml"),
        format!(
            r#"
[campfire]
image = "{}"

[[ports]]
container = 8080
host = {host_port}
"#,
            podman_test_image()
        ),
    )
    .expect("write config");

    let mut child = std::process::Command::cargo_bin("cf")
        .expect("cf binary")
        .current_dir(project.path())
        .args([
            "run",
            "--",
            "sh",
            "-lc",
            "while true; do printf 'HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\ncampfire-port-ok' | nc -l -p 8080; done",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cf server");

    let response = wait_for_http_response(&mut child, host_port);
    stop_child(&mut child);

    assert!(
        response.contains("campfire-port-ok"),
        "unexpected response: {response}"
    );
}

fn skip_unless_enabled() -> bool {
    if std::env::var("CAMPFIRE_RUN_PODMAN_TESTS").ok().as_deref() == Some("1") {
        return false;
    }

    eprintln!("skipping real Podman integration test; set CAMPFIRE_RUN_PODMAN_TESTS=1");
    true
}

fn podman_test_image() -> String {
    std::env::var("CAMPFIRE_PODMAN_TEST_IMAGE")
        .unwrap_or_else(|_| "docker.io/library/alpine:3.20".to_string())
}

fn free_local_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind random local port")
        .local_addr()
        .expect("local addr")
        .port()
}

fn wait_for_http_response(child: &mut Child, port: u16) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last_error = String::new();

    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("check child status") {
            panic!("server exited before accepting connections: {status}");
        }

        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .expect("set read timeout");
                stream
                    .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                    .expect("write http request");

                let mut response = String::new();
                if let Err(error) = stream.read_to_string(&mut response)
                    && response.is_empty()
                {
                    last_error = error.to_string();
                }

                if !response.is_empty() {
                    return response;
                }
            }
            Err(error) => {
                last_error = error.to_string();
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    panic!("timed out waiting for published port {port}: {last_error}");
}

fn stop_child(child: &mut Child) {
    if child.try_wait().expect("check child status").is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}
