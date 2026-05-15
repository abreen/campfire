use std::collections::BTreeMap;
use std::path::PathBuf;

use campfire::config::CampfireConfig;
use campfire::host::ResolvedHostInputs;
use campfire::podman::{
    EnterShellSetup, build_enter_args, build_enter_args_with_setup, build_named_run_args,
    build_run_args, build_tool_check_args,
};

#[test]
fn builds_interactive_enter_arguments() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "ghcr.io/acme/service-tools:2026.05"
shell = "/bin/bash"

[workspace]
path = "/workspace"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::from([
            ("APP_ENV".to_string(), "dev".to_string()),
            ("AWS_PROFILE".to_string(), "dev-profile".to_string()),
        ]),
        readonly_files: vec![PathBuf::from("/home/alex/.aws/config")],
    };

    let args = build_enter_args(&config, PathBuf::from("/repo"), &inputs);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-it",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--volume",
            "/home/alex/.aws/config:/home/alex/.aws/config:ro",
            "--env",
            "APP_ENV=dev",
            "--env",
            "AWS_PROFILE=dev-profile",
            "ghcr.io/acme/service-tools:2026.05",
            "/bin/bash",
        ]
    );
}

#[test]
fn builds_non_interactive_tool_check_arguments() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"
shell = "/bin/bash"

[tools.aws]
check = "aws --version"
contains = "aws-cli/2.15."
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::from([("AWS_PROFILE".to_string(), "dev".to_string())]),
        readonly_files: vec![],
    };

    let args = build_tool_check_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &config.tools["aws"],
    );

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--env",
            "AWS_PROFILE=dev",
            "fedora",
            "/bin/bash",
            "-lc",
            "aws --version",
        ]
    );
}

#[test]
fn builds_stdin_open_run_arguments_without_tty() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::from([("AWS_PROFILE".to_string(), "dev".to_string())]),
        readonly_files: vec![PathBuf::from("/home/alex/.aws/config")],
    };

    let args = build_run_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &["sh".to_string(), "-lc".to_string(), "echo hi".to_string()],
    );

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-i",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--volume",
            "/home/alex/.aws/config:/home/alex/.aws/config:ro",
            "--env",
            "AWS_PROFILE=dev",
            "fedora",
            "sh",
            "-lc",
            "echo hi",
        ]
    );
}

#[test]
fn builds_enter_arguments_with_configured_ports() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080

[[ports]]
container = 3000
host = 13000
bind = "0.0.0.0"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };

    let args = build_enter_args(&config, PathBuf::from("/repo"), &inputs);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-it",
            "--security-opt",
            "label=disable",
            "--publish",
            "127.0.0.1:8080:8080",
            "--publish",
            "0.0.0.0:13000:3000",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "fedora",
            "/bin/sh",
        ]
    );
}

#[test]
fn builds_run_arguments_with_configured_ports() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };

    let args = build_run_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &["sh".to_string(), "-lc".to_string(), "echo hi".to_string()],
    );

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-i",
            "--security-opt",
            "label=disable",
            "--publish",
            "127.0.0.1:8080:8080",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "fedora",
            "sh",
            "-lc",
            "echo hi",
        ]
    );
}

#[test]
fn omits_configured_ports_from_tool_check_arguments() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080

[tools.server]
check = "true"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };

    let args = build_tool_check_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &config.tools["server"],
    );

    assert!(!args.iter().any(|arg| arg == "--publish"));
    assert!(!args.iter().any(|arg| arg == "127.0.0.1:8080:8080"));
}

#[test]
fn builds_named_run_arguments_through_configured_shell() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"
shell = "/bin/bash"

[commands.gs]
run = "git status"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };

    let args = build_named_run_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &config.commands["gs"],
        &["-sb".to_string(), "it's".to_string()],
    );

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-i",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "fedora",
            "/bin/bash",
            "-lc",
            "git status '-sb' 'it'\\''s'",
        ]
    );
}

#[test]
fn builds_named_run_arguments_with_configured_ports() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[[ports]]
container = 8080

[commands.serve]
run = "busybox httpd -f -p 0.0.0.0:8080"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };

    let args = build_named_run_args(
        &config,
        PathBuf::from("/repo"),
        &inputs,
        &config.commands["serve"],
        &[],
    );

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-i",
            "--security-opt",
            "label=disable",
            "--publish",
            "127.0.0.1:8080:8080",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "fedora",
            "/bin/sh",
            "-lc",
            "busybox httpd -f -p 0.0.0.0:8080",
        ]
    );
}

#[test]
fn builds_enter_arguments_with_posix_command_setup() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"
shell = "/bin/sh"

[commands.versions]
run = "cat /etc/alpine-release"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };
    let setup = EnterShellSetup {
        host_path: PathBuf::from("/home/alex/.cache/campfire/commands.sh"),
        container_path: "/tmp/campfire-commands.sh".to_string(),
    };

    let args = build_enter_args_with_setup(&config, PathBuf::from("/repo"), &inputs, &setup);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-it",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--volume",
            "/home/alex/.cache/campfire/commands.sh:/tmp/campfire-commands.sh:ro",
            "--env",
            "ENV=/tmp/campfire-commands.sh",
            "fedora",
            "/bin/sh",
            "-i",
        ]
    );
}

#[test]
fn builds_enter_arguments_with_bash_command_setup() {
    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"
shell = "/bin/bash"

[commands.versions]
run = "cat /etc/alpine-release"
"#,
    )
    .expect("config parses");
    let inputs = ResolvedHostInputs {
        env: BTreeMap::new(),
        readonly_files: vec![],
    };
    let setup = EnterShellSetup {
        host_path: PathBuf::from("/home/alex/.cache/campfire/commands.sh"),
        container_path: "/tmp/campfire-commands.sh".to_string(),
    };

    let args = build_enter_args_with_setup(&config, PathBuf::from("/repo"), &inputs, &setup);

    assert_eq!(
        args,
        vec![
            "run",
            "--rm",
            "-it",
            "--security-opt",
            "label=disable",
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--volume",
            "/home/alex/.cache/campfire/commands.sh:/tmp/campfire-commands.sh:ro",
            "fedora",
            "/bin/bash",
            "--rcfile",
            "/tmp/campfire-commands.sh",
            "-i",
        ]
    );
}
