use std::fs;

use campfire::config::{CampfireConfig, discover_config};

#[test]
fn parses_compact_project_config() {
    let source = r#"
[campfire]
image = "ghcr.io/acme/service-tools:2026.05"
shell = "/bin/bash"

[workspace]
path = "/workspace"

[env]
pass = ["AWS_PROFILE", "AWS_REGION"]
required = ["AWS_PROFILE"]
set = { APP_ENV = "dev" }

[files]
readonly = ["~/.aws/config"]
required_readonly = ["~/.aws/credentials"]

[tools.aws]
check = "aws --version"
contains = "aws-cli/2.15."
"#;

    let config: CampfireConfig = toml::from_str(source).expect("config parses");

    assert_eq!(config.campfire.image, "ghcr.io/acme/service-tools:2026.05");
    assert_eq!(config.campfire.shell, "/bin/bash");
    assert_eq!(config.workspace.path, "/workspace");
    assert_eq!(config.env.pass, vec!["AWS_PROFILE", "AWS_REGION"]);
    assert_eq!(config.env.required, vec!["AWS_PROFILE"]);
    assert_eq!(config.env.set.get("APP_ENV").unwrap(), "dev");
    assert_eq!(config.files.readonly, vec!["~/.aws/config"]);
    assert_eq!(config.files.required_readonly, vec!["~/.aws/credentials"]);
    assert_eq!(config.tools["aws"].check, "aws --version");
    assert_eq!(
        config.tools["aws"].contains.as_deref(),
        Some("aws-cli/2.15.")
    );
}

#[test]
fn uses_defaults_for_optional_sections() {
    let source = r#"
[campfire]
image = "registry.fedoraproject.org/fedora-toolbox:latest"
"#;

    let config: CampfireConfig = toml::from_str(source).expect("config parses");

    assert_eq!(config.campfire.shell, "/bin/sh");
    assert_eq!(config.workspace.path, "/workspace");
    assert!(config.env.pass.is_empty());
    assert!(config.env.required.is_empty());
    assert!(config.env.set.is_empty());
    assert!(config.files.readonly.is_empty());
    assert!(config.files.required_readonly.is_empty());
    assert!(config.tools.is_empty());
}

#[test]
fn discovers_campfire_toml_by_walking_upward() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let nested = root.join("services/api");
    fs::create_dir_all(&nested).expect("nested dirs");
    fs::write(
        root.join("Campfire.toml"),
        "[campfire]\nimage = \"fedora\"\n",
    )
    .expect("write config");

    let discovered = discover_config(&nested).expect("config discovered");

    assert_eq!(discovered, root.join("Campfire.toml"));
}
