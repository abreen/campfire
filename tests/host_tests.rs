use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use campfire::config::CampfireConfig;
use campfire::host::{HostContext, expand_user_path, validate_host_inputs};

#[test]
fn expands_home_relative_paths_with_provided_home() {
    let home = PathBuf::from("/home/alex");

    assert_eq!(expand_user_path("~", &home), PathBuf::from("/home/alex"));
    assert_eq!(
        expand_user_path("~/.aws/config", &home),
        PathBuf::from("/home/alex/.aws/config")
    );
    assert_eq!(
        expand_user_path("/etc/hosts", &home),
        PathBuf::from("/etc/hosts")
    );
}

#[test]
fn resolves_passed_set_and_readonly_inputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home/alex");
    fs::create_dir_all(home.join(".aws")).expect("aws dir");
    fs::write(home.join(".aws/config"), "[profile dev]\n").expect("aws config");

    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[env]
pass = ["AWS_PROFILE", "AWS_REGION", "MISSING_OPTIONAL", "APP_ENV"]
set = { APP_ENV = "dev" }

[files]
readonly = ["~/.aws/config", "~/.aws/missing"]
"#,
    )
    .expect("config parses");
    let context = HostContext::new(
        BTreeMap::from([
            ("AWS_PROFILE".to_string(), "dev".to_string()),
            ("AWS_REGION".to_string(), "us-east-1".to_string()),
            ("APP_ENV".to_string(), "host".to_string()),
        ]),
        home.clone(),
    );

    let resolved = validate_host_inputs(&config, &context, temp.path()).expect("inputs resolve");

    assert_eq!(resolved.env["AWS_PROFILE"], "dev");
    assert_eq!(resolved.env["AWS_REGION"], "us-east-1");
    assert_eq!(resolved.env["APP_ENV"], "dev");
    assert!(!resolved.env.contains_key("MISSING_OPTIONAL"));
    assert_eq!(resolved.readonly_files, vec![home.join(".aws/config")]);
}

#[test]
fn reports_missing_required_env_and_files_together() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home/alex");
    fs::create_dir_all(&home).expect("home dir");

    let config: CampfireConfig = toml::from_str(
        r#"
[campfire]
image = "fedora"

[env]
required = ["AWS_PROFILE", "AWS_REGION"]

[files]
required_readonly = ["~/.aws/credentials"]
"#,
    )
    .expect("config parses");
    let context = HostContext::new(
        BTreeMap::from([("AWS_PROFILE".to_string(), "dev".to_string())]),
        home.clone(),
    );

    let error =
        validate_host_inputs(&config, &context, temp.path()).expect_err("missing inputs fail");

    assert_eq!(error.missing_env, vec!["AWS_REGION"]);
    assert_eq!(error.missing_files, vec![home.join(".aws/credentials")]);
}
