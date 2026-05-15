# Campfire 🏕️

Campfire (`cf`) opens a repo-defined Podman toolbox for a project. The project keeps a
`Campfire.toml` file at its root, and Campfire uses that file to choose the container
image, mount the workspace, pass selected host inputs, and run quick tool checks.

Campfire is a private Rust CLI prototype. It is intentionally not published to
crates.io (`publish = false`).

## What it gives you

- One command to enter the same containerized shell for a repo.
- A read-write project mount inside the container, usually at `/workspace`.
- Optional or required host environment variables and read-only files.
- A `cf check` command that validates Podman, required inputs, and configured tools.

## Requirements

- Podman installed and available on `PATH`.
- Rust/Cargo to build the `cf` binary.
- A container image that contains the tools your project needs.

## Quick start

Build Campfire, create a starter config, then enter the container:

```sh
cargo build
./target/debug/cf init --image docker.io/library/fedora:latest
./target/debug/cf check
./target/debug/cf enter
```

If `cf` is already on your `PATH`, use `cf` directly instead of `./target/debug/cf`.

Run a one-off command in the Campfire environment:

```sh
cf run -- sh -lc 'echo hello > /workspace/new-file.txt'
```

## Commands

| Command | What it does |
| --- | --- |
| `cf init --image IMAGE` | Writes a starter `Campfire.toml` in the current directory. |
| `cf check` | Validates the config, required host inputs, Podman, and configured tool checks. |
| `cf enter` | Starts an interactive shell inside the configured container. |
| `cf run -- COMMAND ...` | Runs a command inside the configured container. |

Campfire looks for `Campfire.toml` in the current directory and then walks upward. The
directory containing `Campfire.toml` is the project root.

## `Campfire.toml`

A compact config can be just an image:

```toml
[campfire]
image = "docker.io/library/fedora:latest"
```

A fuller example:

```toml
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
contains = "aws-cli/2."
```

### Config reference

- `[campfire]`
  - `image` is required.
  - `shell` defaults to `/bin/sh` and is used by `cf enter` and tool checks.
- `[workspace]`
  - `path` defaults to `/workspace`.
  - The project root is mounted read-write at this path.
- `[env]`
  - `pass` copies host variables when they exist.
  - `required` copies host variables and fails early if any are missing.
  - `set` defines fixed values and overrides copied values with the same name.
- `[files]`
  - `readonly` mounts existing files read-only and ignores missing files.
  - `required_readonly` mounts files read-only and fails early if any are missing.
  - `~` expands from the host `HOME`; relative paths resolve from the project root.
- `[tools.NAME]`
  - `check` runs during `cf check` inside the container.
  - `contains` is optional and requires the combined stdout/stderr to contain the text.

Read-only files are mounted at the same absolute path inside the container.

## How the pieces fit together

- `src/app.rs` owns the CLI commands and error flow.
- `src/config.rs` defines `Campfire.toml` and config discovery.
- `src/host.rs` resolves host environment variables and files.
- `src/podman.rs` builds the `podman run` arguments.
- `src/main.rs` is the small binary entry point.
- `tests/` covers CLI behavior, config parsing, host input resolution, Podman argument
  building, and opt-in real Podman integration tests.

## Specs

Design notes live in `spec/`. They describe intended behavior without tying that
behavior to a specific Rust implementation.

## Development

Run the same checks as CI before opening a PR:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

On Fedora Silverblue, Cargo may only be available inside the project toolbox:

```sh
toolbox run --container fedora-toolbox-43 cargo fmt --check
toolbox run --container fedora-toolbox-43 cargo clippy --all-targets -- -D warnings
toolbox run --container fedora-toolbox-43 cargo test --all-targets
```

Run real Podman integration tests only when host Podman is available:

```sh
CAMPFIRE_RUN_PODMAN_TESTS=1 cargo test --test podman_integration_tests -- --nocapture
```

You can override the integration-test image with `CAMPFIRE_PODMAN_TEST_IMAGE`.

## Troubleshooting

- `could not find Campfire.toml`: run `cf` from the project root or a subdirectory.
- `missing required env vars`: export the variables listed in the error.
- `missing required files`: create the listed files or update `required_readonly`.
- `podman is not installed or not on PATH`: install Podman or update `PATH`.
- SELinux bind mounts on Fedora Silverblue are expected to work because Campfire passes
  `--security-opt label=disable` to Podman.
