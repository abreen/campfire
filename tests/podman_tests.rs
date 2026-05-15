use std::collections::BTreeMap;
use std::path::PathBuf;

use campfire::config::CampfireConfig;
use campfire::host::ResolvedHostInputs;
use campfire::podman::{build_enter_args, build_tool_check_args};

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
            "--workdir",
            "/workspace",
            "--volume",
            "/repo:/workspace:rw",
            "--env",
            "AWS_PROFILE=dev",
            "fedora",
            "/bin/sh",
            "-lc",
            "aws --version",
        ]
    );
}
